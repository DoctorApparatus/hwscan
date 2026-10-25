use async_trait::async_trait;

use super::{Collector, Fs};
use crate::error::ScanError;
use crate::model::{NicInfo, NicKind};

pub struct NicCollector;

#[async_trait]
impl Collector for NicCollector {
    type Output = Vec<NicInfo>;

    fn name(&self) -> &'static str {
        "nics"
    }

    async fn collect(&self, fs: &Fs) -> Result<Vec<NicInfo>, ScanError> {
        let mut nics = Vec::new();

        for name in fs.list_dir("/sys/class/net").await? {
            if name == "lo" {
                continue;
            }
            let base = format!("/sys/class/net/{name}");

            let kind = if fs.exists(&format!("{base}/device")).await {
                NicKind::Physical
            } else {
                NicKind::Virtual
            };

            nics.push(NicInfo {
                mac: fs
                    .read_opt(&format!("{base}/address"))
                    .await
                    .filter(|m| m != "00:00:00:00:00:00"),
                kind,
                driver: fs.link_basename(&format!("{base}/device/driver")).await,
                link_up: Some(
                    fs.read_opt(&format!("{base}/operstate")).await.as_deref() == Some("up"),
                ),
                // Reads as -1 (or fails with EINVAL) while the link is down.
                speed_mbps: fs
                    .read_opt(&format!("{base}/speed"))
                    .await
                    .and_then(|v| v.parse::<i64>().ok())
                    .and_then(|v| u32::try_from(v).ok())
                    .filter(|v| *v > 0),
                mtu: fs
                    .read_opt(&format!("{base}/mtu"))
                    .await
                    .and_then(|v| v.parse().ok()),
                name,
            });
        }

        Ok(nics)
    }
}
