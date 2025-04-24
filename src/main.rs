mod display;
mod wifi;
mod button;
mod app;
mod spiffs;

use std::{collections::HashMap, fs, path::Path, sync::mpsc::{self, SyncSender}, time::Duration};

use app::{render_initial_menu, update_initial_menu_state, InitMenuDisplayOptions, INIT_MENU_DISPLAY_STATE};
use button::{check_button_event, ButtonEvent};
use display::{clear_display, draw_final_count, draw_rect, draw_start_up, draw_status_update, draw_text, flush_display, DISPLAY_ADDRESS, DISPLAY_I2C_FREQ};
use esp_idf_hal::{gpio::PinDriver, i2c::APBTickType, sys::{esp_deep_sleep_start, esp_wifi_set_promiscuous_rx_cb}};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, 
    hal::{delay::FreeRtos, prelude::{Peripherals, FromValueType}}, 
    nvs::EspDefaultNvsPartition, 
};
use log::{debug, info, error};
use spiffs::get_space_info;
use ssd1306::{mode::DisplayConfig, prelude::DisplayRotation, size::DisplaySize128x64, I2CDisplayInterface, Ssd1306};
use wifi::create_wifi_driver;

const DURRATION_U64: u64 = 30;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    debug!("Hello, world!");

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    debug!("Setting up button");
    let button = button::init_button(peripherals.pins.gpio0)?;

    debug!("Setting up I2C for display using GPIO17(SDA) and GPIO18(SCL)");

    let i2c_config = esp_idf_hal::i2c::I2cConfig::new()
        .baudrate(DISPLAY_I2C_FREQ.Hz())
        .sda_enable_pullup(true)
        .scl_enable_pullup(true)
        .timeout(APBTickType::from(Duration::from_millis(100)));
    let i2c = esp_idf_hal::i2c::I2cDriver::new(
        peripherals.i2c0, 
        peripherals.pins.gpio17,  // SDA_OLED
        peripherals.pins.gpio18,  // SCL_OLED
        &i2c_config
    )?;

    debug!("Configuring OLED reset pin on GPIO21");
    let mut reset_pin = PinDriver::output(peripherals.pins.gpio21)?;

    // Initialize display
    // Display driver has to be created in main
    // The Bus Lock gets lost (released?) at the end of the scope even on returned
    debug!("Initializing display");
    // Reset sequence (matching typical OLED reset procedure)
    reset_pin.set_low()?;
    FreeRtos::delay_ms(10);
    reset_pin.set_high()?;
    FreeRtos::delay_ms(1000);
    let interface = I2CDisplayInterface::new_custom_address(i2c, DISPLAY_ADDRESS);
    let mut display = Ssd1306::new(
        interface,
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    ).into_buffered_graphics_mode();
    display.init().map_err(|e| anyhow::anyhow!("Failed to init display: {:?}", e))?;
    FreeRtos::delay_ms(1000);
    draw_start_up(&mut display)?;
    render_initial_menu(&mut display)?;
    loop {
        clear_display(&mut display)?;
        button::update_button_state(&button);
        match check_button_event() {
            ButtonEvent::LongPress => {
                break;
            },
            ButtonEvent::ShortPress => {
                update_initial_menu_state(&ButtonEvent::ShortPress)?;
                render_initial_menu(&mut display)?;
            },
            ButtonEvent::None => {}
        }
        FreeRtos::delay_ms(100);
    }

    let init_menu_state = INIT_MENU_DISPLAY_STATE.lock().map_err(|e| { anyhow::anyhow!("Mutex poisoned: {:?}", e)})?;

    match *init_menu_state {
        InitMenuDisplayOptions::Scan => {
            type MacAddress = [u8; 6];
            let mut mac_map: HashMap<MacAddress, bool> = HashMap::with_capacity(200);
            let (tx_mac_map, rx_mac_map) = mpsc::sync_channel(100);
            static mut TX: Option<SyncSender<MacAddress>> = None;

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
                    let destination_mac = frame_data[4..10].try_into().unwrap();
                    let source_mac = frame_data[10..16].try_into().unwrap();
                    
                    if let Some(tx) = &TX {
                        let _ = tx.send(destination_mac);
                        let _ = tx.send(source_mac);
                    }
                    
                    debug!("Frame: {} (type: {}, subtype: {})", type_str, frame_type, frame_subtype);
                    debug!("  Destination: {:?}", destination_mac);
                    debug!("  Source: {:?}", source_mac);
                }
            }

            let _wifi_driver = create_wifi_driver(peripherals.modem, sys_loop, nvs)?;
            unsafe {
                esp_wifi_set_promiscuous_rx_cb(Some(rx_callback));
            }

            // Run for 30 seconds then exit
            let start = std::time::Instant::now();
            let duration = std::time::Duration::from_secs(DURRATION_U64);
            let mut last_check_in_time = start;

            while start.elapsed() < duration {
                // Update button state
                button::update_button_state(&button);
                // Check for new MAC addresses
                match rx_mac_map.try_recv() {
                    Ok(mac) => {
                        mac_map.entry(mac).or_insert(true);
                    },
                    Err(mpsc::TryRecvError::Empty) => {},
                    Err(mpsc::TryRecvError::Disconnected) => {
                        info!("Channel disconnected");
                        break;
                    }
                }

                let button_event = button::check_button_event();
                if last_check_in_time.elapsed() >= std::time::Duration::from_secs(3) {
                    info!("Time remaining: {} seconds, Unique MACs: {}", 
                        DURRATION_U64 - start.elapsed().as_secs(),
                        mac_map.len()
                    );
                    // Update display with current status
                    draw_status_update(&mut display, &(DURRATION_U64 - start.elapsed().as_secs()), &mac_map.len(), &button_event)?;
                    last_check_in_time = std::time::Instant::now();
                }
                FreeRtos::delay_ms(100);
            }

            info!("Found {} unique MAC addresses", mac_map.len());

            draw_final_count(&mut display, &mac_map.len())?;
        
            info!("{} seconds elapsed, exiting...", DURRATION_U64);

        },
        InitMenuDisplayOptions::Size => {
            info!("Mounting SPIFFS filesystem");
            spiffs::mount(
                "/spffs"
            )?;
            let (total, used) = get_space_info()?;
            draw_text(&mut display, 5, 5, &format!("Total: {} bytes", total), true)?;
            draw_text(&mut display, 5, 15, &format!("Used: {} bytes", used), true)?;
            flush_display(&mut display)?;

            FreeRtos::delay_ms(5000);
        },
    }
    
    unsafe {
        esp_deep_sleep_start();
    }
}
