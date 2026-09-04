# hwprobe

Cross-platform hardware detection for local AI: RAM, GPU(s), VRAM, unified
memory and memory kind, on Windows, macOS and Linux. No opinions, only
measurements — one call, one serialisable struct.

```rust
let info = hwprobe::detect(); // -> HardwareInfo
```

or

```
hwprobe --json
```

Probe order per vendor: vendor API (NVML) → OS/kernel interface
(DXGI / Metal / sysfs) → generic PCI enumeration. Every probe fails
gracefully; unknown hardware degrades to vendor-id-only, never a panic.

Status: scaffold. Linux + NVML paths written first; Windows/macOS compile
but need testing on real machines. See the reference map for the full
probe tree, gotchas and test matrix.

