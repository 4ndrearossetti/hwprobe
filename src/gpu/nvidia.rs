use crate::types::{GpuInfo, GpuState, GpuVendor};
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::Clock;

/// Probe 1 (highest authority, all OSes): NVML, dlopened from the driver.
/// Err/None here just means "no usable NVIDIA driver" — callers fall through.
pub fn probe() -> Option<Vec<GpuInfo>> {
    let nvml = Nvml::init().ok()?; // DriverNotLoaded / lib not found => None
    let count = nvml.device_count().ok()?;
    let mut gpus = Vec::new();
    for i in 0..count {
        let dev = nvml.device_by_index(i).ok()?;
        let mem = dev.memory_info().ok()?;

        // Effective bandwidth estimate: bus width x memory clock x 2 (DDR),
        // derated 0.85 for real-world efficiency. NVML's memory clock is
        // ambiguous across generations (command rate vs half) — validated
        // against measured decode on GDDR6X; revisit if field reports skew.
        let bandwidth_gb_s = match (dev.memory_bus_width(), dev.max_clock_info(Clock::Memory)) {
            (Ok(bits), Ok(mhz)) => {
                Some((bits as f64 / 8.0) * (mhz as f64) * 2.0 * 1e6 / 1e9 * 0.85)
            }
            _ => None,
        };

        gpus.push(GpuInfo {
            vendor: GpuVendor::Nvidia,
            model: dev.name().unwrap_or_else(|_| "NVIDIA GPU".into()),
            vram_mb: Some(mem.total / (1024 * 1024)),
            shared: false,
            state: GpuState::Ok,
            primary: false, // set by mod.rs
            bandwidth_gb_s,
        });
    }
    (!gpus.is_empty()).then_some(gpus)
}
