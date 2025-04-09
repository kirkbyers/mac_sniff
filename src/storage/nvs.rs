use esp_idf_svc::nvs::{EspNvs, NvsDefault};
use anyhow::Result;
use log::info;

pub struct NvsStorage {
    nvs: EspNvs<NvsDefault>,
    namespace: &'static str,
}

impl NvsStorage {
    pub fn new(namespace: &'static str) -> Result<Self> {
        let nvs = EspNvs::new(NvsDefault::partition(), namespace, true)?;
        info!("NVS storage initialized with namespace: {}", namespace);
        Ok(Self { nvs, namespace })
    }

    pub fn store_mac_count(&mut self, count: u32) -> Result<()> {
        self.nvs.set_u32("mac_count", count)?;
        Ok(())
    }

    pub fn store_mac_address(&mut self, index: u32, mac: &[u8; 6]) -> Result<()> {
        let key = format!("mac_{}", index);
        // Store as blob (binary data)
        self.nvs.set_blob(&key, mac)?;
        Ok(())
    }

    pub fn get_mac_count(&self) -> Result<u32> {
        Ok(self.nvs.get_u32("mac_count")?.unwrap_or(0))
    }

    pub fn get_mac_address(&self, index: u32) -> Result<Option<[u8; 6]>> {
        let key = format!("mac_{}", index);
        let mut mac = [0u8; 6];
        
        if self.nvs.get_blob(&key, &mut mac)?.is_some() {
            Ok(Some(mac))
        } else {
            Ok(None)
        }
    }
}
