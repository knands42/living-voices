# -------------------- comfy (image) -------------------- #
# ComfyUI image models setup.
# Models: Flux.1 [dev] (Q4/Q8), Chroma, Qwen-Image — all GGUF, token-free.

# Flux.1 [dev] GGUF — top quality, slower. Q4 fits VRAM; Q8 offloads to RAM.
# Needs the ComfyUI-GGUF custom node + T5/CLIP encoders + VAE (all token-free).
flux_q4_url := "https://huggingface.co/city96/FLUX.1-dev-gguf/resolve/main/flux1-dev-Q4_K_S.gguf"
flux_q8_url := "https://huggingface.co/city96/FLUX.1-dev-gguf/resolve/main/flux1-dev-Q8_0.gguf"
flux_t5_url := "https://huggingface.co/city96/t5-v1_1-xxl-encoder-gguf/resolve/main/t5-v1_1-xxl-encoder-Q5_K_M.gguf"
flux_clip_url := "https://huggingface.co/comfyanonymous/flux_text_encoders/resolve/main/clip_l.safetensors"
flux_vae_url := "https://huggingface.co/Comfy-Org/Lumina_Image_2.0_Repackaged/resolve/main/split_files/vae/ae.safetensors"

# Chroma (Flux retrain, artistic/unfiltered) — reuses Flux T5 + VAE, new unet.
chroma_url := "https://huggingface.co/silveroxides/Chroma-GGUF/resolve/main/chroma-unlocked-v47-flash-heun/chroma-unlocked-v47-flash-heun-Q4_K_S.gguf"
# Qwen-Image (20B, top-tier + readable text) — its own encoder and VAE.
qwen_url := "https://huggingface.co/city96/Qwen-Image-gguf/resolve/main/qwen-image-Q4_K_S.gguf"
qwen_vae_url := "https://huggingface.co/Comfy-Org/Qwen-Image_ComfyUI/resolve/main/split_files/vae/qwen_image_vae.safetensors"
qwen_clip_url := "https://huggingface.co/Comfy-Org/Qwen-Image_ComfyUI/resolve/main/split_files/text_encoders/qwen_2.5_vl_7b_fp8_scaled.safetensors"

# install the ComfyUI-GGUF custom node (needed for every GGUF loader)
image-flux-node:
    test -d {{ comfy_ws }}/custom_nodes/ComfyUI-GGUF && echo "GGUF node already present" || git clone --depth 1 https://github.com/city96/ComfyUI-GGUF {{ comfy_ws }}/custom_nodes/ComfyUI-GGUF
    {{ comfy_venv }}/bin/pip install --upgrade gguf

# download Flux's shared support files: T5 + CLIP-L encoders and the VAE
# (also used by Chroma)
image-flux-support:
    test -f {{ comfy_ws }}/models/clip/$(basename {{ flux_t5_url }}) && echo "T5 already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ flux_t5_url }} --relative-path models/clip
    test -f {{ comfy_ws }}/models/clip/$(basename {{ flux_clip_url }}) && echo "CLIP-L already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ flux_clip_url }} --relative-path models/clip
    test -f {{ comfy_ws }}/models/vae/flux_ae.safetensors && echo "VAE already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ flux_vae_url }} --filename flux_ae.safetensors --relative-path models/vae

# Flux Q4 (~6.4GB, fits VRAM, ~80s/image on 8GB) — node + support + unet
image-model-flux-q4: image-flux-node image-flux-support
    test -f {{ comfy_ws }}/models/unet/$(basename {{ flux_q4_url }}) && echo "Flux Q4 already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ flux_q4_url }} --relative-path models/unet

# Flux Q8 (~12GB, offloads to RAM, higher quality) — node + support + unet
image-model-flux-q8: image-flux-node image-flux-support
    test -f {{ comfy_ws }}/models/unet/$(basename {{ flux_q8_url }}) && echo "Flux Q8 already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ flux_q8_url }} --relative-path models/unet

# Chroma (~5.3GB unet) — reuses Flux's GGUF node + T5 + VAE; ~3 min/image
image-model-chroma: image-flux-node image-flux-support
    test -f {{ comfy_ws }}/models/unet/$(basename {{ chroma_url }}) && echo "Chroma already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ chroma_url }} --relative-path models/unet

# Qwen-Image (20B, readable text) — GGUF node + its own encoder and VAE; ~3-4 min
image-model-qwen: image-flux-node
    test -f {{ comfy_ws }}/models/unet/$(basename {{ qwen_url }}) && echo "Qwen unet already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ qwen_url }} --relative-path models/unet
    test -f {{ comfy_ws }}/models/vae/$(basename {{ qwen_vae_url }}) && echo "Qwen VAE already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ qwen_vae_url }} --relative-path models/vae
    test -f {{ comfy_ws }}/models/text_encoders/$(basename {{ qwen_clip_url }}) && echo "Qwen encoder already present" || {{ comfy_bin }} --workspace {{ comfy_ws }} model download --url {{ qwen_clip_url }} --relative-path models/text_encoders

# download every model (Flux Q4 + Q8 + Chroma + Qwen)
image-models-all: image-model-flux-q4 image-model-flux-q8 image-model-chroma image-model-qwen

image-models:
    curl -sf {{ comfy_url }}/object_info/UnetLoaderGGUF | jq '.UnetLoaderGGUF.input.required.unet_name[0]'

image-test workflow="tests/ai/image-example-flux-q4.json":
    curl -s -X POST {{ comfy_url }}/prompt \
        -H 'Content-Type: application/json' \
        -d "$(jq -c '{prompt: .}' {{ workflow }})"
    @echo "Output will be generated at {{ comfy_ws }}/output"
