use serde_json::{Value, json};

use crate::backend::application::port::image_port::ImageError;
use crate::backend::domain::character::ImageFraming;
use crate::config::SupportedModel;

const LOAD_IMAGE_NODE: &str = "20";
const REFERENCE_LATENT_NODE: &str = "23";
const KONTEXT_LATENT_NODE: &str = "24";

/// Each graph below samples a square, so a full-body reference is exactly this size.
const FULL_BODY_SIZE: u32 = 1024;
const PORTRAIT_WIDTH: u32 = 832;
const PORTRAIT_HEIGHT: u32 = 1216;

pub struct Workflow {
    prompt: String,
    destination: String,
    reference: Option<(String, ImageFraming)>,
}

impl Workflow {
    pub fn new(prompt: &str, destination: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            destination: destination.to_string(),
            reference: None,
        }
    }

    pub fn with_shot_reference(mut self, reference: &str, framing: ImageFraming) -> Self {
        self.reference = Some((reference.to_string(), framing));
        self
    }

    /// `original` makes shots from nothing; `edit` makes them from the reference.
    pub fn get_workflow_prompt(
        &self,
        original: SupportedModel,
        edit: SupportedModel,
    ) -> Result<Value, ImageError> {
        let Some((reference, framing)) = &self.reference else {
            return self.graph(original);
        };

        if edit.is_reference_model() {
            return Ok(self.kontext(reference, *framing, edit));
        }

        Ok(Self::img2img(
            self.graph(edit)?,
            reference,
            Self::denoise(edit),
        ))
    }

    /// How much of the reference the model is allowed to redraw
    fn denoise(workflow: SupportedModel) -> f64 {
        match workflow {
            SupportedModel::FluxQ4 | SupportedModel::FluxQ8 => 0.6,
            SupportedModel::Qwen => 0.75,
            SupportedModel::FluxKontextQ4 | SupportedModel::FluxKontextQ8 => 1.0,
        }
    }

    fn graph(&self, workflow: SupportedModel) -> Result<Value, ImageError> {
        match workflow {
            SupportedModel::FluxQ8 => Ok(self.flux_q8()),
            SupportedModel::FluxQ4 => Ok(self.flux_q4()),
            SupportedModel::Qwen => Ok(self.qwen()),
            SupportedModel::FluxKontextQ4 | SupportedModel::FluxKontextQ8 => Err(
                ImageError::Provider(format!("{workflow:?} only draws from a reference")),
            ),
        }
    }

    /// Kontext regenerates the shot from the instruction while keeping most of its integrity.
    fn kontext(&self, reference: &str, framing: ImageFraming, workflow: SupportedModel) -> Value {
        let (width, height) = match framing {
            ImageFraming::Portrait => (PORTRAIT_WIDTH, PORTRAIT_HEIGHT),
            ImageFraming::FullBody | ImageFraming::FullBodyPose => (FULL_BODY_SIZE, FULL_BODY_SIZE),
        };
        let unet_name = match workflow {
            SupportedModel::FluxKontextQ8 => "flux1-kontext-dev-Q8_0.gguf",
            _ => "flux1-kontext-dev-Q4_K_S.gguf",
        };

        json!({
            "10": { "class_type": "UnetLoaderGGUF", "inputs": { "unet_name": unet_name } },
            "11": { "class_type": "DualCLIPLoaderGGUF", "inputs": {
                "clip_name1": "t5-v1_1-xxl-encoder-Q5_K_M.gguf",
                "clip_name2": "clip_l.safetensors",
                "type": "flux"
            } },
            "12": { "class_type": "VAELoader", "inputs": { "vae_name": "flux_ae.safetensors" } },
            "20": { "class_type": "LoadImage", "inputs": { "image": reference } },
            "21": { "class_type": "FluxKontextImageScale", "inputs": { "image": [LOAD_IMAGE_NODE, 0] } },
            "22": { "class_type": "VAEEncode", "inputs": {
                "pixels": ["21", 0],
                "vae": ["12", 0]
            } },
            "6": { "class_type": "CLIPTextEncode", "inputs": { "text": self.prompt, "clip": ["11", 0] } },
            "7": { "class_type": "CLIPTextEncode", "inputs": { "text": "", "clip": ["11", 0] } },
            "24": { "class_type": "ReferenceLatent", "inputs": {
                "conditioning": ["6", 0],
                "latent": ["22", 0]
            } },
            "13": { "class_type": "FluxGuidance", "inputs": { "guidance": 2.5, "conditioning": [KONTEXT_LATENT_NODE, 0] } },
            "5": { "class_type": "EmptySD3LatentImage", "inputs": { "width": width, "height": height, "batch_size": 1 } },
            "3": { "class_type": "KSampler", "inputs": {
                "seed": 42,
                "steps": 20,
                "cfg": 1,
                "sampler_name": "euler",
                "scheduler": "simple",
                "denoise": Self::denoise(workflow),
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

    /// The fallback when no reference model is provisioned, use the default model.
    fn img2img(mut graph: Value, reference: &str, denoise: f64) -> Value {
        let sampler = node_id(&graph, "KSampler");
        let vae = node_id(&graph, "VAELoader");

        graph[LOAD_IMAGE_NODE] = json!({
            "class_type": "LoadImage",
            "inputs": { "image": reference }
        });
        graph[REFERENCE_LATENT_NODE] = json!({
            "class_type": "VAEEncode",
            "inputs": {
                "pixels": [LOAD_IMAGE_NODE, 0],
                "vae": [vae.as_deref().unwrap_or_default(), 0]
            }
        });

        if let Some(sampler) = sampler {
            graph[sampler.as_str()]["inputs"]["latent_image"] = json!([REFERENCE_LATENT_NODE, 0]);
            graph[sampler.as_str()]["inputs"]["denoise"] = json!(denoise);
        }

        graph
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
    use crate::backend::domain::character::ImageFraming;
    use serde_json::json;

    #[test]
    fn kontext_draws_a_portrait_from_an_empty_latent_the_framing_sizes() {
        // given a full body to derive a portrait from
        let workflow = Workflow::new("portrait of a lamplighter", "salt_ash/wren/wren_portrait_0")
            .with_shot_reference("wren_full_body_0_00001_.png", ImageFraming::Portrait);

        // when
        let graph = workflow
            .get_workflow_prompt(SupportedModel::FluxQ8, SupportedModel::FluxKontextQ4)
            .expect("a kontext graph");

        // then the reference only conditions, and the sampler starts from a portrait latent
        assert_eq!(
            graph["20"]["inputs"]["image"],
            json!("wren_full_body_0_00001_.png")
        );
        assert_eq!(graph["21"]["class_type"], json!("FluxKontextImageScale"));
        assert_eq!(graph["24"]["class_type"], json!("ReferenceLatent"));
        assert_eq!(graph["24"]["inputs"]["latent"], json!(["22", 0]));
        assert_eq!(graph["13"]["inputs"]["conditioning"], json!(["24", 0]));
        assert_eq!(graph["5"]["inputs"]["width"], json!(832));
        assert_eq!(graph["5"]["inputs"]["height"], json!(1216));
        assert_eq!(graph["3"]["inputs"]["latent_image"], json!(["5", 0]));
        assert_eq!(graph["3"]["inputs"]["denoise"], json!(1.0));
    }

    #[test]
    fn kontext_loads_the_quant_the_config_asks_for() {
        // given the same shot against the q8 weights
        let workflow = Workflow::new("portrait of a lamplighter", "salt_ash/wren/wren_portrait_0")
            .with_shot_reference("wren_full_body_0_00001_.png", ImageFraming::Portrait);

        // when
        let graph = workflow
            .get_workflow_prompt(SupportedModel::FluxQ8, SupportedModel::FluxKontextQ8)
            .expect("a kontext graph");

        // then it loads the q8 unet and keeps the flux support trio
        assert_eq!(
            graph["10"]["inputs"]["unet_name"],
            json!("flux1-kontext-dev-Q8_0.gguf")
        );
        assert_eq!(graph["11"]["inputs"]["type"], json!("flux"));
    }

    #[test]
    fn without_a_reference_model_the_shot_falls_back_to_img2img() {
        // given the same portrait asked for from both plain families
        let portrait = Workflow::new("portrait of a lamplighter", "salt_ash/wren/wren_portrait_0")
            .with_shot_reference("wren_full_body_0_00001_.png", ImageFraming::Portrait);

        // when
        let flux = portrait
            .get_workflow_prompt(SupportedModel::Qwen, SupportedModel::FluxQ8)
            .expect("a flux graph");
        let qwen = portrait
            .get_workflow_prompt(SupportedModel::Qwen, SupportedModel::Qwen)
            .expect("a qwen graph");

        // then the reference becomes the latent, uncropped, and qwen redraws more of it
        assert_eq!(flux["23"]["class_type"], json!("VAEEncode"));
        assert_eq!(flux["23"]["inputs"]["pixels"], json!(["20", 0]));
        assert_eq!(flux["3"]["inputs"]["latent_image"], json!(["23", 0]));
        assert!(flux["21"].is_null());
        assert!(qwen["3"]["inputs"]["denoise"].as_f64() > flux["3"]["inputs"]["denoise"].as_f64());
    }

    #[test]
    fn a_shot_without_a_reference_uses_the_original_workflow() {
        // given a first shot, nothing to stay faithful to
        let workflow = Workflow::new(
            "full body of a lamplighter",
            "salt_ash/wren/wren_full_body_0",
        );

        // when
        let graph = workflow
            .get_workflow_prompt(SupportedModel::FluxQ8, SupportedModel::FluxKontextQ4)
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
        assert!(graph["20"].is_null());
    }

    #[test]
    fn kontext_refuses_to_make_an_original() {
        // given kontext configured as the workflow originals come from
        let workflow = Workflow::new(
            "full body of a lamplighter",
            "salt_ash/wren/wren_full_body_0",
        );

        // when
        let err = workflow
            .get_workflow_prompt(SupportedModel::FluxKontextQ4, SupportedModel::FluxKontextQ4)
            .expect_err("kontext has no text-to-image graph");

        // then it says so rather than drawing the wrong thing
        assert!(
            err.to_string().contains("only draws from a reference"),
            "{err}"
        );
    }
}
