//! Probes that read hardware facts from `/proc` and `/sys`.
//!
//! Each probe is a [`Collector`]. A failing probe never aborts the scan: its
//! section stays at the default value and the error lands in `warnings`.

// Linux-only probes (/proc, /sys). Compiled everywhere so they stay testable.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod cpu;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod disks;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod machine;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod memory;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod nics;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod pci;
// sysinfo-based probes for everything else (Windows, macOS).
#[cfg_attr(target_os = "linux", allow(dead_code))]
mod portable;

#[cfg(not(target_os = "linux"))]
use self::portable::{
    CpuCollector, DiskCollector, MachineCollector, MemoryCollector, NicCollector, PciCollector,
};
#[cfg(target_os = "linux")]
use self::{
    cpu::CpuCollector, disks::DiskCollector, machine::MachineCollector, memory::MemoryCollector,
    nics::NicCollector, pci::PciCollector,
};

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use crate::error::ScanError;
use crate::model::HardwareReport;

/// Filesystem view rooted at `/` (or a fixture directory in tests).
#[derive(Debug, Clone)]
pub struct Fs {
    root: PathBuf,
}

impl Fs {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path(&self, p: &str) -> PathBuf {
        self.root.join(p.trim_start_matches('/'))
    }

    pub async fn read(&self, p: &str) -> Result<String, ScanError> {
        let path = self.path(p);
        tokio::fs::read_to_string(&path)
            .await
            .map_err(|source| ScanError::Io { path, source })
    }

    /// Trimmed file content; `None` if unreadable or empty.
    pub async fn read_opt(&self, p: &str) -> Option<String> {
        let text = tokio::fs::read_to_string(self.path(p)).await.ok()?;
        let text = text.trim();
        (!text.is_empty()).then(|| text.to_owned())
    }

    pub async fn exists(&self, p: &str) -> bool {
        tokio::fs::try_exists(self.path(p)).await.unwrap_or(false)
    }

    /// Sorted entry names of a directory.
    pub async fn list_dir(&self, p: &str) -> Result<Vec<String>, ScanError> {
        let path = self.path(p);
        let io = |source| ScanError::Io {
            path: path.clone(),
            source,
        };
        let mut rd = tokio::fs::read_dir(&path).await.map_err(io)?;
        let mut names = Vec::new();
        while let Some(entry) = rd.next_entry().await.map_err(io)? {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        names.sort();
        Ok(names)
    }

    /// Final path component of a symlink target (e.g. a driver name).
    pub async fn link_basename(&self, p: &str) -> Option<String> {
        let target = tokio::fs::read_link(self.path(p)).await.ok()?;
        Some(target.file_name()?.to_string_lossy().into_owned())
    }
}

#[async_trait]
pub trait Collector: Send + Sync {
    type Output: Default + Send;

    fn name(&self) -> &'static str;

    async fn collect(&self, fs: &Fs) -> Result<Self::Output, ScanError>;
}

async fn run<C: Collector>(collector: C, fs: &Fs) -> (C::Output, Option<String>) {
    match collector.collect(fs).await {
        Ok(out) => (out, None),
        Err(err) => {
            tracing::warn!(collector = collector.name(), %err, "collector failed");
            (
                C::Output::default(),
                Some(format!("{}: {err}", collector.name())),
            )
        }
    }
}

/// Run every collector concurrently and assemble the report.
pub async fn scan(fs: &Fs) -> HardwareReport {
    let (
        (machine, w_machine),
        (cpu, w_cpu),
        (memory, w_memory),
        (disks, w_disks),
        (nics, w_nics),
        (pci, w_pci),
    ) = tokio::join!(
        run(MachineCollector, fs),
        run(CpuCollector, fs),
        run(MemoryCollector, fs),
        run(DiskCollector, fs),
        run(NicCollector, fs),
        run(PciCollector, fs),
    );

    let warnings = [w_machine, w_cpu, w_memory, w_disks, w_nics, w_pci]
        .into_iter()
        .flatten()
        .collect();

    HardwareReport::builder()
        .agent_version(env!("CARGO_PKG_VERSION").to_owned())
        .collected_at(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or_default(),
        )
        .machine(machine)
        .cpu(cpu)
        .memory(memory)
        .disks(disks)
        .nics(nics)
        .pci(pci)
        .warnings(warnings)
        .build()
}
