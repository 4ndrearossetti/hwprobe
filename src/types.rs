use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HardwareInfo {
    pub ram_mb: u64,
    /// "DDR4", "DDR5", "LPDDR5", ... None if undetectable (e.g. Linux without root)
    pub ram_kind: Option<String>,
    pub gpus: Vec<GpuInfo>,
    /// Apple Silicon: CPU and GPU share ram_mb
    pub unified_memory: bool,
    /// macOS only: MTLDevice.recommendedMaxWorkingSetSize in MB, the
    /// authoritative "usable by GPU" number. None elsewhere.
    pub metal_max_working_set_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub vendor: GpuVendor,
    pub model: String,
    /// None = shared/unknown
    pub vram_mb: Option<u64>,
    /// true = iGPU / dynamic shared memory, not dedicated VRAM
    pub shared: bool,
    pub state: GpuState,
    /// Highest dedicated VRAM among detected GPUs
    pub primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Other(u16), // PCI vendor id
}

impl GpuVendor {
    pub fn from_pci_id(id: u16) -> Self {
        match id {
            0x10de => Self::Nvidia,
            0x1002 => Self::Amd,
            0x8086 => Self::Intel,
            0x106b => Self::Apple,
            other => Self::Other(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum GpuState {
    Ok,
    /// Hardware present (PCI vendor id seen) but no usable driver API.
    /// NVIDIA: NVML failed to load => proprietary driver absent (incl. nouveau).
    DriverMissing,
}
