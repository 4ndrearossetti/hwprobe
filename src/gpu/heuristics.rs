//! Platform-neutral interpretation logic, split out of the OS probes so it
//! can be tested on any machine. The probes fetch; this file decides.

use crate::types::{GpuInfo, GpuState, GpuVendor};

/// "8 GB" / "1536 MB" strings from system_profiler.
pub fn parse_vram_mb(s: &str) -> Option<u64> {
    let mut parts = s.split_whitespace();
    let n: u64 = parts.next()?.parse().ok()?;
    match parts.next()? {
        "GB" => Some(n * 1024),
        "MB" => Some(n),
        _ => None,
    }
}

/// Interpret one entry of SPDisplaysDataType (Intel Macs).
pub fn intel_mac_gpu(model: &str, vram_mb: Option<u64>) -> GpuInfo {
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
        // Intel-Mac iGPUs carve out <=1536 MB dynamic VRAM
        shared: vram_mb.is_none_or(|v| v <= 1536),
        vendor,
        model: model.to_string(),
        vram_mb,
        state: GpuState::Ok,
        primary: false,
        bandwidth_gb_s: None,
    }
}

/// Parse full SPDisplaysDataType -json output (Intel Macs).
pub fn parse_intel_mac_json(json: &str) -> Vec<GpuInfo> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
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
                .unwrap_or("GPU");
            let vram_mb = c
                .get("spdisplays_vram")
                .and_then(|s| s.as_str())
                .and_then(parse_vram_mb);
            intel_mac_gpu(model, vram_mb)
        })
        .collect()
}

/// Interpret one DXGI adapter descriptor (Windows). None = skip this adapter.
pub fn dxgi_gpu(
    vendor_id: u16,
    dedicated_mb: u64,
    model: String,
    nvml_succeeded: bool,
) -> Option<GpuInfo> {
    // Microsoft Basic Render Driver (software rasteriser)
    if vendor_id == 0x1414 {
        return None;
    }
    let vendor = GpuVendor::from_pci_id(vendor_id);
    if vendor == GpuVendor::Nvidia && nvml_succeeded {
        return None; // NVML already reported it with exact numbers
    }
    Some(GpuInfo {
        vendor,
        model,
        vram_mb: (dedicated_mb > 0).then_some(dedicated_mb),
        shared: dedicated_mb <= 1024, // iGPUs carve <=1 GB dedicated
        state: if vendor == GpuVendor::Nvidia {
            GpuState::DriverMissing
        } else {
            GpuState::Ok
        },
        primary: false,
        bandwidth_gb_s: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vram_strings() {
        assert_eq!(parse_vram_mb("8 GB"), Some(8192));
        assert_eq!(parse_vram_mb("1536 MB"), Some(1536));
        assert_eq!(parse_vram_mb("garbage"), None);
        assert_eq!(parse_vram_mb(""), None);
    }

    #[test]
    fn intel_mac_igpu_and_dgpu() {
        let igpu = intel_mac_gpu("Intel Iris Plus Graphics", Some(1536));
        assert_eq!(igpu.vendor, GpuVendor::Intel);
        assert!(igpu.shared);

        let dgpu = intel_mac_gpu("Radeon Pro 5500M", Some(8192));
        assert_eq!(dgpu.vendor, GpuVendor::Amd);
        assert!(!dgpu.shared);
    }

    #[test]
    fn intel_mac_json_dual_gpu() {
        // Shape of a 16" MBP 2019: iGPU + Radeon dGPU
        let json = r#"{
          "SPDisplaysDataType": [
            { "sppci_model": "Intel UHD Graphics 630", "spdisplays_vram": "1536 MB" },
            { "sppci_model": "AMD Radeon Pro 5500M", "spdisplays_vram": "8 GB" }
          ]
        }"#;
        let gpus = parse_intel_mac_json(json);
        assert_eq!(gpus.len(), 2);
        assert!(gpus[0].shared);
        assert_eq!(gpus[1].vram_mb, Some(8192));
        assert!(!gpus[1].shared);
    }

    #[test]
    fn intel_mac_json_malformed() {
        assert!(parse_intel_mac_json("not json").is_empty());
        assert!(parse_intel_mac_json("{}").is_empty());
    }

    #[test]
    fn dxgi_skips_software_and_reported_nvidia() {
        assert!(dxgi_gpu(0x1414, 0, "Microsoft Basic Render Driver".into(), false).is_none());
        assert!(dxgi_gpu(0x10de, 8192, "RTX 3070".into(), true).is_none());
    }

    #[test]
    fn dxgi_nvidia_without_nvml_is_driver_missing() {
        let g = dxgi_gpu(0x10de, 8192, "RTX 3070".into(), false).unwrap();
        assert_eq!(g.state, GpuState::DriverMissing);
    }

    #[test]
    fn dxgi_amd_and_igpu() {
        let amd = dxgi_gpu(0x1002, 16384, "RX 7800 XT".into(), false).unwrap();
        assert_eq!(amd.vendor, GpuVendor::Amd);
        assert!(!amd.shared);

        let igpu = dxgi_gpu(0x8086, 128, "Intel Iris Xe".into(), false).unwrap();
        assert!(igpu.shared);
    }
}
