use esp_idf_hal::{modem::Modem, sys::wifi_mode_t_WIFI_MODE_NULL};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop, 
    nvs::EspDefaultNvsPartition, 
    wifi::{self, ClientConfiguration, WifiDriver},
    sys::{esp_wifi_set_mode, esp_wifi_set_promiscuous_filter, wifi_promiscuous_filter_t, WIFI_PROMIS_FILTER_MASK_ALL},
};
use anyhow::Result;
use log::debug;


pub fn create_wifi_driver(modem: Modem, sys_loop: EspSystemEventLoop, nvs: EspDefaultNvsPartition) -> Result<WifiDriver<'static>> {
    let basic_client_config = ClientConfiguration {
        ssid: "".try_into().unwrap(),
        password: "".try_into().unwrap(),
        channel: Some(1),  // Set to a specific channel
        auth_method: wifi::AuthMethod::None,
        ..Default::default()
    };
    let wifi_config = wifi::Configuration::Client(basic_client_config);
    let mut wifi_driver = WifiDriver::new(modem, sys_loop.clone(), Some(nvs)).map_err(|e| { anyhow::anyhow!("Error creating wifi driver: {:?}", e)})?;

    wifi_driver.set_configuration(&wifi_config)?;
    unsafe { 
        esp_wifi_set_mode(wifi_mode_t_WIFI_MODE_NULL);
    };

    wifi_driver.start()?;
    wifi_driver.set_promiscuous(true)?;

    unsafe {
        let filter = wifi_promiscuous_filter_t {
            filter_mask: WIFI_PROMIS_FILTER_MASK_ALL,
        };
        esp_wifi_set_promiscuous_filter(&filter);
    }

    debug!("Wifi Started");

    Ok(wifi_driver)
}