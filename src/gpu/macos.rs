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

/// Apple publishes fixed unified-memory bandwidth per chip; no query API
/// exists, so a table off the Metal device name is the honest source.
/// Theoretical GB/s, derated x0.85 like everywhere else. Longest match
/// first ("M4 Pro" before "M4"). Unknown/future chips -> None; Max-tier
/// chips use the lower memory-config bin (conservative).
fn apple_bandwidth_gb_s(model: &str) -> Option<f64> {
    const TABLE: &[(&str, f64)] = &[
        ("M1 Ultra", 800.0),
        ("M1 Max", 400.0),
        ("M1 Pro", 200.0),
        ("M2 Ultra", 800.0),
        ("M2 Max", 400.0),
        ("M2 Pro", 200.0),
        ("M3 Ultra", 800.0),
        ("M3 Max", 300.0),
        ("M3 Pro", 150.0),
        ("M4 Max", 410.0),
        ("M4 Pro", 273.0),
        // bases last so "M4 Pro" never matches the "M4" row
        ("M1", 68.0),
        ("M2", 100.0),
        ("M3", 102.0),
        ("M4", 120.0),
        ("M5", 153.0),
    ];
    TABLE
        .iter()
        .find(|(name, _)| model.contains(name))
        .map(|(_, bw)| bw * 0.85)
}

/// Apple Silicon: one GPU, unified memory. Metal's
/// recommendedMaxWorkingSetSize is the authoritative usable number.
pub fn probe_apple_silicon() -> (Vec<GpuInfo>, Option<u64>) {
    let (model, max_ws_mb) = metal_device_info().unwrap_or((None, None));
    let model = model.unwrap_or_else(|| "Apple GPU".into());
    let bandwidth_gb_s = apple_bandwidth_gb_s(&model);
    let gpu = GpuInfo {
        vendor: GpuVendor::Apple,
        model,
        vram_mb: None, // unified: HardwareInfo.ram_mb is the pool
        shared: true,
        state: GpuState::Ok,
        primary: true,
        bandwidth_gb_s,
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
