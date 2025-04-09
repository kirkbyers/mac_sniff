use std::fs::{File, OpenOptions};
use std::io::{self, Write, Read};
use std::path::Path;
use anyhow::Result;
use log::info;

use esp_idf_svc::fs::{EspFs, FsType};

pub struct SpiffsStorage {
    base_path: &'static str,
    initialized: bool,
}

impl SpiffsStorage {
    pub fn new(base_path: &'static str) -> Result<Self> {
        // Mount SPIFFS
        if !Path::new(base_path).exists() {
            EspFs::mount(FsType::Spiffs, "storage", base_path)?;
        }
        
        info!("SPIFFS mounted at: {}", base_path);
        Ok(Self { 
            base_path,
            initialized: true,
        })
    }

    // Write MAC addresses as raw binary - each MAC is exactly 6 bytes
    pub fn write_macs_binary(&self, macs: &[[u8; 6]], filename: &str) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("SPIFFS not initialized"));
        }

        let path = format!("{}/{}", self.base_path, filename);
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        
        // Write count as a u32 (4 bytes)
        let count = macs.len() as u32;
        file.write_all(&count.to_le_bytes())?;
        
        // Write each MAC address as raw bytes (6 bytes each)
        for mac in macs {
            file.write_all(mac)?;
        }
        
        info!("Wrote {} MAC addresses to {} in binary format", macs.len(), path);
        info!("Total bytes: {} (4 byte header + {} × 6 bytes per MAC)", 4 + macs.len() * 6, macs.len());
        Ok(())
    }

    // Append a single MAC in binary format
    pub fn append_mac_binary(&self, mac: &[u8; 6], filename: &str) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("SPIFFS not initialized"));
        }

        let path = format!("{}/{}", self.base_path, filename);
        let file_exists = Path::new(&path).exists();
        
        if !file_exists {
            // New file - initialize with count = 1
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&path)?;
            
            // Write initial count as 1
            let count: u32 = 1;
            file.write_all(&count.to_le_bytes())?;
            
            // Write the MAC
            file.write_all(mac)?;
        } else {
            // File exists - update count and append MAC
            let mut file = OpenOptions::new()
                .read(true)
                .open(&path)?;
            
            // Read current count
            let mut count_bytes = [0u8; 4];
            file.read_exact(&mut count_bytes)?;
            let mut count = u32::from_le_bytes(count_bytes);
            
            // Increment count
            count += 1;
            
            // Reopen file for writing and update count + append MAC
            let mut file = OpenOptions::new()
                .write(true)
                .open(&path)?;
            
            // Update count at beginning of file
            file.write_all(&count.to_le_bytes())?;
            
            // Seek to end to append the new MAC
            use std::io::Seek;
            file.seek(std::io::SeekFrom::End(0))?;
            file.write_all(mac)?;
        }
        
        Ok(())
    }
    
    // Read all MACs from binary file
    pub fn read_macs_binary(&self, filename: &str) -> Result<Vec<[u8; 6]>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("SPIFFS not initialized"));
        }

        let path = format!("{}/{}", self.base_path, filename);
        if !Path::new(&path).exists() {
            return Ok(Vec::new());
        }
        
        let mut file = OpenOptions::new()
            .read(true)
            .open(&path)?;
        
        // Read count (first 4 bytes)
        let mut count_bytes = [0u8; 4];
        file.read_exact(&mut count_bytes)?;
        let count = u32::from_le_bytes(count_bytes) as usize;
        
        // Read all MAC addresses
        let mut macs = Vec::with_capacity(count);
        for _ in 0..count {
            let mut mac = [0u8; 6];
            match file.read_exact(&mut mac) {
                Ok(_) => macs.push(mac),
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        // Handle corrupted file gracefully
                        info!("Reached end of file before reading all MAC addresses. File might be corrupted.");
                        break;
                    } else {
                        return Err(anyhow::anyhow!("Error reading MAC address: {}", e));
                    }
                }
            }
        }
        
        info!("Read {} MAC addresses from binary file {}", macs.len(), path);
        Ok(macs)
    }
}
