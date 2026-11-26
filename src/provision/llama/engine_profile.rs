pub struct EngineProfile {
    pub ctx: u16,
    pub gpu_layers: &'static str,
    pub flash_attn: bool,
    pub kv_type: &'static str,
}

impl EngineProfile {
    /// Offload every layer that fits in VRAM
    pub fn chat() -> Self {
        Self {
            ctx: 8_192,
            gpu_layers: "auto",
            flash_attn: true,
            kv_type: "q8_0",
        }
    }

    pub fn memory() -> Self {
        Self {
            ctx: 8_192,
            gpu_layers: "2",
            flash_attn: true,
            kv_type: "q8_0",
        }
    }
}
