use std::ptr::null;
use std::ffi::CString;
use log::{info, error};
use std::fs::File;
use std::io::Write;

use esp_idf_hal::sys::{esp_vfs_spiffs_conf_t, esp_vfs_spiffs_register, esp_vfs_spiffs_unregister, ESP_OK, esp_spiffs_info};

pub fn mount(path: &str) -> anyhow::Result<()> {
    let base_path = CString::new(path).unwrap();
    let spiffs_config = esp_vfs_spiffs_conf_t {
        base_path: base_path.as_ptr(),
        partition_label: null(),
        max_files: 5,
        format_if_mount_failed: true  // Format if mount fails
    };
    
    unsafe {
        let result = esp_vfs_spiffs_register(&spiffs_config);
        if result != ESP_OK {
            error!("Failed to mount SPIFFS filesystem. Error code: {}", result);
            return Err(anyhow::anyhow!("SPIFFS mount failed with error code: {}", result));
        }
        
        info!("SPIFFS mounted successfully at {}", path);
    }
    
    Ok(())
}

pub fn unmount() -> anyhow::Result<()> {
    unsafe {
        let result = esp_vfs_spiffs_unregister(null());
        if result != ESP_OK {
            error!("Failed to unmount SPIFFS filesystem. Error code: {}", result);
            return Err(anyhow::anyhow!("SPIFFS unmount failed with error code: {}", result));
        }
        
        info!("SPIFFS unmounted successfully");
    }
    
    Ok(())
}

pub fn get_space_info() -> anyhow::Result<(usize, usize)> {
    let mut total_bytes: usize = 0;
    let mut used_bytes: usize = 0;
    
    unsafe {
        let result = esp_spiffs_info(null(), &mut total_bytes, &mut used_bytes);
        if result != ESP_OK {
            error!("Failed to get SPIFFS info. Error code: {}", result);
            return Err(anyhow::anyhow!("Failed to get SPIFFS info with error code: {}", result));
        }
    }
    
    info!("SPIFFS info - total: {} bytes, used: {} bytes", total_bytes, used_bytes);
    Ok((total_bytes, used_bytes))
}

pub fn save_to_file(path: &str, data: &[u8]) -> anyhow::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(data)?;
    info!("Successfully wrote {} bytes to {}", data.len(), path);
    Ok(())
}

pub fn has_enough_space(needed_bytes: usize) -> anyhow::Result<bool> {
    let (total, used) = get_space_info()?;
    let available = total - used;
    info!("SPIFFS space check - available: {} bytes, needed: {} bytes", available, needed_bytes);
    Ok(available >= needed_bytes)
}