use thiserror::Error;

/// Failures from talking to a local model endpoint.
#[derive(Error, Debug)]
pub enum AiError {
    #[error("could not reach the model endpoint at {url}: {source}")]
    Transport {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("model endpoint at {url} returned HTTP {status}: {body}")]
    Status {
        url: String,
        status: u16,
        body: String,
    },

    #[error("could not decode the response from {url}: {source}")]
    Decode {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("model `{model}` returned an empty completion")]
    EmptyCompletion { model: String },

    #[error("memory model did not return a usable memory record: {reply}")]
    MalformedMemory { reply: String },
}
