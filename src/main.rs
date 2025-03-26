use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::prelude::Peripherals, nvs::EspDefaultNvsPartition, sys::EspError, wifi::{self, AccessPointConfiguration, BlockingWifi, ClientConfiguration, EspWifi, WifiDeviceId, WifiDriver, WifiFrame}};
use log::info;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let wifi_client_config = ClientConfiguration{
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: wifi::AuthMethod::WAPIPersonal,
        password: PASSWORD.try_into().unwrap(),
        channel: None,
        ..Default::default()
    };
    let wifi_ap_config = AccessPointConfiguration {
        ssid: "ATRIP Rainbows".try_into().unwrap(),
        ssid_hidden: false,
        auth_method: wifi::AuthMethod::None,
        ..Default::default()
    };
    let wifi_config = wifi::Configuration::Mixed(wifi_client_config, wifi_ap_config);

    let mut wifi_driver = WifiDriver::new(peripherals.modem, sys_loop.clone(), Some(nvs))?;

    let rx_callback = |_wifi_device_id: WifiDeviceId, wifi_frame: WifiFrame| -> Result<(), EspError> {
        let frame_data = wifi_frame.as_slice();
        if frame_data.len() >= 14 {  // Minimum ethernet frame size
            // MAC addresses
            let source_mac = &frame_data[6..12];
            
            info!("Source MAC: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", 
                source_mac[0], source_mac[1], source_mac[2], source_mac[3], source_mac[4], source_mac[5]);
            
        }
        
        Ok(())
    };
    let tx_callback = |wifi_device_id: WifiDeviceId, _data: &[u8], _status: bool| {
        match wifi_device_id {
            WifiDeviceId::Ap | WifiDeviceId::Sta => {
                // Only log if needed for debugging
                // info!("TX from {:?}", wifi_device_id);
            }
        }
    };

    wifi_driver.set_callbacks(rx_callback, tx_callback)?;

    wifi_driver.set_configuration(&wifi_config)?;
    wifi_driver.set_promiscuous(true)?;
    wifi_driver.start()?;

    info!("Wifi Started");

    // Note: Don't connect if you're going to scan
    // wifi_driver.connect()?;
    // info!("Wifi Connected");

    let (scan_result, _found_aps_count) = wifi_driver.scan_n::<10>()?;

    for ap in scan_result.iter() {
        info!("Found AP: {:?}", ap);
    }

    // Run for 30 seconds then exit
    let start = std::time::Instant::now();
    let duration = std::time::Duration::from_secs(30);
    while start.elapsed() < duration {
        std::thread::sleep(std::time::Duration::from_secs(1));
        info!("Time remaining: {} seconds", 30 - start.elapsed().as_secs());
    }

    info!("30 seconds elapsed, exiting...");

    Ok(())
}
