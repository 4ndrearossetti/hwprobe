#![cfg(target_os = "linux")]
use crate::types::{GpuInfo, GpuState, GpuVendor};
use std::fs;
use std::path::Path;

/// Probes 2+3: sysfs. amdgpu/i915/xe expose mem_info_vram_total; anything
/// else falls back to PCI vendor id with vram_mb=None. NVIDIA seen here
/// without NVML having succeeded => DriverMissing.
pub fn probe(nvml_succeeded: bool) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return gpus;
    };
    for e in entries.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy().into_owned();
        // card0, card1... skip connectors like card0-HDMI-A-1
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let dev = e.path().join("device");
        let Some(vendor_id) = read_hex_u16(&dev.join("vendor")) else {
            continue;
        };
        let vendor = GpuVendor::from_pci_id(vendor_id);

        if vendor == GpuVendor::Nvidia {
            if nvml_succeeded {
                continue; // already reported with exact numbers
            }
            gpus.push(GpuInfo {
                vendor,
                model: pci_device_hint(&dev),
                vram_mb: None,
                shared: false,
                state: GpuState::DriverMissing, // hardware present, NVML absent
                primary: false,
            });
            continue;
        }

        let vram_mb = fs::read_to_string(dev.join("mem_info_vram_total"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|b| b / (1024 * 1024));
        // Heuristic: iGPUs (Intel non-Arc, AMD APU) report small/absent VRAM
        let shared = vram_mb.map_or(true, |v| v <= 1024);
        gpus.push(GpuInfo {
            vendor,
            model: pci_device_hint(&dev),
            vram_mb,
            shared,
            state: GpuState::Ok,
            primary: false,
        });
    }
    gpus
}

fn read_hex_u16(p: &Path) -> Option<u16> {
    let s = fs::read_to_string(p).ok()?;
    u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

/// Without a pci.ids database, report "vendor 0xVVVV device 0xDDDD".
/// Good enough for a hint; a pci.ids lookup is a later nicety.
fn pci_device_hint(dev: &Path) -> String {
    let v = fs::read_to_string(dev.join("vendor")).unwrap_or_default();
    let d = fs::read_to_string(dev.join("device")).unwrap_or_default();
    format!("PCI {} {}", v.trim(), d.trim())
}
