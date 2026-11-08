use super::layout;
use std::path::PathBuf;

// URLs mirror comfy-image.just: change them in lockstep. All GGUF/safetensors,
// all token-free.
pub const FLUX_Q4_URL: &str =
    "https://huggingface.co/city96/FLUX.1-dev-gguf/resolve/main/flux1-dev-Q4_K_S.gguf";
pub const FLUX_Q8_URL: &str =
    "https://huggingface.co/city96/FLUX.1-dev-gguf/resolve/main/flux1-dev-Q8_0.gguf";
pub const FLUX_T5_URL: &str = "https://huggingface.co/city96/t5-v1_1-xxl-encoder-gguf/resolve/main/t5-v1_1-xxl-encoder-Q5_K_M.gguf";
pub const FLUX_CLIP_URL: &str =
    "https://huggingface.co/comfyanonymous/flux_text_encoders/resolve/main/clip_l.safetensors";
pub const FLUX_VAE_URL: &str = "https://huggingface.co/Comfy-Org/Lumina_Image_2.0_Repackaged/resolve/main/split_files/vae/ae.safetensors";
pub const CHROMA_URL: &str = "https://huggingface.co/silveroxides/Chroma-GGUF/resolve/main/chroma-unlocked-v47-flash-heun/chroma-unlocked-v47-flash-heun-Q4_K_S.gguf";
pub const QWEN_URL: &str =
    "https://huggingface.co/city96/Qwen-Image-gguf/resolve/main/qwen-image-Q4_K_S.gguf";
pub const QWEN_VAE_URL: &str = "https://huggingface.co/Comfy-Org/Qwen-Image_ComfyUI/resolve/main/split_files/vae/qwen_image_vae.safetensors";
pub const QWEN_CLIP_URL: &str = "https://huggingface.co/Comfy-Org/Qwen-Image_ComfyUI/resolve/main/split_files/text_encoders/qwen_2.5_vl_7b_fp8_scaled.safetensors";

pub const GGUF_NODE_REPO: &str = "https://github.com/city96/ComfyUI-GGUF";
pub const GGUF_NODE_DIR: &str = "ComfyUI-GGUF";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    FluxQ4,
    FluxQ8,
    Chroma,
    Qwen,
}

impl Model {
    pub fn assets(self) -> Vec<Asset> {
        match self {
            Self::FluxQ4 => Asset::flux_assets(FLUX_Q4_URL),
            Self::FluxQ8 => Asset::flux_assets(FLUX_Q8_URL),
            Self::Chroma => Asset::flux_assets(CHROMA_URL),
            Self::Qwen => Asset::qwen_assets(),
        }
    }
}

/// One file a model needs, and where under `comfyui/models` it must land so the
/// loader nodes find it. `filename` overrides the URL's basename (only Flux's
/// VAE is renamed, to `flux_ae.safetensors`).
pub struct Asset {
    pub url: &'static str,
    pub subdir: &'static str,
    pub filename: Option<&'static str>,
}

impl Asset {
    pub const fn new(url: &'static str, subdir: &'static str) -> Self {
        Self {
            url,
            subdir,
            filename: None,
        }
    }

    pub const fn renamed(url: &'static str, subdir: &'static str, filename: &'static str) -> Self {
        Self {
            url,
            subdir,
            filename: Some(filename),
        }
    }

    /// Flux Q4/Q8 and Chroma share the T5 + CLIP-L + VAE support trio and differ
    /// only in the unet weights.
    fn flux_assets(unet_url: &'static str) -> Vec<Self> {
        vec![
            Asset::new(FLUX_T5_URL, "clip"),
            Asset::new(FLUX_CLIP_URL, "clip"),
            Asset::renamed(FLUX_VAE_URL, "vae", "flux_ae.safetensors"),
            Asset::new(unet_url, "unet"),
        ]
    }

    /// Qwen ships its own encoder and VAE, so it shares nothing with Flux.
    fn qwen_assets() -> Vec<Self> {
        vec![
            Asset::new(QWEN_URL, "unet"),
            Asset::new(QWEN_VAE_URL, "vae"),
            Asset::new(QWEN_CLIP_URL, "text_encoders"),
        ]
    }

    pub fn dest(&self) -> PathBuf {
        let name = self
            .filename
            .unwrap_or_else(|| self.url.rsplit('/').next().unwrap_or(self.url));

        layout::comfy_models_dir().join(self.subdir).join(name)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn flux_q4_pairs_its_unet_with_the_shared_support_trio() {
        // given the Flux Q4 model
        let assets = Model::FluxQ4.assets();

        // then it carries T5 + CLIP-L + VAE + its own unet
        assert_eq!(assets.len(), 4);
        assert!(
            assets
                .iter()
                .any(|a| a.url == FLUX_Q4_URL && a.subdir == "unet")
        );
        assert!(
            assets
                .iter()
                .any(|a| a.url == FLUX_T5_URL && a.subdir == "clip")
        );
        assert!(
            assets
                .iter()
                .any(|a| a.url == FLUX_CLIP_URL && a.subdir == "clip")
        );

        // and only the VAE is renamed, in models/vae
        let vae = assets
            .iter()
            .find(|a| a.url == FLUX_VAE_URL)
            .expect("vae present");
        assert_eq!(vae.subdir, "vae");
        assert_eq!(vae.filename, Some("flux_ae.safetensors"));

        // and the unet keeps its own name
        let unet = assets
            .iter()
            .find(|a| a.url == FLUX_Q4_URL)
            .expect("unet present");
        assert_eq!(unet.filename, None);
    }

    #[test]
    fn chroma_reuses_flux_support_with_its_own_unet() {
        // given Chroma
        let assets = Model::Chroma.assets();

        // then it has the same support trio but the Chroma unet
        assert_eq!(assets.len(), 4);
        assert!(
            assets
                .iter()
                .any(|a| a.url == CHROMA_URL && a.subdir == "unet")
        );
        assert!(assets.iter().any(|a| a.url == FLUX_T5_URL));
        assert!(assets.iter().any(|a| a.url == FLUX_VAE_URL));
    }

    #[test]
    fn qwen_brings_its_own_encoder_and_vae() {
        // given Qwen
        let assets = Model::Qwen.assets();

        // then it ships unet + its own vae + its own text encoder, no Flux files
        assert_eq!(assets.len(), 3);
        assert!(
            assets
                .iter()
                .any(|a| a.url == QWEN_URL && a.subdir == "unet")
        );
        assert!(
            assets
                .iter()
                .any(|a| a.url == QWEN_VAE_URL && a.subdir == "vae")
        );
        assert!(
            assets
                .iter()
                .any(|a| a.url == QWEN_CLIP_URL && a.subdir == "text_encoders")
        );
        assert!(!assets.iter().any(|a| a.url == FLUX_T5_URL));
    }

    #[test]
    fn dest_uses_the_url_basename_unless_renamed() {
        // given an asset with no rename and one with a rename
        let plain = Asset::new(FLUX_Q4_URL, "unet");
        let renamed = Asset::renamed(FLUX_VAE_URL, "vae", "flux_ae.safetensors");

        // then the plain one keeps the URL's filename
        assert!(plain.dest().ends_with("unet/flux1-dev-Q4_K_S.gguf"));

        // and the renamed one uses the override
        assert!(renamed.dest().ends_with("vae/flux_ae.safetensors"));
    }
}
