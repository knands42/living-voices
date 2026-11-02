use std::time::Duration;

use serde_json::{Value, json};

use crate::backend::application::port::image_port::{ImageError, ImageHandler};
use crate::config::ImageModel;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

const PROMPT_NODE: &str = "6";
const SAVE_NODE: &str = "9";

#[derive(Debug, Clone)]
pub struct ComfyImage {
    config: ImageModel,
    http: reqwest::Client,
}

impl ComfyImage {
    pub fn new(config: ImageModel) -> Result<Self, ImageError> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| ImageError::Provider(e.to_string()))?;
        Ok(Self { config, http })
    }

    fn base_url(&self) -> &str {
        self.config.base_url.trim_end_matches('/')
    }

    fn prefix(&self, kind: &str) -> String {
        format!("{}/{kind}", self.config.output_prefix)
    }

    async fn queue(&self, prompt: &str, kind: &str) -> Result<(), ImageError> {
        let mut workflow = flux_q8_workflow();
        workflow[PROMPT_NODE]["inputs"]["text"] = json!(prompt);
        workflow[SAVE_NODE]["inputs"]["filename_prefix"] = json!(self.prefix(kind));

        let url = format!("{}/prompt", self.base_url());
        let response = self
            .http
            .post(&url)
            .json(&json!({ "prompt": workflow }))
            .send()
            .await
            .map_err(|e| ImageError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ImageError::Provider(format!("queue failed: {body}")));
        }
        Ok(())
    }
}

impl ImageHandler for ComfyImage {
    async fn generate_scene(&self, prompt: &str) -> Result<(), ImageError> {
        self.queue(prompt, "scenes").await
    }

    async fn generate_character_portrait(&self, prompt: &str) -> Result<(), ImageError> {
        self.queue(prompt, "portraits").await
    }

    async fn generate_character_image(&self, prompt: &str) -> Result<(), ImageError> {
        self.queue(prompt, "characters").await
    }
}

fn flux_q8_workflow() -> Value {
    json!({
        "10": { "class_type": "UnetLoaderGGUF", "inputs": { "unet_name": "flux1-dev-Q8_0.gguf" } },
        "11": { "class_type": "DualCLIPLoaderGGUF", "inputs": {
            "clip_name1": "t5-v1_1-xxl-encoder-Q5_K_M.gguf",
            "clip_name2": "clip_l.safetensors",
            "type": "flux"
        } },
        "12": { "class_type": "VAELoader", "inputs": { "vae_name": "flux_ae.safetensors" } },
        "6": { "class_type": "CLIPTextEncode", "inputs": { "text": "", "clip": ["11", 0] } },
        "7": { "class_type": "CLIPTextEncode", "inputs": { "text": "", "clip": ["11", 0] } },
        "13": { "class_type": "FluxGuidance", "inputs": { "guidance": 3.5, "conditioning": ["6", 0] } },
        "5": { "class_type": "EmptySD3LatentImage", "inputs": { "width": 1024, "height": 1024, "batch_size": 1 } },
        "3": { "class_type": "KSampler", "inputs": {
            "seed": 42,
            "steps": 20,
            "cfg": 1,
            "sampler_name": "euler",
            "scheduler": "simple",
            "denoise": 1,
            "model": ["10", 0],
            "positive": ["13", 0],
            "negative": ["7", 0],
            "latent_image": ["5", 0]
        } },
        "8": { "class_type": "VAEDecode", "inputs": { "samples": ["3", 0], "vae": ["12", 0] } },
        "9": { "class_type": "SaveImage", "inputs": {
            "filename_prefix": "living_voices",
            "images": ["8", 0]
        } }
    })
}
