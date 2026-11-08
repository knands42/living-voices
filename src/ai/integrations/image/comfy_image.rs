use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::ai::integrations::image::workflow::Workflow;
use crate::backend::application::port::image_port::{ImageError, ImagePort, ShotReference};
use crate::config::ImageModel;

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

    async fn queue(&self, workflow: &Workflow) -> Result<(), ImageError> {
        let prompt_id = self.submit(workflow).await?;

        tokio::time::timeout(WAIT_TIMEOUT, self.wait_for_result(&prompt_id))
            .await
            .map_err(|_| {
                ImageError::Provider(format!("{prompt_id} did not finish in {WAIT_TIMEOUT:?}"))
            })?
    }

    /// The shot the new one is drawn against has to be in ComfyUI's workspace first.
    async fn shot_workflow(
        &self,
        prompt: &str,
        destination: &str,
        reference: Option<ShotReference<'_>>,
    ) -> Result<Workflow, ImageError> {
        let workflow = Workflow::new(prompt, destination);

        let Some(reference) = reference else {
            return Ok(workflow);
        };

        let staged = self.stage_reference(reference).await?;

        Ok(workflow.with_reference(&staged, reference.framing))
    }

    /// ComfyUI's `LoadImage` only reads its workspace `input` dir, so a shot needs to be copied there.
    async fn stage_reference(&self, reference: ShotReference<'_>) -> Result<String, ImageError> {
        let name = reference.path.file_name().ok_or_else(|| {
            ImageError::Provider(format!("{} is not a file", reference.path.display()))
        })?;
        let input_dir = self.input_dir();

        tokio::fs::create_dir_all(&input_dir)
            .await
            .map_err(|e| ImageError::Provider(format!("{}: {e}", input_dir.display())))?;
        tokio::fs::copy(reference.path, input_dir.join(name))
            .await
            .map_err(|e| ImageError::Provider(format!("{}: {e}", reference.path.display())))?;

        Ok(name.to_string_lossy().into_owned())
    }

    fn input_dir(&self) -> PathBuf {
        self.config
            .output_dir
            .parent()
            .map(|workspace| workspace.join("input"))
            .unwrap_or_else(|| PathBuf::from("input"))
    }

    async fn submit(&self, workflow: &Workflow) -> Result<String, ImageError> {
        let graph = workflow.get_workflow_prompt(self.config.workflow)?;

        let url = format!("{}/prompt", self.base_url());
        let response = self
            .http
            .post(&url)
            .json(&serde_json::json!({ "prompt": graph }))
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
        self.queue(&Workflow::new(prompt, destination)).await
    }

    async fn queue_generate_character_image(
        &self,
        prompt: &str,
        destination: &str,
        reference: Option<ShotReference<'_>>,
    ) -> Result<(), ImageError> {
        let workflow = self.shot_workflow(prompt, destination, reference).await?;
        self.queue(&workflow).await
    }

    async fn generate_character_image(
        &self,
        prompt: &str,
        destination: &str,
        reference: Option<ShotReference<'_>>,
    ) -> Result<(), ImageError> {
        let workflow = self.shot_workflow(prompt, destination, reference).await?;
        self.submit(&workflow).await?;

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
