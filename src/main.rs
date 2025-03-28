mod display;

use display::Display;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, 
    hal::{delay::FreeRtos, gpio::{InputPin, OutputPin}, i2c::{I2c, I2cConfig, I2cDriver}, peripheral::Peripheral, prelude::{Peripherals, FromValueType}}, 
    nvs::EspDefaultNvsPartition, 
    sys::{esp_wifi_set_mode, esp_wifi_set_promiscuous_filter, esp_wifi_set_promiscuous_rx_cb, i2c_trans_mode_t_I2C_DATA_MODE_LSB_FIRST, wifi_mode_t_WIFI_MODE_NULL, wifi_promiscuous_filter_t, WIFI_PROMIS_FILTER_MASK_ALL},
    wifi::{self, ClientConfiguration, WifiDriver}
};
use log::info;

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

    info!("Setting up I2C for display");
    let i2c = setup_i2c(peripherals.i2c0, peripherals.pins.gpio4, peripherals.pins.gpio15)?;
    
    // Initialize display
    info!("Initializing display");
    let mut display = Display::new(i2c)?;
    
    // Clear display
    display.clear()?;
    
    // Draw demo content
    display.draw_text(10, 10, "Hello from Rust!", true)?;
    display.draw_text(10, 25, "Detecting WiFi...", true)?;
    
    // Update display
    display.flush()?;

    let basic_client_config = ClientConfiguration {
        ssid: "".try_into().unwrap(),
        password: "".try_into().unwrap(),
        channel: Some(1),  // Set to a specific channel
        auth_method: wifi::AuthMethod::None,
        ..Default::default()
    };
    let wifi_config = wifi::Configuration::Client(basic_client_config);

    let mut wifi_driver = WifiDriver::new(peripherals.modem, sys_loop.clone(), Some(nvs))?;

    unsafe extern "C" fn rx_callback(buf: *mut core::ffi::c_void, _type: u32) {
        if !buf.is_null() {
            let frame_data = std::slice::from_raw_parts(buf as *const u8, 24); // Increased to 24 bytes to capture all MAC addresses
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
            let bssid = &frame_data[16..22];
    
            info!("Frame: {} (type: {}, subtype: {})", type_str, frame_type, frame_subtype);
            // info!("  Destination: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", 
            //     destination_mac[0], destination_mac[1], destination_mac[2],
            //     destination_mac[3], destination_mac[4], destination_mac[5]);
            info!("  Source: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                source_mac[0], source_mac[1], source_mac[2],
                source_mac[3], source_mac[4], source_mac[5]);
            info!("  BSSID: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                bssid[0], bssid[1], bssid[2],
                bssid[3], bssid[4], bssid[5]);
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
        if last_check_in_time.elapsed() >= std::time::Duration::from_secs(1) {
            info!("Time remaining: {} seconds", DURRATION_U64 - start.elapsed().as_secs());

            last_check_in_time = std::time::Instant::now();
        }
        FreeRtos::delay_ms(50);
    }

    // Cleanup before exit
    wifi_driver.set_promiscuous(false)?;
    wifi_driver.stop()?;
    info!("{} seconds elapsed, exiting...", DURRATION_U64);

    Ok(())
}

fn setup_i2c(i2c: impl Peripheral<P = impl I2c> + 'static, 
             sda: impl Peripheral<P = impl OutputPin + InputPin> + 'static,
             scl: impl Peripheral<P = impl OutputPin + InputPin> + 'static) -> anyhow::Result<I2cDriver<'static>> {
    
    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;
    
    Ok(i2c)
}