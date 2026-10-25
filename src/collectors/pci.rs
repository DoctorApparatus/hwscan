use async_trait::async_trait;

use super::{Collector, Fs};
use crate::error::ScanError;
use crate::model::{PciClass, PciDevice};

pub struct PciCollector;

#[async_trait]
impl Collector for PciCollector {
    type Output = Vec<PciDevice>;

    fn name(&self) -> &'static str {
        "pci"
    }

    async fn collect(&self, fs: &Fs) -> Result<Vec<PciDevice>, ScanError> {
        // Absent on machines without PCI (some ARM boards, containers).
        if !fs.exists("/sys/bus/pci/devices").await {
            return Ok(Vec::new());
        }

        let mut devices = Vec::new();
        for address in fs.list_dir("/sys/bus/pci/devices").await? {
            let base = format!("/sys/bus/pci/devices/{address}");
            let (Some(vendor_id), Some(device_id), Some(class)) = (
                fs.read_opt(&format!("{base}/vendor")).await,
                fs.read_opt(&format!("{base}/device")).await,
                fs.read_opt(&format!("{base}/class")).await,
            ) else {
                continue;
            };

            devices.push(PciDevice {
                class_name: classify(&class),
                address,
                vendor_id,
                device_id,
                class,
            });
        }

        Ok(devices)
    }
}

/// Map the base-class byte of a `0xCCSSPP` class code.
fn classify(class: &str) -> PciClass {
    let code = u32::from_str_radix(class.trim_start_matches("0x"), 16).unwrap_or(0);
    match code >> 16 {
        0x01 => PciClass::Storage,
        0x02 => PciClass::Network,
        0x03 => PciClass::Display,
        0x06 => PciClass::Bridge,
        _ => PciClass::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_base_class() {
        assert_eq!(classify("0x030000"), PciClass::Display);
        assert_eq!(classify("0x020000"), PciClass::Network);
        assert_eq!(classify("0x010802"), PciClass::Storage);
        assert_eq!(classify("garbage"), PciClass::Other);
    }
}
