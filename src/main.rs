use std::iter;

use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::prelude::Peripherals, nvs::EspDefaultNvsPartition, wifi::{BlockingWifi, EspWifi, WifiDriver}};
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

    // let mut wifi = BlockingWifi::wrap(
    //     EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
    //     sys_loop
    // )?;

    let mut wifi_driver = WifiDriver::new(peripherals.modem, sys_loop.clone(), Some(nvs))?;
    wifi_driver.start()?;

    let (res, aps_count) = wifi_driver.scan_n::<3>()?;

    info!("{:?}", res);
    info!("{:?}", aps_count);

    for ap in res.iter() {
        info!("{:?}", ap.bssid);
    }

    Ok(())
}
