mod display;

use std::{collections::HashMap, sync::{mpsc, mpsc::Sender}, time::Duration};

use display::{clear_display, draw_rect, draw_text, flush_display, AppDisplay, DISPLAY_ADDRESS, DISPLAY_I2C_FREQ};
use esp_idf_hal::{gpio::{OutputPin, PinDriver}, i2c::{APBTickType, I2cDriver}, rmt::Receive};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, 
    hal::{delay::FreeRtos, prelude::{Peripherals, FromValueType}}, 
    nvs::EspDefaultNvsPartition, 
    sys::{esp_wifi_set_mode, esp_wifi_set_promiscuous_filter, esp_wifi_set_promiscuous_rx_cb, wifi_mode_t_WIFI_MODE_NULL, wifi_promiscuous_filter_t, WIFI_PROMIS_FILTER_MASK_ALL},
    wifi::{self, ClientConfiguration, WifiDriver}
};
use log::{debug, info};
use ssd1306::{mode::DisplayConfig, prelude::DisplayRotation, size::DisplaySize128x64, I2CDisplayInterface, Ssd1306};

const DURRATION_U64: u64 = 30;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Hello, world!");

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    info!("Setting up I2C for display using GPIO17(SDA) and GPIO18(SCL)");

    let i2c_config = esp_idf_hal::i2c::I2cConfig::new()
        .baudrate(DISPLAY_I2C_FREQ.Hz().into())
        .sda_enable_pullup(true)
        .scl_enable_pullup(true)
        .timeout(APBTickType::from(Duration::from_millis(100)));
    let mut i2c = esp_idf_hal::i2c::I2cDriver::new(
        peripherals.i2c0, 
        peripherals.pins.gpio17,  // SDA_OLED
        peripherals.pins.gpio18,  // SCL_OLED
        &i2c_config
    )?;

    info!("Configuring OLED reset pin on GPIO21");
    let mut reset_pin = PinDriver::output(peripherals.pins.gpio21)?;

    // Initialize display
    info!("Initializing display");
    info!("Performing display reset sequence");
    // Reset sequence (matching typical OLED reset procedure)
    reset_pin.set_low()?;
    FreeRtos::delay_ms(10);
    reset_pin.set_high()?;
    FreeRtos::delay_ms(1000);
    
    let interface = I2CDisplayInterface::new_custom_address(i2c, DISPLAY_ADDRESS);
    
    // Display driver has to be created in main
    // The Bus Lock gets lost at the end of the scope of creation
    let mut display = Ssd1306::new(
        interface,
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    ).into_buffered_graphics_mode();

    // Initialize with better error handling
    info!("Initializing display");
    display.init().map_err(|e| anyhow::anyhow!("Failed to init display: {:?}", e))?;
    
    FreeRtos::delay_ms(1000);

    info!("clearing");
    clear_display(&mut display)?;
    flush_display(&mut display)?;
    FreeRtos::delay_ms(1000);

    // Draw demo content
    draw_text(&mut display, 10, 10, "Hello from Rust!", true)?;
    draw_text(&mut display, 10, 25, "Detecting WiFi...", true)?;
    draw_rect(&mut display, 0, 0, 128, 64, true)?;
    
    info!("Flushing test text");
    flush_display(&mut display)?;
    FreeRtos::delay_ms(1000);

    let basic_client_config = ClientConfiguration {
        ssid: "".try_into().unwrap(),
        password: "".try_into().unwrap(),
        channel: Some(1),  // Set to a specific channel
        auth_method: wifi::AuthMethod::None,
        ..Default::default()
    };
    let wifi_config = wifi::Configuration::Client(basic_client_config);

    let mut wifi_driver = WifiDriver::new(peripherals.modem, sys_loop.clone(), Some(nvs))?;

    let mut mac_map: HashMap<String, bool> = HashMap::new();
    let (tx_mac_map, rx_mac_map): (mpsc::Sender<String>, mpsc::Receiver<String>) = mpsc::channel();
    static mut TX: Option<Sender<String>> = None;

    unsafe {
        TX = Some(tx_mac_map.clone());
    }

    unsafe extern "C" fn rx_callback(buf: *mut core::ffi::c_void, _type: u32) {
        if !buf.is_null() {
            let frame_data = std::slice::from_raw_parts(buf as *const u8, 24);
            let frame_type = (frame_data[0] & 0x0C) >> 2;
            let frame_subtype = (frame_data[0] & 0xF0) >> 4;
            
            let type_str = match frame_type {
                0 => "Management",
                1 => "Control",
                2 => "Data",
                3 => "Extension",
                _ => "Unknown"
            };
    
            // Extract MAC addresses
            let destination_mac = &frame_data[4..10];
            let source_mac = &frame_data[10..16];
            
            // Create MAC address strings with prefixes
            let dst_mac_str = format!("dst_{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                destination_mac[0], destination_mac[1], destination_mac[2],
                destination_mac[3], destination_mac[4], destination_mac[5]);
            
            let src_mac_str = format!("src_{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                source_mac[0], source_mac[1], source_mac[2],
                source_mac[3], source_mac[4], source_mac[5]);
            
            if let Some(tx) = &TX {
                let _ = tx.send(dst_mac_str);
                let _ = tx.send(src_mac_str);
            }
            
            debug!("Frame: {} (type: {}, subtype: {})", type_str, frame_type, frame_subtype);
            debug!("  Destination: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", 
                destination_mac[0], destination_mac[1], destination_mac[2],
                destination_mac[3], destination_mac[4], destination_mac[5]);
            debug!("  Source: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                source_mac[0], source_mac[1], source_mac[2],
                source_mac[3], source_mac[4], source_mac[5]);
        }
    }

    wifi_driver.set_configuration(&wifi_config)?;
    unsafe { 
        esp_wifi_set_mode(wifi_mode_t_WIFI_MODE_NULL);
    };
    wifi_driver.start()?;
    wifi_driver.set_promiscuous(true)?;
    unsafe {
        esp_wifi_set_promiscuous_rx_cb(Some(rx_callback));
        let filter = wifi_promiscuous_filter_t {
            filter_mask: WIFI_PROMIS_FILTER_MASK_ALL,
        };
        esp_wifi_set_promiscuous_filter(&filter);
    }

    info!("Wifi Started");

    // Run for 30 seconds then exit
    let start = std::time::Instant::now();
    let duration = std::time::Duration::from_secs(DURRATION_U64);
    let mut last_check_in_time = start;

    while start.elapsed() < duration {
        // Check for new MAC addresses
        match rx_mac_map.try_recv() {
            Ok(mac) => {
                if !mac_map.contains_key(&mac) {
                    mac_map.insert(mac, true);
                }
            },
            Err(mpsc::TryRecvError::Empty) => {},
            Err(mpsc::TryRecvError::Disconnected) => {
                info!("Channel disconnected");
                break;
            }
        }

        if last_check_in_time.elapsed() >= std::time::Duration::from_secs(1) {
            info!("Time remaining: {} seconds, Unique MACs: {}", 
                DURRATION_U64 - start.elapsed().as_secs(),
                mac_map.len()
            );
            // Update display with current status
            clear_display(&mut display)?;
            draw_text(&mut display, 10, 10, &format!("Time left: {}s", DURRATION_U64 - start.elapsed().as_secs()), true)?;
            draw_text(&mut display, 10, 30, &format!("MACs found: {}", mac_map.len()), true)?;
            flush_display(&mut display)?;
            last_check_in_time = std::time::Instant::now();
        }
        FreeRtos::delay_ms(100);
    }

    info!("Found {} unique MAC addresses", mac_map.len());

    // Show final count on display
    clear_display(&mut display)?;
    draw_text(&mut display, 10, 10, &format!("Found {} MACs", mac_map.len()), true)?;
    flush_display(&mut display)?;
    FreeRtos::delay_ms(5000); // Show the result for 5 seconds

    // Cleanup before exit
    wifi_driver.set_promiscuous(false)?;
    wifi_driver.stop()?;
    info!("{} seconds elapsed, exiting...", DURRATION_U64);

    Ok(())
}
