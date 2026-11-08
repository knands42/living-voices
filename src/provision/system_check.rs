enum GraphicsCard {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}
pub struct Gpu {
    pub card: GraphicsCard,
    pub vram_mb: Option<u64>,
}

impl Gpu {
    #[cfg(target_os = "linux")]
    fn detect() -> Gpu {
        todo!()
    }

    #[cfg(target_os = "windows")]
    fn detect() -> Gpu {
        todo!()
    }
}
