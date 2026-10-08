//! GPU discovery and sampling.
//!
//! Two backends, both dependency-free:
//! - **AMD**: amdgpu sysfs files (`gpu_busy_percent`, `mem_info_vram_*`).
//! - **NVIDIA**: NVML, dlopened from `libnvidia-ml.so.1` at startup —
//!   spawning `nvidia-smi` every refresh interval would cost ~25 ms of
//!   CPU per sample, which is exactly what we try to avoid.
//!
//! GPUs are discovered once in [`discover`] and sampled on every refresh
//! in [`sample`]. Each GPU keeps a bounded `(gpu%, vram%)` history for the
//! nvtop-style chart on the stats page.

use std::{
    ffi::{CStr, c_void},
    path::{Path, PathBuf},
    ptr::NonNull,
};

/// How many `(gpu%, vram%)` samples we keep — same depth as the other
/// histories on the stats page.
const HISTORY_LEN: usize = 60;

/// One physical GPU: current values plus a rolling history for the chart.
pub struct GpuInfo {
    pub name: String,
    /// Core utilization in percent (0-100).
    pub usage: f32,
    /// Video memory in bytes.
    pub mem_used: u64,
    pub mem_total: u64,
    /// `(gpu%, vram%)` samples, newest last, capped at [`HISTORY_LEN`].
    pub history: Vec<(f32, f32)>,
    backend: GpuBackend,
}

enum GpuBackend {
    /// amdgpu sysfs files, read directly every sample.
    AmD {
        busy: PathBuf,
        used: PathBuf,
        total: PathBuf,
    },
    /// Opaque NVML device handle plus the two entry points we need.
    Nvml {
        dev: NonNull<c_void>,
        util: unsafe extern "C" fn(NonNull<c_void>, *mut NvmlUtilization) -> i32,
        mem: unsafe extern "C" fn(NonNull<c_void>, *mut NvmlMemory) -> i32,
    },
}

// SAFETY: NVML device handles are plain opaque values and every NVML
// call happens while the App mutex is held, so the GpuInfo values can
// move freely between threads without data races.
unsafe impl Send for GpuBackend {}

#[repr(C)]
struct NvmlUtilization {
    gpu: u32,
    memory: u32,
}

/// NVML's `nvmlMemory_t` — note the declaration order is
/// `total, free, used` (confirmed against the NVML docs and empirically).
/// `used` includes the driver's reserved bookkeeping memory, which is
/// what nvtop shows as well.
#[repr(C)]
struct NvmlMemory {
    total: u64,
    free: u64,
    used: u64,
}

/// Discovers all usable GPUs. Called once at startup; a machine without
/// GPUs (or without the NVIDIA driver loaded) simply yields an empty vec.
pub fn discover() -> Vec<GpuInfo> {
    let mut gpus = Vec::new();
    discover_amdgpu(&mut gpus);
    discover_nvml(&mut gpus);
    gpus
}

/// Samples every GPU and appends to its history. Called once per refresh.
pub fn sample(gpus: &mut [GpuInfo]) {
    for gpu in gpus {
        gpu.sample();
    }
}

impl GpuInfo {
    fn sample(&mut self) {
        let mem_pct = match &self.backend {
            GpuBackend::AmD { busy, used, total } => {
                // A vanished sysfs file (GPU reset/unbind) reads as idle.
                self.usage = read_f32(busy).unwrap_or(0.0);
                self.mem_used = read_u64(used).unwrap_or(0);
                self.mem_total = read_u64(total).unwrap_or(self.mem_total);
                percent(self.mem_used, self.mem_total)
            }
            GpuBackend::Nvml { dev, util, mem } => {
                // SAFETY: handles and function pointers come from
                // `discover`, the library stays loaded for the process
                // lifetime, and the caller holds the App lock.
                unsafe {
                    let mut u = NvmlUtilization { gpu: 0, memory: 0 };
                    let mut m = NvmlMemory {
                        total: 0,
                        free: 0,
                        used: 0,
                    };
                    let util_ok = util(*dev, &mut u) == 0;
                    let mem_ok = mem(*dev, &mut m) == 0;
                    if util_ok {
                        self.usage = u.gpu as f32;
                    } else {
                        // e.g. the dGPU was powered off in the meantime.
                        self.usage = 0.0;
                    }
                    if mem_ok {
                        self.mem_used = m.used;
                        self.mem_total = m.total;
                    } else {
                        self.mem_used = 0;
                    }
                    percent(self.mem_used, self.mem_total)
                }
            }
        };

        self.history.push((self.usage, mem_pct));
        if self.history.len() > HISTORY_LEN {
            self.history.remove(0);
        }
    }
}

fn percent(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        used as f32 / total as f32 * 100.0
    }
}

fn read_f32(path: &Path) -> Option<f32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_u64(path: &Path) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

