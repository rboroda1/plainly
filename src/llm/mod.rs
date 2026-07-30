pub mod mock;
pub mod openai;

use crate::error::Result;
use async_trait::async_trait;

/// A single completion request. Every stage in the pipeline speaks in these terms,
/// which is what makes the model backend swappable.
#[derive(Debug, Clone)]
pub struct Prompt {
    /// Which pipeline stage produced this prompt. Used for error messages and mocking.
    pub stage: &'static str,
    pub system: String,
    pub user: String,
    /// Ask the provider to constrain output to a JSON object where supported.
    pub json: bool,
}

#[async_trait]
pub trait Llm: Send + Sync {
    async fn complete(&self, prompt: &Prompt) -> Result<String>;

    /// Identifier used in cache keys, so switching models invalidates cached answers.
    fn id(&self) -> String;
}

/// Models like to wrap JSON in ``` fences even when told not to. Strip them.
pub fn strip_fences(raw: &str) -> &str {
    let trimmed = raw.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    let rest = rest.strip_prefix("json").unwrap_or(rest);
    let rest = rest.trim_start_matches(['\r', '\n']);
    rest.strip_suffix("```").unwrap_or(rest).trim()
}

pub fn parse_json<T: serde::de::DeserializeOwned>(stage: &'static str, raw: &str) -> Result<T> {
    let cleaned = strip_fences(raw);
    serde_json::from_str(cleaned).map_err(|source| crate::error::Error::BadModelJson {
        stage,
        raw: raw.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::strip_fences;

    #[test]
    fn strips_json_fences() {
        assert_eq!(strip_fences("```json\n{\"a\":1}\n```"), "{\"a\":1}");
        assert_eq!(strip_fences("```\n{\"a\":1}\n```"), "{\"a\":1}");
        assert_eq!(strip_fences("  {\"a\":1}  "), "{\"a\":1}");
    }
}
