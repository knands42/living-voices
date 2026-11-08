use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::backend::application::port::image_port::ImageError;
use crate::backend::domain::character::ImageFraming;

const LOAD_IMAGE_NODE: &str = "20";
const SCALED_IMAGE_NODE: &str = "21";
const CROPPED_IMAGE_NODE: &str = "22";
const REFERENCE_LATENT_NODE: &str = "23";

/// Each graph below samples a square, so a full-body reference is exactly this size.
const FULL_BODY_SIZE: u32 = 1024;
const PORTRAIT_WIDTH: u32 = 832;
const PORTRAIT_HEIGHT: u32 = 1216;
/// The head-and-torso window of a full body, kept to the portrait aspect so the scale
/// after it only resizes: 640 / 936 is 832 / 1216.
const PORTRAIT_CROP_WIDTH: u32 = 640;
const PORTRAIT_CROP_HEIGHT: u32 = 936;

/// Which ComfyUI graph a shot is built from.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowType {
    Qwen,
    FluxQ4,
    FluxQ8,
}

/// The graph one shot is submitted with: the wording, the file name, and the shot it
/// has to stay faithful to.
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

    /// `reference` is the shot on disk the new one is drawn from, already staged where
    /// the provider can read it.
    pub fn with_reference(mut self, reference: &str, framing: ImageFraming) -> Self {
        self.reference = Some((reference.to_string(), framing));
        self
    }

    /// Ready to submit: the whole graph, wired for this workflow and its reference.
    pub fn get_workflow_prompt(&self, workflow: WorkflowType) -> Result<Value, ImageError> {
        let graph = self.graph(workflow);

        match &self.reference {
            Some((reference, framing)) => {
                Self::img2img(graph, reference, *framing, Self::denoise(workflow))
            }
            None => Ok(graph),
        }
    }

    /// How much of the reference the model is allowed to redraw. Flux holds a face at a
    /// low denoise; Qwen at cfg 2.5 just re-renders the reference unless it is pushed.
    fn denoise(workflow: WorkflowType) -> f64 {
        match workflow {
            WorkflowType::FluxQ4 | WorkflowType::FluxQ8 => 0.6,
            WorkflowType::Qwen => 0.75,
        }
    }

    fn graph(&self, workflow: WorkflowType) -> Value {
        match workflow {
            WorkflowType::FluxQ8 => self.flux_q8(),
            WorkflowType::FluxQ4 => self.flux_q4(),
            WorkflowType::Qwen => self.qwen(),
        }
    }

    /// Turns a text-to-image graph into img2img: the reference is loaded, framed, and
    /// encoded into the latent the sampler starts from, so the character carries over
    /// into the new shot instead of the model inventing another one.
    fn img2img(
        mut graph: Value,
        reference: &str,
        framing: ImageFraming,
        denoise: f64,
    ) -> Result<Value, ImageError> {
        let sampler = node_id(&graph, "KSampler")?;
        let vae = node_id(&graph, "VAELoader")?;

        graph[LOAD_IMAGE_NODE] = json!({
            "class_type": "LoadImage",
            "inputs": { "image": reference }
        });

        let framed = match framing {
            ImageFraming::Portrait => {
                graph[CROPPED_IMAGE_NODE] = json!({
                    "class_type": "ImageCrop",
                    "inputs": {
                        "image": [LOAD_IMAGE_NODE, 0],
                        "width": PORTRAIT_CROP_WIDTH,
                        "height": PORTRAIT_CROP_HEIGHT,
                        "x": (FULL_BODY_SIZE - PORTRAIT_CROP_WIDTH) / 2,
                        "y": 0
                    }
                });
                graph[SCALED_IMAGE_NODE] = json!({
                    "class_type": "ImageScale",
                    "inputs": {
                        "image": [CROPPED_IMAGE_NODE, 0],
                        "upscale_method": "lanczos",
                        "width": PORTRAIT_WIDTH,
                        "height": PORTRAIT_HEIGHT,
                        "crop": "disabled"
                    }
                });
                SCALED_IMAGE_NODE
            }
            ImageFraming::FullBody | ImageFraming::FullBodyPose => LOAD_IMAGE_NODE,
        };

        graph[REFERENCE_LATENT_NODE] = json!({
            "class_type": "VAEEncode",
            "inputs": {
                "pixels": [framed, 0],
                "vae": [vae, 0]
            }
        });

        graph[sampler.as_str()]["inputs"]["latent_image"] = json!([REFERENCE_LATENT_NODE, 0]);
        graph[sampler.as_str()]["inputs"]["denoise"] = json!(denoise);

        Ok(graph)
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
fn node_id(graph: &Value, class_type: &str) -> Result<String, ImageError> {
    graph
        .as_object()
        .and_then(|nodes| {
            nodes
                .iter()
                .find_map(|(id, node)| (node["class_type"] == class_type).then(|| id.to_string()))
        })
        .ok_or_else(|| {
            ImageError::Provider(format!("the workflow has no {class_type} node to rewire"))
        })
}

#[cfg(test)]
mod test {
    use super::{Workflow, WorkflowType};
    use crate::backend::domain::character::ImageFraming;
    use serde_json::json;

    #[test]
    fn a_portrait_reference_is_cropped_to_the_portrait_aspect_before_encoding() {
        // given a full body to derive a portrait from
        let workflow = Workflow::new("portrait of a lamplighter", "salt_ash/wren/wren_portrait_0")
            .with_reference("wren_full_body_0_00001_.png", ImageFraming::Portrait);

        // when
        let graph = workflow
            .get_workflow_prompt(WorkflowType::FluxQ8)
            .expect("a flux graph to rewire");

        // then the head and torso are cropped out and the sampler starts from them
        assert_eq!(
            graph["20"]["inputs"]["image"],
            json!("wren_full_body_0_00001_.png")
        );
        assert_eq!(graph["22"]["class_type"], json!("ImageCrop"));
        assert_eq!(graph["22"]["inputs"]["width"], json!(640));
        assert_eq!(graph["21"]["inputs"]["width"], json!(832));
        assert_eq!(graph["21"]["inputs"]["height"], json!(1216));
        assert_eq!(graph["23"]["inputs"]["pixels"], json!(["21", 0]));
        assert_eq!(graph["23"]["inputs"]["vae"], json!(["12", 0]));
        assert_eq!(graph["3"]["inputs"]["latent_image"], json!(["23", 0]));
        assert!(
            graph["3"]["inputs"]["denoise"]
                .as_f64()
                .is_some_and(|d| d < 1.0)
        );
    }

    #[test]
    fn a_pose_keeps_the_whole_full_body_frame() {
        // given the same reference asked for as a pose, which is a full body shot
        let workflow = Workflow::new(
            "the same lamplighter, reaching for the lamp",
            "salt_ash/wren/wren_pose_0",
        )
        .with_reference("wren_full_body_0_00001_.png", ImageFraming::FullBodyPose);

        // when
        let graph = workflow
            .get_workflow_prompt(WorkflowType::FluxQ8)
            .expect("a flux graph to rewire");

        // then nothing is cropped or scaled, the reference itself becomes the latent
        assert!(graph["21"].is_null());
        assert!(graph["22"].is_null());
        assert_eq!(graph["23"]["inputs"]["pixels"], json!(["20", 0]));
        assert_eq!(graph["3"]["inputs"]["latent_image"], json!(["23", 0]));
    }

    #[test]
    fn qwen_redraws_more_of_the_reference_than_flux() {
        // given the same portrait against both families
        let workflow = Workflow::new("portrait of a lamplighter", "salt_ash/wren/wren_portrait_0")
            .with_reference("wren_full_body_0_00001_.png", ImageFraming::Portrait);

        // when
        let flux = workflow
            .get_workflow_prompt(WorkflowType::FluxQ8)
            .expect("a flux graph");
        let qwen = workflow
            .get_workflow_prompt(WorkflowType::Qwen)
            .expect("a qwen graph");

        // then qwen denoises further, and both keep the prompt and the filename
        assert!(
            qwen["3"]["inputs"]["denoise"].as_f64() > flux["3"]["inputs"]["denoise"].as_f64(),
            "{qwen} vs {flux}"
        );
        assert_eq!(
            qwen["6"]["inputs"]["text"],
            json!("portrait of a lamplighter")
        );
        assert_eq!(
            qwen["9"]["inputs"]["filename_prefix"],
            json!("salt_ash/wren/wren_portrait_0")
        );
    }

    #[test]
    fn without_a_reference_the_graph_starts_from_an_empty_latent() {
        // given a first shot, nothing to stay faithful to
        let workflow = Workflow::new(
            "full body of a lamplighter",
            "salt_ash/wren/wren_full_body_0",
        );

        // when
        let graph = workflow
            .get_workflow_prompt(WorkflowType::FluxQ8)
            .expect("a flux graph");

        // then the sampler keeps its own latent, full denoise, and the prompt reaches it
        assert_eq!(graph["3"]["inputs"]["latent_image"], json!(["5", 0]));
        assert_eq!(graph["3"]["inputs"]["denoise"], json!(1));
        assert_eq!(
            graph["6"]["inputs"]["text"],
            json!("full body of a lamplighter")
        );
        assert!(graph["20"].is_null());
    }
}
