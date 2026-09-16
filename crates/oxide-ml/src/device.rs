use candle_core::Device;

pub fn select_device(preference: &str) -> Device {
    match preference {
        #[cfg(feature = "cuda")]
        "cuda" | "gpu" => Device::new_cuda(0).unwrap_or(Device::Cpu),
        #[cfg(feature = "metal")]
        "metal" => Device::new_metal(0).unwrap_or(Device::Cpu),
        _ => Device::Cpu,
    }
}
