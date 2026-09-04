use crate::types::{GpuInfo, GpuState, GpuVendor};
use nvml_wrapper::Nvml;

/// Probe 1 (highest authority, all OSes): NVML, dlopened from the driver.
/// Err/None here just means "no usable NVIDIA driver" — callers fall through.
pub fn probe() -> Option<Vec<GpuInfo>> {
    let nvml = Nvml::init().ok()?; // DriverNotLoaded / lib not found => None
    let count = nvml.device_count().ok()?;
    let mut gpus = Vec::new();
    for i in 0..count {
        let dev = nvml.device_by_index(i).ok()?;
        let mem = dev.memory_info().ok()?;
        gpus.push(GpuInfo {
            vendor: GpuVendor::Nvidia,
            model: dev.name().unwrap_or_else(|_| "NVIDIA GPU".into()),
            vram_mb: Some(mem.total / (1024 * 1024)),
            shared: false,
            state: GpuState::Ok,
            primary: false, // set by mod.rs
        });
    }
    (!gpus.is_empty()).then_some(gpus)
}