// ---------------------------------------------------------------- AMD

fn discover_amdgpu(gpus: &mut Vec<GpuInfo>) {
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return;
    };

    // `card0`, `card1`, ... but not connector entries like `card0-eDP-1`.
    let mut cards: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("card"))
                .is_some_and(|n| n.parse::<u32>().is_ok())
        })
        .collect();
    cards.sort();

    for card in cards {
        let device = card.join("device");
        let busy = device.join("gpu_busy_percent");
        // Only amdgpu exposes `gpu_busy_percent`; this also keeps the
        // NVIDIA card from being picked up by the sysfs backend.
        if !busy.exists() {
            continue;
        }

        let name = ["label", "product_name"]
            .iter()
            .find_map(|f| std::fs::read_to_string(device.join(f)).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "AMD GPU".to_string());

        gpus.push(GpuInfo {
            name,
            usage: 0.0,
            mem_used: 0,
            mem_total: 0,
            history: vec![(0.0, 0.0); HISTORY_LEN],
            backend: GpuBackend::AmD {
                busy,
                used: device.join("mem_info_vram_used"),
                total: device.join("mem_info_vram_total"),
            },
        });
    }
}

// -------------------------------------------------------------- NVIDIA

type NvmlInitFn = unsafe extern "C" fn() -> i32;
type NvmlCountFn = unsafe extern "C" fn(*mut u32) -> i32;
type NvmlHandleFn = unsafe extern "C" fn(u32, *mut *mut c_void) -> i32;
type NvmlNameFn = unsafe extern "C" fn(*mut c_void, *mut u8, u32) -> i32;
type NvmlUtilFn = unsafe extern "C" fn(NonNull<c_void>, *mut NvmlUtilization) -> i32;
type NvmlMemFn = unsafe extern "C" fn(NonNull<c_void>, *mut NvmlMemory) -> i32;

fn discover_nvml(gpus: &mut Vec<GpuInfo>) {
    // SAFETY: dlopen of a well-known SONAME; the handle is intentionally
    // kept for the whole process lifetime (dlsym results and device
    // handles stay valid for as long as the library is loaded).
    unsafe {
        let lib = libc::dlopen(c"libnvidia-ml.so.1".as_ptr(), libc::RTLD_LAZY);
        if lib.is_null() {
            return;
        }

        let init: Option<NvmlInitFn> = sym(lib, c"nvmlInit_v2");
        let count: Option<NvmlCountFn> = sym(lib, c"nvmlDeviceGetCount_v2");
        let handle: Option<NvmlHandleFn> = sym(lib, c"nvmlDeviceGetHandleByIndex_v2");
        let name: Option<NvmlNameFn> = sym(lib, c"nvmlDeviceGetName");
        let util: Option<NvmlUtilFn> = sym(lib, c"nvmlDeviceGetUtilizationRates");
        let mem: Option<NvmlMemFn> = sym(lib, c"nvmlDeviceGetMemoryInfo");

        // All or nothing: a partial set of symbols would only fail later.
        let (Some(init), Some(count), Some(handle), Some(name), Some(util), Some(mem)) =
            (init, count, handle, name, util, mem)
        else {
            return;
        };

        if init() != 0 {
            return;
        }

        let mut device_count = 0u32;
        if count(&mut device_count) != 0 {
            return;
        }

        for index in 0..device_count {
            let mut raw = std::ptr::null_mut();
            if handle(index, &mut raw) != 0 || raw.is_null() {
                continue;
            }
            let dev = match NonNull::new(raw) {
                Some(d) => d,
                None => continue,
            };

            let mut buf = [0u8; 96];
            let gpu_name = if name(dev.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) == 0 {
                let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                String::from_utf8_lossy(&buf[..len]).into_owned()
            } else {
                format!("NVIDIA GPU {index}")
            };

            gpus.push(GpuInfo {
                name: gpu_name,
                usage: 0.0,
                mem_used: 0,
                mem_total: 0,
                history: vec![(0.0, 0.0); HISTORY_LEN],
                backend: GpuBackend::Nvml { dev, util, mem },
            });
        }
    }
}

/// Resolves a C symbol to a typed function pointer.
///
/// # Safety
/// `lib` must be a valid handle from `dlopen` that outlives all uses of
/// the returned pointer; `name` must be a NUL-terminated symbol name of
/// the same ABI as `T`.
unsafe fn sym<T: Copy>(lib: *mut c_void, name: &CStr) -> Option<T> {
    // SAFETY: forwarded from the caller — `lib` is a live dlopen handle.
    let ptr = unsafe { libc::dlsym(lib, name.as_ptr()) };
    if ptr.is_null() {
        return None;
    }
    // SAFETY: function pointers and `*mut c_void` have the same size, so
    // this is a plain bit copy of an address.
    Some(unsafe { std::mem::transmute_copy(&ptr) })
}
