use std::time::Duration;

use serde_json::{Value, json};

use crate::backend::application::port::image_port::{ImageError, ImagePort};
use crate::config::{ImageModel, ImageModelWorkflow};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const WAIT_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

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

    async fn queue(&self, prompt: &str, destination: &str) -> Result<(), ImageError> {
        let prompt_id = self.submit(prompt, destination).await?;

        tokio::time::timeout(WAIT_TIMEOUT, self.wait_for_result(&prompt_id))
            .await
            .map_err(|_| {
                ImageError::Provider(format!("{prompt_id} did not finish in {WAIT_TIMEOUT:?}"))
            })?
    }

    async fn submit(&self, prompt: &str, destination: &str) -> Result<String, ImageError> {
        let workflow_format = pick_workflow_format(&self.config.workflow, prompt, destination);

        let url = format!("{}/prompt", self.base_url());
        let response = self
            .http
            .post(&url)
            .json(&json!({ "prompt": workflow_format }))
            .send()
            .await
            .map_err(|e| ImageError::Provider(e.to_string()))?;

        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| ImageError::Provider(e.to_string()))?;

        if !status.is_success() {
            return Err(ImageError::Provider(format!("queue failed: {body}")));
        }

        body["prompt_id"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| ImageError::Provider(format!("no prompt id in {body}")))
    }

    async fn wait_for_result(&self, prompt_id: &str) -> Result<(), ImageError> {
        let url = format!("{}/history/{}", self.base_url(), prompt_id);

        loop {
            let reply: Value = self
                .http
                .get(&url)
                .send()
                .await
                .map_err(|e| ImageError::Provider(e.to_string()))?
                .json()
                .await
                .map_err(|e| ImageError::Provider(e.to_string()))?;

            if let Some(entry) = reply.get(prompt_id) {
                return verdict(entry);
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }
}

impl ImagePort for ComfyImage {
    async fn queue_generate_scene(
        &self,
        prompt: &str,
        destination: &str,
    ) -> Result<(), ImageError> {
        self.queue(prompt, destination).await
    }

    async fn queue_generate_character_image(
        &self,
        prompt: &str,
        destination: &str,
    ) -> Result<(), ImageError> {
        self.queue(prompt, destination).await
    }

    async fn generate_generate_character_image(
        &self,
        prompt: &str,
        destination: &str,
    ) -> Result<(), ImageError> {
        self.submit(prompt, destination).await?;
        Ok(())
    }
}

fn verdict(entry: &Value) -> Result<(), ImageError> {
    let status = &entry["status"];

    if status["status_str"] == "success" && status["completed"] == true {
        return Ok(());
    }

    Err(ImageError::Provider(failure_message(status)))
}

/// ComfyUI reports the failure as `messages: [["execution_error", {exception_message, ...}]]`.
fn failure_message(status: &Value) -> String {
    let detail = status["messages"]
        .as_array()
        .and_then(|messages| {
            messages
                .iter()
                .find(|message| message[0] == "execution_error")
        })
        .and_then(|message| message[1]["exception_message"].as_str());

    match detail {
        Some(detail) => format!("comfy error: {detail}"),
        None => format!("comfy error: {status}"),
    }
}

fn pick_workflow_format(workflow: &ImageModelWorkflow, prompt: &str, prefix: &str) -> Value {
    match workflow {
        ImageModelWorkflow::FluxQ8 => flux_q8_workflow(prompt, prefix),
        ImageModelWorkflow::FluxQ4 => flux_q4_workflow(prompt, prefix),
        ImageModelWorkflow::Qwen => qwen_workflow(prompt, prefix),
    }
}

fn qwen_workflow(prompt: &str, prefix: &str) -> Value {
    json!({
        "10": { "class_type": "UnetLoaderGGUF", "inputs": { "unet_name": "qwen-image-Q4_K_S.gguf" } },
        "11": { "class_type": "CLIPLoader", "inputs": {
            "clip_name": "qwen_2.5_vl_7b_fp8_scaled.safetensors",
            "type": "qwen_image"
        } },
        "12": { "class_type": "VAELoader", "inputs": { "vae_name": "qwen_image_vae.safetensors" } },
        "6": { "class_type": "CLIPTextEncode", "inputs": { "text": prompt, "clip": ["11", 0] } },
        "7": { "class_type": "CLIPTextEncode", "inputs": { "text": "", "clip": ["11", 0] } },
        "5": { "class_type": "EmptySD3LatentImage", "inputs": { "width": 1024, "height": 1024, "batch_size": 1 } },
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
            "filename_prefix": prefix,
            "images": ["8", 0]
        } }
    })
}

