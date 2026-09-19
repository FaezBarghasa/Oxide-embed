use candle_core::Device;
use tracing::{info, warn};

pub fn select_device(preference: &str) -> Device {
    let pref = preference.trim().to_lowercase();
    match pref.as_str() {
        "auto" => {
            #[cfg(feature = "cuda")]
            {
                if let Ok(dev) = Device::new_cuda(0) {
                    info!("Hardware acceleration: NVIDIA CUDA GPU 0 active");
                    return dev;
                }
            }
            #[cfg(feature = "metal")]
            {
                if let Ok(dev) = Device::new_metal(0) {
                    info!("Hardware acceleration: Apple Metal GPU active");
                    return dev;
                }
            }
            info!("Hardware acceleration: CPU compute device active");
            Device::Cpu
        }
        "cuda" | "gpu" => {
            #[cfg(feature = "cuda")]
            {
                match Device::new_cuda(0) {
                    Ok(dev) => {
                        info!("Hardware acceleration: NVIDIA CUDA GPU 0 active");
                        dev
                    }
                    Err(err) => {
                        warn!("Failed to initialize CUDA device: {err}. Falling back to CPU.");
                        Device::Cpu
                    }
                }
            }
            #[cfg(not(feature = "cuda"))]
            {
                warn!("CUDA requested but crate was compiled without 'cuda' feature. Falling back to CPU.");
                Device::Cpu
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
                        warn!("Failed to initialize CUDA device {idx}: {err}. Falling back to CPU.");
                        Device::Cpu
                    }
                }
            }
            #[cfg(not(feature = "cuda"))]
            {
                warn!("CUDA requested but crate was compiled without 'cuda' feature. Falling back to CPU.");
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
                warn!("Metal requested but crate was compiled without 'metal' feature. Falling back to CPU.");
                Device::Cpu
            }
        }
        "rocm" => {
            warn!("ROCm compute requested for Candle: Candle utilizes hipBLAS via CUDA compatibility; selecting GPU 0.");
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
