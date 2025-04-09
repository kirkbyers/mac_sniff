pub mod nvs;
pub mod spiffs;
pub mod sdcard;

use anyhow::Result;

pub enum StorageType {
    Nvs,
    Spiffs,
    SdCard,
}

pub trait MacStorage {
    fn store_mac(&mut self, mac: &[u8; 6]) -> Result<()>;
    fn store_mac_batch(&mut self, macs: &[[u8; 6]]) -> Result<()>;
    fn get_mac_count(&self) -> Result<usize>;
}
