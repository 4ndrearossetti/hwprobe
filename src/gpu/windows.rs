#![cfg(target_os = "windows")]
use crate::types::GpuInfo;
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
        let model = String::from_utf16_lossy(&desc.Description)
            .trim_end_matches('\0')
            .to_string();
        let dedicated_mb = (desc.DedicatedVideoMemory / (1024 * 1024)) as u64;
        if let Some(g) = crate::gpu::heuristics::dxgi_gpu(
            desc.VendorId as u16,
            dedicated_mb,
            model,
            nvml_succeeded,
        ) {
            gpus.push(g);
        }
    }
    gpus
}
