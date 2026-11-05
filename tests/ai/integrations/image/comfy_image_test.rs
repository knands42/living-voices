use living_voices::ai::integrations::image::comfy_image::ComfyImage;
use living_voices::backend::application::port::image_port::ImageHandler;
use living_voices::config::{ImageModel, ImageModelWorkflow};

const COMFYUI_URL: &str = "http://localhost:8188";
const OUTPUT_PREFIX: &str = "living_voices-tests";

fn comfy() -> ComfyImage {
    ComfyImage::new(ImageModel {
        backend: "comfyui".to_string(),
        base_url: COMFYUI_URL.to_string(),
        workflow: ImageModelWorkflow::Qwen,
        output_prefix: OUTPUT_PREFIX.to_string(),
    })
    .expect("build ComfyUI image handler")
}

#[tokio::test]
#[ignore = "requires a running ComfyUI server with the Qwen workflow model"]
async fn generate_scene_queues_a_scene_prompt() {
    comfy()
        .generate_scene(
            "A flooded harbour at dusk, broken masts rising from black water, \
             pale lantern light, painterly fantasy illustration.",
        )
        .await
        .expect("generate_scene");
}

#[tokio::test]
#[ignore = "requires a running ComfyUI server with the Qwen workflow model"]
async fn generate_character_image_queues_a_character_prompt() {
    comfy()
        .generate_character_image(
            "Full-body character reference of a young lamplighter in an oversized \
             oilskin coat and tarred sou'wester, neutral pose, plain background.",
        )
        .await
        .expect("generate_character_image");
}
