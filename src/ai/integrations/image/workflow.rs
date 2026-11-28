use serde_json::{Value, json};

use crate::backend::application::port::image_port::ImageError;
use crate::config::SupportedModel;

/// Each graph below samples a square, the size every character shot uses.
const FULL_BODY_SIZE: u32 = 1024;

pub struct Workflow {
    prompt: String,
    destination: String,
}

impl Workflow {
    pub fn new(prompt: &str, destination: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            destination: destination.to_string(),
        }
    }

    pub fn get_workflow_prompt(&self, model: SupportedModel) -> Result<Value, ImageError> {
        match model {
            SupportedModel::FluxQ8 => Ok(self.flux_q8()),
            SupportedModel::FluxQ4 => Ok(self.flux_q4()),
            SupportedModel::Qwen => Ok(self.qwen()),
        }
    }

    fn qwen(&self) -> Value {
        json!({
            "10": { "class_type": "UnetLoaderGGUF", "inputs": { "unet_name": "qwen-image-Q4_K_S.gguf" } },
            "11": { "class_type": "CLIPLoader", "inputs": {
                "clip_name": "qwen_2.5_vl_7b_fp8_scaled.safetensors",
                "type": "qwen_image"
            } },
            "12": { "class_type": "VAELoader", "inputs": { "vae_name": "qwen_image_vae.safetensors" } },
            "6": { "class_type": "CLIPTextEncode", "inputs": { "text": self.prompt, "clip": ["11", 0] } },
            "7": { "class_type": "CLIPTextEncode", "inputs": { "text": "", "clip": ["11", 0] } },
            "5": { "class_type": "EmptySD3LatentImage", "inputs": { "width": FULL_BODY_SIZE, "height": FULL_BODY_SIZE, "batch_size": 1 } },
            "3": { "class_type": "KSampler", "inputs": {
                "seed": 42,
                "steps": 20,
                "cfg": 2.5,
                "sampler_name": "euler",
                "scheduler": "simple",
                "denoise": 1,
                "model": ["10", 0],
                "positive": ["6", 0],
                "negative": ["7", 0],
                "latent_image": ["5", 0]
            } },
            "8": { "class_type": "VAEDecode", "inputs": { "samples": ["3", 0], "vae": ["12", 0] } },
            "9": { "class_type": "SaveImage", "inputs": {
                "filename_prefix": self.destination,
                "images": ["8", 0]
            } }
        })
    }

    fn flux_q4(&self) -> Value {
        json!({
            "10": { "class_type": "UnetLoaderGGUF", "inputs": { "unet_name": "flux1-dev-Q4_K_S.gguf" } },
            "11": { "class_type": "DualCLIPLoaderGGUF", "inputs": {
                "clip_name1": "t5-v1_1-xxl-encoder-Q5_K_M.gguf",
                "clip_name2": "clip_l.safetensors",
                "type": "flux"
            } },
            "12": { "class_type": "VAELoader", "inputs": { "vae_name": "flux_ae.safetensors" } },
            "6": { "class_type": "CLIPTextEncode", "inputs": { "text": self.prompt, "clip": ["11", 0] } },
            "7": { "class_type": "CLIPTextEncode", "inputs": { "text": "", "clip": ["11", 0] } },
            "13": { "class_type": "FluxGuidance", "inputs": { "guidance": 3.5, "conditioning": ["6", 0] } },
            "5": { "class_type": "EmptySD3LatentImage", "inputs": { "width": FULL_BODY_SIZE, "height": FULL_BODY_SIZE, "batch_size": 1 } },
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
                "filename_prefix": self.destination,
                "images": ["8", 0]
            } }
        })
    }

    fn flux_q8(&self) -> Value {
        json!({
            "10": { "class_type": "UnetLoaderGGUF", "inputs": { "unet_name": "flux1-dev-Q8_0.gguf" } },
            "11": { "class_type": "DualCLIPLoaderGGUF", "inputs": {
                "clip_name1": "t5-v1_1-xxl-encoder-Q5_K_M.gguf",
                "clip_name2": "clip_l.safetensors",
                "type": "flux"
            } },
            "12": { "class_type": "VAELoader", "inputs": { "vae_name": "flux_ae.safetensors" } },
            "6": { "class_type": "CLIPTextEncode", "inputs": { "text": self.prompt, "clip": ["11", 0] } },
            "7": { "class_type": "CLIPTextEncode", "inputs": { "text": "", "clip": ["11", 0] } },
            "13": { "class_type": "FluxGuidance", "inputs": { "guidance": 3.5, "conditioning": ["6", 0] } },
            "5": { "class_type": "EmptySD3LatentImage", "inputs": { "width": FULL_BODY_SIZE, "height": FULL_BODY_SIZE, "batch_size": 1 } },
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
                "filename_prefix": self.destination,
                "images": ["8", 0]
            } }
        })
    }
}

#[cfg(test)]
mod test {
    use super::{SupportedModel, Workflow};
    use serde_json::json;

    #[test]
    fn a_shot_samples_from_an_empty_latent_with_the_prompt() {
        // given a shot with no reference to stay faithful to
        let workflow = Workflow::new(
            "full body of a lamplighter",
            "salt_ash/wren/wren_full_body_0",
        );

        // when
        let graph = workflow
            .get_workflow_prompt(SupportedModel::FluxQ8)
            .expect("a flux graph");

        // then the sampler keeps its own latent, full denoise, and the prompt reaches it
        assert_eq!(graph["3"]["inputs"]["latent_image"], json!(["5", 0]));
        assert_eq!(graph["3"]["inputs"]["denoise"], json!(1));
        assert_eq!(
            graph["6"]["inputs"]["text"],
            json!("full body of a lamplighter")
        );
        assert_eq!(
            graph["9"]["inputs"]["filename_prefix"],
            json!("salt_ash/wren/wren_full_body_0")
        );
    }
}
