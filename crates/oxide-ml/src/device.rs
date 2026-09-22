use candle_core::Device;
use tracing::{info, warn};

pub fn select_device(preference: &str) -> Device {
    let pref = preference.trim().to_lowercase();
    match pref.as_str() {
        "auto" => {
            #[cfg(feature = "cuda")]
            {
                match Device::new_cuda(0) {
                    Ok(dev) => {
                        println!("🚀 Hardware acceleration active: NVIDIA CUDA GPU 0 (Tensor Cores)");
                        return dev;
                    }
                    Err(err) => {
                        warn!("CUDA runtime detected but GPU init failed: {err}");
                    }
                }
            }
            #[cfg(feature = "metal")]
            {
                if let Ok(dev) = Device::new_metal(0) {
                    println!("🚀 Hardware acceleration active: Apple Metal GPU");
                    return dev;
                }
            }
            println!("⚡ Compute device: CPU (SIMD AVX2/NEON)");
            Device::Cpu
        }
        "cuda" | "gpu" => {
            #[cfg(feature = "cuda")]
            {
                match Device::new_cuda(0) {
                    Ok(dev) => {
                        println!("🚀 Hardware acceleration active: NVIDIA CUDA GPU 0");
                        dev
                    }
                    Err(err) => {
                        panic!("CUDA acceleration requested but GPU initialization failed: {err}");
                    }
                }
            }
            #[cfg(not(feature = "cuda"))]
            {
                panic!("CUDA acceleration requested but oxide-embed was compiled without '--features cuda'");
            }
        }
        s if s.starts_with("cuda:") => {
            #[cfg(feature = "cuda")]
            {
                let idx: usize = s.trim_start_matches("cuda:").parse().unwrap_or(0);
                match Device::new_cuda(idx) {
                    Ok(dev) => {
                        info!("Hardware acceleration: NVIDIA CUDA GPU {idx} active");
                        dev
                    }
                    Err(err) => {
                        warn!(
                            "Failed to initialize CUDA device {idx}: {err}. Falling back to CPU."
                        );
                        Device::Cpu
                    }
                }
            }
            #[cfg(not(feature = "cuda"))]
            {
                warn!(
                    "CUDA requested but crate was compiled without 'cuda' feature. Falling back to CPU."
                );
                Device::Cpu
            }
        }
        "metal" => {
            #[cfg(feature = "metal")]
            {
                match Device::new_metal(0) {
                    Ok(dev) => {
                        info!("Hardware acceleration: Apple Metal GPU active");
                        dev
                    }
                    Err(err) => {
                        warn!("Failed to initialize Metal device: {err}. Falling back to CPU.");
                        Device::Cpu
                    }
                }
            }
            #[cfg(not(feature = "metal"))]
            {
                warn!(
                    "Metal requested but crate was compiled without 'metal' feature. Falling back to CPU."
                );
                Device::Cpu
            }
        }
        "rocm" => {
            warn!(
                "ROCm compute requested for Candle: Candle utilizes hipBLAS via CUDA compatibility; selecting GPU 0."
            );
            #[cfg(feature = "cuda")]
            {
                Device::new_cuda(0).unwrap_or(Device::Cpu)
            }
            #[cfg(not(feature = "cuda"))]
            {
                Device::Cpu
            }
        }
        "cpu" => {
            info!("Compute device: CPU");
            Device::Cpu
        }
        other => {
            warn!("Unrecognized device preference '{other}', defaulting to CPU");
            Device::Cpu
        }
    }
}

pub fn device_name(device: &Device) -> &'static str {
    match device {
        Device::Cpu => "cpu",
        Device::Cuda(_) => "cuda",
        Device::Metal(_) => "metal",
    }
}

/// Computes the optimal batch size automatically based on target compute device.
/// - CUDA: 64 (maximizes tensor core occupancy on desktop/mobile GPUs without OOM)
/// - Metal: 32 (optimized for Apple Unified Memory bandwidth)
/// - CPU: Derived from available logical cores (clamped between 8 and 32)
pub fn optimal_batch_size(device: &Device) -> usize {
    match device {
        Device::Cuda(_) => 64,
        Device::Metal(_) => 32,
        Device::Cpu => {
            let cores = std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(4);
            (cores * 2).clamp(8, 32)
        }
    }
}
