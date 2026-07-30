use super::Grounder;
use crate::error::Result;
use crate::llm::{parse_json, Llm, Prompt};
use crate::model::{Citation, Explanation, Topic};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

const SYSTEM: &str = "\
You attach sources to an explanation. Only cite documents you are confident exist: \
language references, RFCs, published papers, or official project documentation.

An empty list is a correct answer. A fabricated URL is a serious failure - if you are not \
sure of the address, give the title and set \"url\" to null.

Reply with JSON only, using exactly this shape:

{
  \"citations\": [
    { \"title\": \"...\", \"url\": \"... or null\", \"supports\": \"which claim this backs up\" }
  ]
}";

#[derive(Deserialize)]
struct Grounded {
    #[serde(default)]
    citations: Vec<Citation>,
}

pub struct LlmGrounder {
    llm: Arc<dyn Llm>,
}

impl LlmGrounder {
    pub fn new(llm: Arc<dyn Llm>) -> Self {
        Self { llm }
    }
}

#[async_trait]
impl Grounder for LlmGrounder {
    async fn ground(&self, topic: &Topic, draft: Explanation) -> Result<Explanation> {
        let user = format!(
            "Concept: {}\nField: {}\n\nExplanation:\n{}\n\n{}",
            topic.canonical, topic.domain, draft.plain, draft.summary,
        );

        let raw = self
            .llm
            .complete(&Prompt {
                stage: "ground",
                system: SYSTEM.to_string(),
                user,
                json: true,
            })
            .await?;

        let grounded: Grounded = parse_json("ground", &raw)?;
        Ok(Explanation {
            citations: grounded.citations,
            ..draft
        })
    }
}
