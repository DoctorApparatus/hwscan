//! Wire format sent to the controller. Run `hwscan schema` for the JSON Schema.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use typed_builder::TypedBuilder;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TypedBuilder)]
pub struct HardwareReport {
    #[builder(default = SCHEMA_VERSION)]
    pub schema_version: u32,
    pub agent_version: String,
    /// Unix seconds.
    pub collected_at: u64,
    pub machine: MachineInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub disks: Vec<DiskInfo>,
    pub nics: Vec<NicInfo>,
    pub pci: Vec<PciDevice>,
    /// Non-fatal problems hit while probing (missing files, no permission, ...).
    #[builder(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct MachineInfo {
    pub hostname: Option<String>,
    pub machine_id: Option<String>,
    pub os: Option<String>,
    pub kernel: Option<String>,
    pub architecture: String,
    pub boot_mode: BootMode,
    pub dmi: DmiInfo,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    Display,
    EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum BootMode {
    Uefi,
    Bios,
    #[default]
    Unknown,
}

/// Firmware-reported identity. Most fields need root; missing ones are `None`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct DmiInfo {
    pub system_vendor: Option<String>,
    pub product_name: Option<String>,
    pub product_serial: Option<String>,
    pub product_uuid: Option<String>,
    pub board_vendor: Option<String>,
    pub board_name: Option<String>,
    pub board_serial: Option<String>,
    pub bios_vendor: Option<String>,
    pub bios_version: Option<String>,
    pub bios_date: Option<String>,
    pub chassis_type: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CpuInfo {
    pub model: Option<String>,
    pub vendor: Option<String>,
    pub sockets: u32,
    pub physical_cores: u32,
    pub logical_cpus: u32,
    pub max_mhz: Option<u32>,
    /// `None` where the platform cannot tell.
    pub virtualization: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub swap_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiskInfo {
    /// Kernel name, e.g. `nvme0n1`.
    pub name: String,
    pub kind: DiskKind,
    pub size_bytes: u64,
    pub logical_block_size: Option<u32>,
    pub model: Option<String>,
    pub vendor: Option<String>,
    pub serial: Option<String>,
    pub removable: bool,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Display, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum DiskKind {
    Nvme,
    Ssd,
    Hdd,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NicInfo {
    pub name: String,
    pub mac: Option<String>,
    pub kind: NicKind,
    pub driver: Option<String>,
    /// `None` where the platform cannot tell.
    pub link_up: Option<bool>,
    pub speed_mbps: Option<u32>,
    pub mtu: Option<u32>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Display, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum NicKind {
    /// Backed by a bus device (PCI, USB, ...).
    Physical,
    /// Bridges, veths, tunnels, bonds, ...
    Virtual,
    /// The platform cannot tell (non-Linux).
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PciDevice {
    /// e.g. `0000:00:02.0`
    pub address: String,
    pub vendor_id: String,
    pub device_id: String,
    /// 24-bit class code, e.g. `0x030000`.
    pub class: String,
    pub class_name: PciClass,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Display, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum PciClass {
    Storage,
    Network,
    Display,
    Bridge,
    Other,
}
