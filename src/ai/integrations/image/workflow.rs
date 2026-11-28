use serde_json::{Value, json};

use crate::backend::application::port::image_port::ImageError;
use crate::config::SupportedModel;

/// Each graph below samples a square, the size every character shot uses.
const FULL_BODY_SIZE: u32 = 1024;

pub struct Workflow {
    prompt: String,
    destination: String,
    loras: Vec<(String, f32)>,
}

impl Workflow {
    pub fn new(prompt: &str, destination: &str, loras: Vec<(String, f32)>) -> Self {
        Self {
            prompt: prompt.to_string(),
            destination: destination.to_string(),
            loras,
        }
    }

    pub fn get_workflow_prompt(&self, model: SupportedModel) -> Result<Value, ImageError> {
        let graph = match model {
            SupportedModel::FluxQ8 => self.flux_q8(),
            SupportedModel::FluxQ4 => self.flux_q4(),
            SupportedModel::Qwen => self.qwen(),
        };

        Ok(with_loras(graph, &self.loras))
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

/// Adapters are chained onto the unet before the sampler, and skipped when the graph has
fn with_loras(mut graph: Value, loras: &[(String, f32)]) -> Value {
    if loras.is_empty() {
        return graph;
    }

    let Some(unet) = node_id(&graph, "UnetLoaderGGUF") else {
        return graph;
    };
    let Some(sampler) = node_id(&graph, "KSampler") else {
        return graph;
    };

    let mut source = unet;
    for (index, (name, scale)) in loras.iter().enumerate() {
        let id = format!("9{index}");
        graph[id.as_str()] = json!({
            "class_type": "LoraLoaderModelOnly",
            "inputs": {
                "model": [source, 0],
                "lora_name": name,
                "strength_model": scale
            }
        });
        source = id;
    }

    graph[sampler.as_str()]["inputs"]["model"] = json!([source, 0]);

    graph
}

/// Nodes are found by what they do, not by the id a template happened to give them.
fn node_id(graph: &Value, class_type: &str) -> Option<String> {
    graph.as_object().and_then(|nodes| {
        nodes
            .iter()
            .find_map(|(id, node)| (node["class_type"] == class_type).then(|| id.to_string()))
    })
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
            Vec::new(),
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

    #[test]
    fn adapters_are_chained_onto_the_unet() {
        // given an adapter the builder registered for the character
        let workflow = Workflow::new(
            "full body of a lamplighter",
            "salt_ash/wren/wren_full_body_0",
            vec![("wren.safetensors".to_string(), 0.5)],
        );

        // when
        let graph = workflow
            .get_workflow_prompt(SupportedModel::FluxQ8)
            .expect("a flux graph");

        // then the sampler reads the adapter, and the adapter reads the unet
        assert_eq!(graph["90"]["class_type"], json!("LoraLoaderModelOnly"));
        assert_eq!(graph["90"]["inputs"]["model"], json!(["10", 0]));
        assert_eq!(
            graph["90"]["inputs"]["lora_name"],
            json!("wren.safetensors")
        );
        assert_eq!(graph["90"]["inputs"]["strength_model"], json!(0.5));
        assert_eq!(graph["3"]["inputs"]["model"], json!(["90", 0]));
    }
}
