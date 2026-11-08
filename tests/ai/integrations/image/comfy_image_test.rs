use std::path::PathBuf;

use living_voices::ai::integrations::image::comfy_image::ComfyImage;
use living_voices::backend::application::port::image_port::ImagePort;
use living_voices::config::{ImageModel, ImageModelWorkflow};

const COMFYUI_URL: &str = "http://localhost:8188";
const OUTPUT_DIR: &str = "comfyui/output";

fn comfy() -> ComfyImage {
    ComfyImage::new(ImageModel {
        enabled: true,
        backend: "comfyui".to_string(),
        base_url: COMFYUI_URL.to_string(),
        workflow: ImageModelWorkflow::Qwen,
        output_dir: PathBuf::from(OUTPUT_DIR),
    })
    .expect("build ComfyUI image handler")
}

#[tokio::test]
#[ignore = "requires a running ComfyUI server with the Qwen workflow model"]
async fn generate_scene_queues_a_scene_prompt() {
    comfy()
        .queue_generate_scene(
            "A flooded harbour at dusk, broken masts rising from black water, \
             pale lantern light, painterly fantasy illustration.",
            "living_voices-tests/scenes/harbour",
        )
        .await
        .expect("generate_scene");
}

#[tokio::test]
#[ignore = "requires a running ComfyUI server with the Qwen workflow model"]
async fn generate_character_image_queues_a_character_prompt() {
    comfy()
        .queue_generate_character_image(
            "Full-body character reference of a young lamplighter in an oversized \
             oilskin coat and tarred sou'wester, neutral pose, plain background.",
            "living_voices-tests/mira/portrait",
            None,
        )
        .await
        .expect("generate_character_image");
}
