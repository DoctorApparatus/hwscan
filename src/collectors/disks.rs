use async_trait::async_trait;

use super::{Collector, Fs};
use crate::error::ScanError;
use crate::model::{DiskInfo, DiskKind};

/// Pseudo and stacked devices that are not physical disks.
const IGNORED_PREFIXES: &[&str] = &["loop", "ram", "zram", "sr", "fd", "dm-", "md", "nbd"];

pub struct DiskCollector;

#[async_trait]
impl Collector for DiskCollector {
    type Output = Vec<DiskInfo>;

    fn name(&self) -> &'static str {
        "disks"
    }

    async fn collect(&self, fs: &Fs) -> Result<Vec<DiskInfo>, ScanError> {
        let mut disks = Vec::new();

        for name in fs.list_dir("/sys/block").await? {
            if IGNORED_PREFIXES.iter().any(|p| name.starts_with(p)) {
                continue;
            }
            let base = format!("/sys/block/{name}");

            // `size` is always in 512-byte units, regardless of sector size.
            let sectors: u64 = fs
                .read_opt(&format!("{base}/size"))
                .await
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            if sectors == 0 {
                continue; // empty card reader slot etc.
            }

            let rotational = fs
                .read_opt(&format!("{base}/queue/rotational"))
                .await
                .as_deref()
                == Some("1");
            let kind = if name.starts_with("nvme") {
                DiskKind::Nvme
            } else if rotational {
                DiskKind::Hdd
            } else {
                DiskKind::Ssd
            };

            disks.push(DiskInfo {
                kind,
                size_bytes: sectors * 512,
                logical_block_size: fs
                    .read_opt(&format!("{base}/queue/logical_block_size"))
                    .await
                    .and_then(|v| v.parse().ok()),
                model: fs.read_opt(&format!("{base}/device/model")).await,
                vendor: fs.read_opt(&format!("{base}/device/vendor")).await,
                serial: fs.read_opt(&format!("{base}/device/serial")).await,
                removable: fs.read_opt(&format!("{base}/removable")).await.as_deref() == Some("1"),
                name,
            });
        }

        Ok(disks)
    }
}
