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

/// Intel Macs: system_profiler. Rare and shrinking population; VRAM string
/// parsing is best-effort.
pub fn probe_intel_mac() -> Vec<GpuInfo> {
    let Some(out) = Command::new("system_profiler")
        .args(["SPDisplaysDataType", "-json"])
        .output()
        .ok()
    else {
        return vec![];
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return vec![];
    };
    let Some(cards) = v.get("SPDisplaysDataType").and_then(|c| c.as_array()) else {
        return vec![];
    };
    cards
        .iter()
        .map(|c| {
            let model = c
                .get("sppci_model")
                .and_then(|m| m.as_str())
                .unwrap_or("GPU")
                .to_string();
            // "spdisplays_vram": "8 GB" / "1536 MB"
            let vram_mb = c
                .get("spdisplays_vram")
                .and_then(|s| s.as_str())
                .and_then(parse_vram_mb);
            let vendor = if model.contains("Intel") {
                GpuVendor::Intel
            } else if model.contains("AMD") || model.contains("Radeon") {
                GpuVendor::Amd
            } else if model.contains("NVIDIA") || model.contains("GeForce") {
                GpuVendor::Nvidia
            } else {
                GpuVendor::Other(0)
            };
            GpuInfo {
                shared: vram_mb.map_or(true, |v| v <= 1536),
                vendor,
                model,
                vram_mb,
                state: GpuState::Ok,
                primary: false,
            }
        })
        .collect()
}

fn parse_vram_mb(s: &str) -> Option<u64> {
    let mut parts = s.split_whitespace();
    let n: u64 = parts.next()?.parse().ok()?;
    match parts.next()? {
        "GB" => Some(n * 1024),
        "MB" => Some(n),
        _ => None,
    }
}
