# -------------------- comfy (video) -------------------- #
# ComfyUI video models setup.
# Models: Wan 2.1 text-to-video (1.3B, fits 8GB).

# Wan 2.1 T2V 1.3B — fp16 diffusion model + UMT5 text encoder + Wan VAE.
wan_url := "https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/resolve/main/split_files/diffusion_models/wan2.1_t2v_1.3B_fp16.safetensors"
wan_clip_url := "https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/resolve/main/split_files/text_encoders/umt5_xxl_fp8_e4m3fn_scaled.safetensors"
wan_vae_url := "https://huggingface.co/Comfy-Org/Wan_2.1_ComfyUI_repackaged/resolve/main/split_files/vae/wan_2.1_vae.safetensors"

# download Wan 2.1 1.3B + its text encoder and VAE (skip any already present)
video-model-wan:
    test -f {{ comfy_ws }}/models/diffusion_models/$(basename {{ wan_url }}) && echo "Wan diffusion model already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ wan_url }} --relative-path models/diffusion_models
    test -f {{ comfy_ws }}/models/text_encoders/$(basename {{ wan_clip_url }}) && echo "Wan text encoder already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ wan_clip_url }} --relative-path models/text_encoders
    test -f {{ comfy_ws }}/models/vae/$(basename {{ wan_vae_url }}) && echo "Wan VAE already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ wan_vae_url }} --relative-path models/vae

# queue a Wan text-to-video workflow (3s loop); render is slow, poll the output dir
video-test workflow="tests/ai/video-example-wan.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at {{ comfy_ws }}/output"


video-model-all: video-model-wan