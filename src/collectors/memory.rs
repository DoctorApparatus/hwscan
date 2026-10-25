use async_trait::async_trait;

use super::{Collector, Fs};
use crate::error::ScanError;
use crate::model::MemoryInfo;

pub struct MemoryCollector;

#[async_trait]
impl Collector for MemoryCollector {
    type Output = MemoryInfo;

    fn name(&self) -> &'static str {
        "memory"
    }

    async fn collect(&self, fs: &Fs) -> Result<MemoryInfo, ScanError> {
        parse_meminfo(&fs.read("/proc/meminfo").await?)
    }
}

pub fn parse_meminfo(text: &str) -> Result<MemoryInfo, ScanError> {
    let kib = |key: &str| -> Option<u64> {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix(':'))
            .and_then(|rest| rest.split_whitespace().next()?.parse().ok())
    };

    let total = kib("MemTotal").ok_or_else(|| ScanError::Parse {
        what: "/proc/meminfo",
        reason: "MemTotal missing".into(),
    })?;

    Ok(MemoryInfo {
        total_bytes: total * 1024,
        swap_bytes: kib("SwapTotal").unwrap_or(0) * 1024,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_totals() {
        let mem =
            parse_meminfo("MemTotal:       16384 kB\nMemFree: 1 kB\nSwapTotal:       2048 kB\n")
                .unwrap();
        assert_eq!(mem.total_bytes, 16384 * 1024);
        assert_eq!(mem.swap_bytes, 2048 * 1024);
    }

    #[test]
    fn missing_total_is_an_error() {
        assert!(parse_meminfo("SwapTotal: 1 kB\n").is_err());
    }
}
