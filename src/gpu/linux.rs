#![cfg(target_os = "linux")]
use crate::types::{GpuInfo, GpuState, GpuVendor};
use std::fs;
use std::path::Path;

/// Probes 2+3: sysfs. amdgpu/i915/xe expose mem_info_vram_total; anything
/// else falls back to PCI vendor id with vram_mb=None. NVIDIA seen here
/// without NVML having succeeded => DriverMissing.
pub fn probe(nvml_succeeded: bool) -> Vec<GpuInfo> {
    probe_at(Path::new("/sys/class/drm"), nvml_succeeded)
}

fn probe_at(root: &Path, nvml_succeeded: bool) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
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
        let shared = vram_mb.is_none_or(|v| v <= 1024);
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

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn fake_card(root: &Path, name: &str, vendor: &str, vram_bytes: Option<u64>) {
        let dev = root.join(name).join("device");
        fs::create_dir_all(&dev).unwrap();
        fs::write(dev.join("vendor"), format!("{vendor}\n")).unwrap();
        fs::write(dev.join("device"), "0x1234\n").unwrap();
        if let Some(v) = vram_bytes {
            fs::write(dev.join("mem_info_vram_total"), format!("{v}\n")).unwrap();
        }
    }

    #[test]
    fn amdgpu_reports_vram() {
        let tmp = tempfile::tempdir().unwrap();
        fake_card(tmp.path(), "card0", "0x1002", Some(16 * GIB));
        let gpus = probe_at(tmp.path(), false);
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].vendor, GpuVendor::Amd);
        assert_eq!(gpus[0].vram_mb, Some(16384));
        assert!(!gpus[0].shared);
        assert_eq!(gpus[0].state, GpuState::Ok);
    }

    #[test]
    fn old_radeon_without_vram_file_is_shared_unknown() {
        let tmp = tempfile::tempdir().unwrap();
        fake_card(tmp.path(), "card0", "0x1002", None);
        let gpus = probe_at(tmp.path(), false);
        assert_eq!(gpus[0].vram_mb, None);
        assert!(gpus[0].shared);
    }

    #[test]
    fn nvidia_without_nvml_is_driver_missing() {
        let tmp = tempfile::tempdir().unwrap();
        fake_card(tmp.path(), "card0", "0x10de", None);
        let gpus = probe_at(tmp.path(), false);
        assert_eq!(gpus[0].vendor, GpuVendor::Nvidia);
        assert_eq!(gpus[0].state, GpuState::DriverMissing);
    }

    #[test]
    fn nvidia_with_nvml_is_skipped_hybrid_keeps_igpu() {
        let tmp = tempfile::tempdir().unwrap();
        fake_card(tmp.path(), "card0", "0x10de", None); // dGPU, NVML reported it
        fake_card(tmp.path(), "card1", "0x8086", None); // Intel iGPU
        let gpus = probe_at(tmp.path(), true);
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].vendor, GpuVendor::Intel);
        assert!(gpus[0].shared);
    }

    #[test]
    fn connector_entries_are_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        fake_card(tmp.path(), "card0", "0x1002", Some(8 * GIB));
        fake_card(tmp.path(), "card0-HDMI-A-1", "0x1002", None);
        let gpus = probe_at(tmp.path(), false);
        assert_eq!(gpus.len(), 1);
    }

    #[test]
    fn garbage_vendor_file_does_not_panic() {
        let tmp = tempfile::tempdir().unwrap();
        fake_card(tmp.path(), "card0", "not-hex-at-all", None);
        let gpus = probe_at(tmp.path(), false);
        assert!(gpus.is_empty());
    }

    #[test]
    fn intel_igpu_small_carveout_is_shared() {
        let tmp = tempfile::tempdir().unwrap();
        fake_card(tmp.path(), "card0", "0x8086", Some(512 * 1024 * 1024));
        let gpus = probe_at(tmp.path(), false);
        assert_eq!(gpus[0].vram_mb, Some(512));
        assert!(gpus[0].shared);
    }
}
