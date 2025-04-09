use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use anyhow::Result;
use log::{info, error};
use esp_idf_hal::gpio::OutputPin;
use esp_idf_hal::spi::SpiDeviceDriver;
use esp_idf_svc::fs::{EspFs, FsType, StorageType};

pub struct SdCardStorage<'d> {
    base_path: &'static str,
    initialized: bool,
}

impl<'d> SdCardStorage<'d> {
    pub fn new<CS: OutputPin + 'd>(
        base_path: &'static str,
        spi: SpiDeviceDriver<'d>,
        cs: CS
    ) -> Result<Self> {
        // Mount SD card
        // Note: You'll need to configure SPI and GPIO pins properly
        EspFs::mount_sdmmc_spi(spi, cs, StorageType::SDCard, base_path)?;
        
        info!("SD card mounted at: {}", base_path);
        Ok(Self { 
            base_path,
            initialized: true,
        })
    }

    pub fn write_macs_csv(&self, macs: &[[u8; 6]], filename: &str) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("SD card not initialized"));
        }

        let path = format!("{}/{}", self.base_path, filename);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        // Write CSV header
        writeln!(file, "index,mac_address,timestamp")?;
        
        // Write each MAC with index
        for (i, mac) in macs.iter().enumerate() {
            writeln!(
                file,
                "{},{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x},{}",
                i, mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
                esp_idf_hal::delay::SystemTime::now().as_secs()
            )?;
        }
        
        info!("Wrote {} MAC addresses to CSV file {}", macs.len(), path);
        Ok(())
    }
    
    pub fn append_mac(&self, mac: &[u8; 6], filename: &str) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("SD card not initialized"));
        }

        let path = format!("{}/{}", self.base_path, filename);
        let file_exists = Path::new(&path).exists();
        
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&path)?;

        // Write header if file is new
        if !file_exists {
            writeln!(file, "index,mac_address,timestamp")?;
        }
        
        // Append MAC with timestamp
        writeln!(
            file,
            "{},{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x},{}",
            0, // You'd need to track the index separately
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
            esp_idf_hal::delay::SystemTime::now().as_secs()
        )?;
        
        Ok(())
    }
}
