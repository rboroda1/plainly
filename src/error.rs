use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no API key found: set PLAINLY_API_KEY (or pass --api-key)")]
    MissingApiKey,

    #[error("the {provider} API returned {status}: {body}")]
    Api {
        provider: String,
        status: u16,
        body: String,
    },

    #[error("could not reach the model provider")]
    Transport(#[from] reqwest::Error),

    #[error("the model replied with something that was not valid JSON for stage '{stage}': {source}\n---\n{raw}\n---")]
    BadModelJson {
        stage: &'static str,
        raw: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("could not read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("{0}")]
    Input(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
