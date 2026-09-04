//! hwprobe — cross-platform hardware detection for local AI.
//!
//! One call, one serialisable struct:
//! ```no_run
//! let info = hwprobe::detect();
//! println!("{}", serde_json::to_string_pretty(&info).unwrap());
//! ```

mod gpu;
mod ram;
pub mod types;

pub use types::{GpuInfo, GpuState, GpuVendor, HardwareInfo};

pub fn detect() -> HardwareInfo {
    let (gpus, metal_max_working_set_mb) = gpu::probe_all();

    #[cfg(target_os = "macos")]
    let unified_memory = gpu::macos::is_apple_silicon();
    #[cfg(not(target_os = "macos"))]
    let unified_memory = false;

    HardwareInfo {
        ram_mb: ram::total_ram_mb(),
        ram_kind: ram::ram_kind(),
        gpus,
        unified_memory,
        metal_max_working_set_mb,
    }
}
