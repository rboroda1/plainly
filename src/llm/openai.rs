use super::{Llm, Prompt};
use crate::error::{Error, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

/// A client for any OpenAI-compatible `/chat/completions` endpoint.
///
/// That covers OpenAI, Azure OpenAI, GitHub Models, Groq, together.ai, and a local
/// Ollama or llama.cpp server - only `base_url` and `model` change.
pub struct OpenAiCompatible {
    http: reqwest::Client,
    base_url: String,
    model: String,
    api_key: String,
    temperature: f32,
}

impl OpenAiCompatible {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>, api_key: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            model: model.into(),
            api_key,
            temperature: 0.2,
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    #[serde(default)]
    content: String,
}

#[async_trait]
impl Llm for OpenAiCompatible {
    async fn complete(&self, prompt: &Prompt) -> Result<String> {
        let mut body = json!({
            "model": self.model,
            "temperature": self.temperature,
            "messages": [
                { "role": "system", "content": prompt.system },
                { "role": "user", "content": prompt.user },
            ],
        });
        if prompt.json {
            body["response_format"] = json!({ "type": "json_object" });
        }

        let response = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(Error::Api {
                provider: self.base_url.clone(),
                status: status.as_u16(),
                body: text,
            });
        }

        let parsed: ChatResponse = super::parse_json(prompt.stage, &text)?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| Error::Api {
                provider: self.base_url.clone(),
                status: status.as_u16(),
                body: "the provider returned no choices".to_string(),
            })
    }

    fn id(&self) -> String {
        format!("{}::{}", self.base_url, self.model)
    }
}
