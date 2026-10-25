use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use super::{Collector, Fs};
use crate::error::ScanError;
use crate::model::CpuInfo;

pub struct CpuCollector;

#[async_trait]
impl Collector for CpuCollector {
    type Output = CpuInfo;

    fn name(&self) -> &'static str {
        "cpu"
    }

    async fn collect(&self, fs: &Fs) -> Result<CpuInfo, ScanError> {
        let mut cpu = parse_cpuinfo(&fs.read("/proc/cpuinfo").await?);
        cpu.max_mhz = fs
            .read_opt("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
            .await
            .and_then(|khz| khz.parse::<u32>().ok())
            .map(|khz| khz / 1000);
        Ok(cpu)
    }
}

pub fn parse_cpuinfo(text: &str) -> CpuInfo {
    let mut logical = 0u32;
    let mut sockets = HashSet::new();
    let mut cores = HashSet::new();
    let mut model = None;
    let mut vendor = None;
    let mut virtualization = false;

    for block in text.split("\n\n") {
        let fields: HashMap<&str, &str> = block
            .lines()
            .filter_map(|line| line.split_once(':'))
            .map(|(k, v)| (k.trim(), v.trim()))
            .collect();

        if !fields.contains_key("processor") {
            continue;
        }
        logical += 1;

        if model.is_none() {
            model = ["model name", "Model Name", "Hardware", "cpu model"]
                .iter()
                .find_map(|k| fields.get(k).filter(|v| !v.is_empty()))
                .map(|v| (*v).to_owned());
        }
        if vendor.is_none() {
            vendor = ["vendor_id", "CPU implementer"]
                .iter()
                .find_map(|k| fields.get(k))
                .map(|v| (*v).to_owned());
        }
        if let Some(pkg) = fields.get("physical id") {
            sockets.insert(*pkg);
            if let Some(core) = fields.get("core id") {
                cores.insert((*pkg, *core));
            }
        }
        if fields
            .get("flags")
            .is_some_and(|f| f.split_whitespace().any(|f| f == "vmx" || f == "svm"))
        {
            virtualization = true;
        }
    }

    CpuInfo {
        model,
        vendor,
        sockets: if sockets.is_empty() {
            u32::from(logical > 0)
        } else {
            sockets.len() as u32
        },
        physical_cores: if cores.is_empty() {
            logical
        } else {
            cores.len() as u32
        },
        logical_cpus: logical,
        max_mhz: None,
        virtualization: Some(virtualization),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
processor\t: 0
vendor_id\t: GenuineIntel
model name\t: Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz
physical id\t: 0
core id\t\t: 0
flags\t\t: fpu vme vmx sse

processor\t: 1
vendor_id\t: GenuineIntel
model name\t: Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz
physical id\t: 0
core id\t\t: 0
flags\t\t: fpu vme vmx sse

processor\t: 2
vendor_id\t: GenuineIntel
model name\t: Intel(R) Xeon(R) CPU E5-2680 v4 @ 2.40GHz
physical id\t: 1
core id\t\t: 0
flags\t\t: fpu vme vmx sse
";

    #[test]
    fn counts_sockets_cores_threads() {
        let cpu = parse_cpuinfo(SAMPLE);
        assert_eq!(cpu.logical_cpus, 3);
        assert_eq!(cpu.sockets, 2);
        assert_eq!(cpu.physical_cores, 2);
        assert_eq!(cpu.virtualization, Some(true));
        assert_eq!(cpu.vendor.as_deref(), Some("GenuineIntel"));
    }

    #[test]
    fn empty_input_is_zeroed() {
        let cpu = parse_cpuinfo("");
        assert_eq!(cpu.logical_cpus, 0);
        assert_eq!(cpu.sockets, 0);
    }
}
