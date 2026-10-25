use async_trait::async_trait;

use super::{Collector, Fs};
use crate::error::ScanError;
use crate::model::{BootMode, DmiInfo, MachineInfo};

pub struct MachineCollector;

#[async_trait]
impl Collector for MachineCollector {
    type Output = MachineInfo;

    fn name(&self) -> &'static str {
        "machine"
    }

    async fn collect(&self, fs: &Fs) -> Result<MachineInfo, ScanError> {
        let boot_mode = if fs.exists("/sys/firmware/efi").await {
            BootMode::Uefi
        } else if fs.exists("/sys/firmware/dmi").await {
            BootMode::Bios
        } else {
            BootMode::Unknown
        };

        Ok(MachineInfo {
            hostname: fs.read_opt("/proc/sys/kernel/hostname").await,
            machine_id: fs.read_opt("/etc/machine-id").await,
            os: fs
                .read_opt("/etc/os-release")
                .await
                .and_then(|text| pretty_name(&text)),
            kernel: fs.read_opt("/proc/sys/kernel/osrelease").await,
            architecture: std::env::consts::ARCH.to_owned(),
            boot_mode,
            dmi: dmi(fs).await,
        })
    }
}

async fn dmi(fs: &Fs) -> DmiInfo {
    let field = |name: &'static str| async move {
        fs.read_opt(&format!("/sys/class/dmi/id/{name}"))
            .await
            .filter(|v| !is_placeholder(v))
    };

    DmiInfo {
        system_vendor: field("sys_vendor").await,
        product_name: field("product_name").await,
        product_serial: field("product_serial").await,
        product_uuid: field("product_uuid").await,
        board_vendor: field("board_vendor").await,
        board_name: field("board_name").await,
        board_serial: field("board_serial").await,
        bios_vendor: field("bios_vendor").await,
        bios_version: field("bios_version").await,
        bios_date: field("bios_date").await,
        chassis_type: field("chassis_type").await.and_then(|v| v.parse().ok()),
    }
}

/// Vendors fill unset DMI fields with boilerplate; treat it as missing.
fn is_placeholder(value: &str) -> bool {
    const JUNK: &[&str] = &[
        "to be filled by o.e.m.",
        "default string",
        "not specified",
        "not applicable",
        "system serial number",
        "none",
    ];
    JUNK.iter().any(|j| value.eq_ignore_ascii_case(j))
}

fn pretty_name(os_release: &str) -> Option<String> {
    os_release
        .lines()
        .find_map(|line| line.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim_matches('"').to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pretty_name() {
        let text = "NAME=\"Ubuntu\"\nPRETTY_NAME=\"Ubuntu 24.04.2 LTS\"\nID=ubuntu\n";
        assert_eq!(pretty_name(text).as_deref(), Some("Ubuntu 24.04.2 LTS"));
    }

    #[test]
    fn detects_placeholders() {
        assert!(is_placeholder("To Be Filled By O.E.M."));
        assert!(!is_placeholder("Dell Inc."));
    }
}
