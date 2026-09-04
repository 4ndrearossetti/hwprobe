# hwprobe

[![crates.io](https://img.shields.io/crates/v/hwprobe.svg)](https://crates.io/crates/hwprobe)
[![docs.rs](https://docs.rs/hwprobe/badge.svg)](https://docs.rs/hwprobe)
[![CI](https://github.com/4ndrearossetti/hwprobe/actions/workflows/ci.yml/badge.svg)](https://github.com/4ndrearossetti/hwprobe/actions)

Cross-platform hardware detection for local AI tooling: RAM, GPU(s), VRAM,
unified memory and memory kind, on Windows, macOS and Linux.

Built for the question every local-LLM launcher, installer and benchmark
tool has to answer first: *what is this machine actually capable of running?*

## Install

As a library:

```
cargo add hwprobe
```

As a CLI:

```
cargo install hwprobe
```

## Usage

As a library:

```rust
let info = hwprobe::detect(); // -> HardwareInfo, serde-serialisable
```

As a CLI, from any language or script:

```
$ hwprobe --json
{
  "ram_mb": 31775,
  "ram_kind": null,
  "gpus": [
    {
      "vendor": "Nvidia",
      "model": "NVIDIA GeForce RTX 3070 Ti Laptop GPU",
      "vram_mb": 8192,
      "shared": false,
      "state": "Ok",
      "primary": true
    },
    {
      "vendor": "Intel",
      "model": "Intel Corporation Alder Lake-P GT2 [Iris Xe Graphics]",
      "vram_mb": null,
      "shared": true,
      "state": "Ok",
      "primary": false
    }
  ],
  "unified_memory": false,
  "metal_max_working_set_mb": null
}
```

Field notes:

- `vram_mb: null` + `shared: true` — an iGPU borrowing system RAM; there is
  no dedicated VRAM to report.
- `unified_memory: true` (Apple Silicon) — CPU and GPU share `ram_mb`;
  `metal_max_working_set_mb` is Metal's own figure for how much of it the
  GPU may use, the number that matters for model sizing.
- `state: "DriverMissing"` — the hardware is present (PCI vendor id seen)
  but no usable driver API is loaded. Currently reported for NVIDIA GPUs
  without the proprietary driver (including nouveau).
- `primary` — the GPU with the most dedicated VRAM. Multi-GPU rigs get one
  entry per device; whether to aggregate VRAM across them (e.g. for
  tensor-split inference) is the consumer's decision.

## How it works

Per vendor, probes run in order of authority, first success wins:

1. **Vendor API** — NVML (dlopened from the NVIDIA driver): exact VRAM.
2. **OS / kernel interface** — DXGI on Windows, Metal + `system_profiler`
   on macOS, `/sys/class/drm` on Linux.
3. **Generic PCI enumeration** — vendor and device ids, resolved to
   readable names via an embedded `pci.ids` database; no memory figures.

Every probe fails gracefully. Unknown or undriven hardware degrades to
partial output (id-only names, `null` fields) — never a panic. `ram_kind`
is `null` on Linux without root by design: the DMI table needs privileges
and a detection tool should never ask for them.

## Platform support

| Platform | GPU detection | Notes |
|---|---|---|
| Windows 10+ | NVML → DXGI | all vendors; iGPUs marked `shared` |
| macOS 11+ (Apple Silicon) | Metal | unified memory + working-set size |
| macOS 11+ (Intel) | `system_profiler` | iGPU/dGPU incl. AMD |
| Linux | NVML → sysfs (`amdgpu`, `i915`/`xe`) → PCI ids | `DriverMissing` detection for NVIDIA |

CI builds and tests on all three (macOS runners are Apple Silicon, so that
path is exercised on real hardware). Decision logic is platform-neutral and
unit-tested everywhere; vendor edge cases (AMD, legacy radeon, hybrid
laptops, malformed sysfs) are covered by fixture tests.

## Reporting hardware

Wrong or missing output on your machine is exactly the feedback this tool
needs. Open an issue with:

1. `hwprobe --json` output
2. what you expected (GPU model, VRAM as you know it)
3. OS and driver version

Each report becomes a fixture test.

## Out of scope

- Recommending models, computing "usable memory", driver install advice —
  those are opinions, and belong in tools built on top of hwprobe.
- Benchmarking, thermals, clock speeds, monitoring.
- Root/admin escalation of any kind.

## License

MIT

