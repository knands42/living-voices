use crate::provision::llama_server::{LlamaStep, run_llama_pipeline};

pub async fn provision_llama() -> i32 {
    let steps = [LlamaStep::Fetch, LlamaStep::Extract];

    run_llama_pipeline(&steps, |event| event.describe()).await
}

pub async fn provision_comfy_ui() -> i32 {
    todo!()
}
