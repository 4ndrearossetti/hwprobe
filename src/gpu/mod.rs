// Callers are cfg'd per-OS, so on every platform some of these functions
// are "unused" outside tests — that's by design.
#[allow(dead_code)]
pub(crate) mod heuristics;

mod nvidia;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "windows")]
mod windows;

use crate::types::GpuInfo;

/// Vendor API first (NVML), then OS interface, then generic enumeration.
/// Returns (gpus, metal_max_working_set_mb).
pub fn probe_all() -> (Vec<GpuInfo>, Option<u64>) {
    let nvml_gpus = nvidia::probe();
    let nvml_ok = nvml_gpus.is_some();
    let mut gpus = nvml_gpus.unwrap_or_default();
    #[allow(unused_mut)]
    let mut metal_ws = None;

    #[cfg(target_os = "linux")]
    gpus.extend(linux::probe(nvml_ok));

    #[cfg(target_os = "windows")]
    gpus.extend(windows::probe(nvml_ok));

    #[cfg(target_os = "macos")]
    {
        let _ = nvml_ok; // no NVIDIA on modern macOS
        if macos::is_apple_silicon() {
            let (g, ws) = macos::probe_apple_silicon();
            gpus.extend(g);
            metal_ws = ws;
        } else {
            gpus.extend(macos::probe_intel_mac());
        }
    }

    mark_primary(&mut gpus);
    (gpus, metal_ws)
}

/// Primary = highest dedicated VRAM; ties/no-dedicated => first non-shared,
/// else first GPU.
fn mark_primary(gpus: &mut [GpuInfo]) {
    if gpus.is_empty() {
        return;
    }
    let idx = gpus
        .iter()
        .enumerate()
        .max_by_key(|(_, g)| (!g.shared, g.vram_mb.unwrap_or(0)))
        .map(|(i, _)| i)
        .unwrap_or(0);
    gpus[idx].primary = true;
}
