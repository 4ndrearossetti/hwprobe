use sysinfo::System;

pub fn total_ram_mb() -> u64 {
    let mut sys = System::new();
    sys.refresh_memory();
    sys.total_memory() / (1024 * 1024)
}

/// DDR4/DDR5/LPDDR5... best-effort, per-OS. None is a legitimate answer.
pub fn ram_kind() -> Option<String> {
    imp::ram_kind()
}

#[cfg(target_os = "linux")]
mod imp {
    /// SMBIOS type-17 memory-device kind. Requires root (/sys/firmware/dmi).
    /// Never escalate: return None on EACCES.
    pub fn ram_kind() -> Option<String> {
        let entries = std::fs::read_dir("/sys/firmware/dmi/entries").ok()?;
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("17-") {
                continue;
            }
            let raw = std::fs::read(e.path().join("raw")).ok()?; // EACCES without root
            // SMBIOS type 17: memory type at offset 0x12
            if let Some(&code) = raw.get(0x12)
                && let Some(kind) = smbios_memory_type(code)
            {
                return Some(kind.to_string());
            }
        }
        None
    }

    fn smbios_memory_type(code: u8) -> Option<&'static str> {
        Some(match code {
            0x1a => "DDR4",
            0x1e => "LPDDR4",
            0x22 => "DDR5",
            0x23 => "LPDDR5",
            0x18 => "DDR3",
            0x1d => "LPDDR3",
            _ => return None,
        })
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use std::process::Command;

    /// SMBIOSMemoryType via CIM. Unprivileged.
    pub fn ram_kind() -> Option<String> {
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_PhysicalMemory | Select-Object -First 1).SMBIOSMemoryType",
            ])
            .output()
            .ok()?;
        let code: u8 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
        Some(
            match code {
                26 => "DDR4",
                30 => "LPDDR4",
                34 => "DDR5",
                35 => "LPDDR5",
                24 => "DDR3",
                29 => "LPDDR3",
                _ => return None,
            }
            .to_string(),
        )
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::process::Command;

    /// system_profiler SPMemoryDataType. Unprivileged.
    pub fn ram_kind() -> Option<String> {
        let out = Command::new("system_profiler")
            .args(["SPMemoryDataType", "-json"])
            .output()
            .ok()?;
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
        let items = v.get("SPMemoryDataType")?.as_array()?;
        // Apple Silicon: dimm_type at top level; Intel Macs: nested _items.
        // VMs report the literal string "unknown" — treat as undetected.
        for item in items {
            if let Some(t) = item.get("dimm_type").and_then(|t| t.as_str()) {
                return valid_kind(t);
            }
            if let Some(sub) = item.get("_items").and_then(|s| s.as_array()) {
                for dimm in sub {
                    if let Some(t) = dimm.get("dimm_type").and_then(|t| t.as_str()) {
                        return valid_kind(t);
                    }
                }
            }
        }
        None
    }

    fn valid_kind(t: &str) -> Option<String> {
        (!t.eq_ignore_ascii_case("unknown")).then(|| t.to_string())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod imp {
    pub fn ram_kind() -> Option<String> {
        None
    }
}
