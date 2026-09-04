#![cfg(target_os = "windows")]
use crate::types::{GpuInfo, GpuState, GpuVendor};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};

/// Probe 2 on Windows: DXGI adapter enumeration. All vendors,
/// DedicatedVideoMemory is reliable (unlike WMI's 4 GB-capped AdapterRAM).
/// NVIDIA adapters seen here without NVML => DriverMissing.
pub fn probe(nvml_succeeded: bool) -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    let factory: IDXGIFactory1 = match unsafe { CreateDXGIFactory1() } {
        Ok(f) => f,
        Err(_) => return gpus,
    };
    let mut i = 0u32;
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(i) } {
        i += 1;
        let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
            continue;
        };
        // Skip "Microsoft Basic Render Driver" (software)
        if desc.VendorId == 0x1414 {
            continue;
        }
        let vendor = GpuVendor::from_pci_id(desc.VendorId as u16);
        if vendor == GpuVendor::Nvidia && nvml_succeeded {
            continue;
        }
        let dedicated_mb = (desc.DedicatedVideoMemory / (1024 * 1024)) as u64;
        let model = String::from_utf16_lossy(&desc.Description)
            .trim_end_matches('\0')
            .to_string();
        gpus.push(GpuInfo {
            vendor,
            model,
            vram_mb: (dedicated_mb > 0).then_some(dedicated_mb),
            shared: dedicated_mb <= 1024, // iGPUs carve <=1 GB, rest is shared
            state: if vendor == GpuVendor::Nvidia {
                GpuState::DriverMissing
            } else {
                GpuState::Ok
            },
            primary: false,
        });
    }
    gpus
}