fn flux_q4_workflow(prompt: &str, prefix: &str) -> Value {
    json!({
        "10": { "class_type": "UnetLoaderGGUF", "inputs": { "unet_name": "flux1-dev-Q4_K_S.gguf" } },
        "11": { "class_type": "DualCLIPLoaderGGUF", "inputs": {
            "clip_name1": "t5-v1_1-xxl-encoder-Q5_K_M.gguf",
            "clip_name2": "clip_l.safetensors",
            "type": "flux"
        } },
        "12": { "class_type": "VAELoader", "inputs": { "vae_name": "flux_ae.safetensors" } },
        "6": { "class_type": "CLIPTextEncode", "inputs": { "text": prompt, "clip": ["11", 0] } },
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
            "filename_prefix": prefix,
            "images": ["8", 0]
        } }
    })
}

fn flux_q8_workflow(prompt: &str, prefix: &str) -> Value {
    json!({
        "10": { "class_type": "UnetLoaderGGUF", "inputs": { "unet_name": "flux1-dev-Q8_0.gguf" } },
        "11": { "class_type": "DualCLIPLoaderGGUF", "inputs": {
            "clip_name1": "t5-v1_1-xxl-encoder-Q5_K_M.gguf",
            "clip_name2": "clip_l.safetensors",
            "type": "flux"
        } },
        "12": { "class_type": "VAELoader", "inputs": { "vae_name": "flux_ae.safetensors" } },
        "6": { "class_type": "CLIPTextEncode", "inputs": { "text": prompt, "clip": ["11", 0] } },
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
            "filename_prefix": prefix,
            "images": ["8", 0]
        } }
    })
}

#[cfg(test)]
mod test {
    use super::verdict;
    use serde_json::json;

    #[test]
    fn a_completed_run_is_ok() {
        // given the history entry of a run comfy finished
        let entry = json!({
            "outputs": { "9": { "images": [{ "filename": "portrait_00001_.png" }] } },
            "status": { "status_str": "success", "completed": true, "messages": [] }
        });

        // when
        let result = verdict(&entry);

        // then
        assert!(result.is_ok());
    }

    #[test]
    fn an_execution_error_carries_comfy_message() {
        // given a run that died, the way an out-of-memory does
        let entry = json!({
            "outputs": {},
            "status": {
                "status_str": "error",
                "completed": false,
                "messages": [["execution_error", {
                    "exception_type": "torch.OutOfMemoryError",
                    "exception_message": "Allocation on device 0 would exceed allowed memory."
                }]]
            }
        });

        // when
        let err = verdict(&entry).expect_err("a failed run");

        // then it names the reason instead of just the status
        assert!(err.to_string().contains("exceed allowed memory"), "{err}");
    }

    #[test]
    fn an_error_without_a_message_still_reports_something() {
        // given an interrupted run, which reports no exception_message
        let entry = json!({
            "outputs": {},
            "status": { "status_str": "error", "completed": false, "messages": [["execution_interrupted", {}]] }
        });

        // when
        let err = verdict(&entry).expect_err("a failed run");

        // then
        assert!(err.to_string().contains("status_str"), "{err}");
    }
}
