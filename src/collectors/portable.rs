//! Cross-platform probes built on `sysinfo`. Used on Windows and macOS.
//!
//! Less detail than the Linux probes: no DMI, no PCI, no disk model or serial,
//! and `sysinfo` lists mounted volumes rather than physical drives. Fields it
//! cannot know are `None` or `Unknown` instead of guessed.

use async_trait::async_trait;
use sysinfo::{Disks, Networks, System};

use super::{Collector, Fs};
use crate::error::ScanError;
use crate::model::{
    BootMode, CpuInfo, DiskInfo, DiskKind, DmiInfo, MachineInfo, MemoryInfo, NicInfo, NicKind,
    PciDevice,
};

pub struct MachineCollector;

#[async_trait]
impl Collector for MachineCollector {
    type Output = MachineInfo;

    fn name(&self) -> &'static str {
        "machine"
    }

    async fn collect(&self, _fs: &Fs) -> Result<MachineInfo, ScanError> {
        Ok(tokio::task::spawn_blocking(|| MachineInfo {
            hostname: System::host_name(),
            machine_id: None,
            os: System::long_os_version(),
            kernel: System::kernel_version(),
            architecture: std::env::consts::ARCH.to_owned(),
            boot_mode: BootMode::Unknown,
            dmi: DmiInfo::default(),
        })
        .await?)
    }
}

pub struct CpuCollector;

#[async_trait]
impl Collector for CpuCollector {
    type Output = CpuInfo;

    fn name(&self) -> &'static str {
        "cpu"
    }

    async fn collect(&self, _fs: &Fs) -> Result<CpuInfo, ScanError> {
        Ok(tokio::task::spawn_blocking(|| {
            let mut sys = System::new();
            sys.refresh_cpu_all();
            let cpus = sys.cpus();
            let logical = cpus.len() as u32;
            CpuInfo {
                model: cpus.first().map(|c| c.brand().trim().to_owned()),
                vendor: cpus.first().map(|c| c.vendor_id().to_owned()),
                // sysinfo has no socket count; assume one when any CPU exists.
                sockets: u32::from(logical > 0),
                physical_cores: System::physical_core_count().map_or(logical, |n| n as u32),
                logical_cpus: logical,
                max_mhz: None,
                virtualization: None,
            }
        })
        .await?)
    }
}

pub struct MemoryCollector;

#[async_trait]
impl Collector for MemoryCollector {
    type Output = MemoryInfo;

    fn name(&self) -> &'static str {
        "memory"
    }

    async fn collect(&self, _fs: &Fs) -> Result<MemoryInfo, ScanError> {
        Ok(tokio::task::spawn_blocking(|| {
            let mut sys = System::new();
            sys.refresh_memory();
            MemoryInfo {
                total_bytes: sys.total_memory(),
                swap_bytes: sys.total_swap(),
            }
        })
        .await?)
    }
}

pub struct DiskCollector;

#[async_trait]
impl Collector for DiskCollector {
    type Output = Vec<DiskInfo>;

    fn name(&self) -> &'static str {
        "disks"
    }

    async fn collect(&self, _fs: &Fs) -> Result<Vec<DiskInfo>, ScanError> {
        Ok(tokio::task::spawn_blocking(|| {
            let mut disks: Vec<DiskInfo> = Disks::new_with_refreshed_list()
                .iter()
                .filter(|d| d.total_space() > 0)
                .map(|d| DiskInfo {
                    name: d.name().to_string_lossy().into_owned(),
                    kind: match d.kind() {
                        sysinfo::DiskKind::SSD => DiskKind::Ssd,
                        sysinfo::DiskKind::HDD => DiskKind::Hdd,
                        _ => DiskKind::Unknown,
                    },
                    size_bytes: d.total_space(),
                    logical_block_size: None,
                    model: None,
                    vendor: None,
                    serial: None,
                    removable: d.is_removable(),
                })
                .collect();
            disks.sort_by(|a, b| a.name.cmp(&b.name));
            disks
        })
        .await?)
    }
}

pub struct NicCollector;

#[async_trait]
impl Collector for NicCollector {
    type Output = Vec<NicInfo>;

    fn name(&self) -> &'static str {
        "nics"
    }

    async fn collect(&self, _fs: &Fs) -> Result<Vec<NicInfo>, ScanError> {
        Ok(tokio::task::spawn_blocking(|| {
            let mut nics: Vec<NicInfo> = Networks::new_with_refreshed_list()
                .iter()
                .filter(|(name, _)| !name.eq_ignore_ascii_case("lo"))
                .map(|(name, data)| NicInfo {
                    name: name.clone(),
                    mac: Some(data.mac_address())
                        .filter(|m| !m.is_unspecified())
                        .map(|m| m.to_string()),
                    kind: NicKind::Unknown,
                    driver: None,
                    link_up: None,
                    speed_mbps: None,
                    mtu: u32::try_from(data.mtu()).ok().filter(|m| *m > 0),
                })
                .collect();
            nics.sort_by(|a, b| a.name.cmp(&b.name));
            nics
        })
        .await?)
    }
}

/// `sysinfo` cannot enumerate PCI devices.
pub struct PciCollector;

#[async_trait]
impl Collector for PciCollector {
    type Output = Vec<PciDevice>;

    fn name(&self) -> &'static str {
        "pci"
    }

    async fn collect(&self, _fs: &Fs) -> Result<Vec<PciDevice>, ScanError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Runs the portable probes on whatever host executes the tests.
    #[tokio::test]
    async fn probes_this_host() {
        let fs = Fs::new("/");
        let cpu = CpuCollector.collect(&fs).await.unwrap();
        assert!(cpu.logical_cpus > 0);
        assert!(cpu.physical_cores > 0);
        assert!(MemoryCollector.collect(&fs).await.unwrap().total_bytes > 0);
        assert!(
            MachineCollector
                .collect(&fs)
                .await
                .unwrap()
                .hostname
                .is_some()
        );
        assert!(PciCollector.collect(&fs).await.unwrap().is_empty());
    }
}
