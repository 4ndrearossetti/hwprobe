#![cfg(target_os = "macos")]
use crate::types::{GpuInfo, GpuState, GpuVendor};
use std::process::Command;

pub fn is_apple_silicon() -> bool {
    Command::new("sysctl")
        .args(["-n", "machdep.cpu.brand_string"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("Apple"))
        .unwrap_or(false)
}

/// Apple Silicon: one GPU, unified memory. Metal's
/// recommendedMaxWorkingSetSize is the authoritative usable number.
pub fn probe_apple_silicon() -> (Vec<GpuInfo>, Option<u64>) {
    let (model, max_ws_mb) = metal_device_info().unwrap_or((None, None));
    let gpu = GpuInfo {
        vendor: GpuVendor::Apple,
        model: model.unwrap_or_else(|| "Apple GPU".into()),
        vram_mb: None, // unified: HardwareInfo.ram_mb is the pool
        shared: true,
        state: GpuState::Ok,
        primary: true,
    };
    (vec![gpu], max_ws_mb)
}

fn metal_device_info() -> Option<(Option<String>, Option<u64>)> {
    use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice};
    let device = unsafe { MTLCreateSystemDefaultDevice() };
    let device = unsafe { device.as_ref() }?;
    let name = Some(device.name().to_string());
    let ws_mb = Some(device.recommendedMaxWorkingSetSize() / (1024 * 1024));
    Some((name, ws_mb))
}

/// Intel Macs: system_profiler. Rare and shrinking population.
pub fn probe_intel_mac() -> Vec<GpuInfo> {
    let Some(out) = Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-json"])
        .output()
        .ok()
    else {
        return vec![];
    };
    crate::gpu::heuristics::parse_intel_mac_json(&String::from_utf8_lossy(&out.stdout))
}
