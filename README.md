# hwprobe

Cross-platform hardware detection: RAM, GPU(s), VRAM, unified memory 
and memory kind, on Windows, macOS and Linux.

```rust
let info = hwprobe::detect(); // -> HardwareInfo
```

or

```
hwprobe --json
```

Probe order per vendor: vendor API (NVML) --> OS/kernel interface
(DXGI / Metal / sysfs) --> generic PCI enumeration. Every probe fails
gracefully; unknown hardware degrades to vendor-id-only.

