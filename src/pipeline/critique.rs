use super::Critic;
use crate::error::Result;
use crate::llm::{parse_json, Llm, Prompt};
use crate::model::{Explanation, Topic};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

const SYSTEM: &str = "\
You are a technical fact-checker reviewing a simplified explanation. Your job is to catch \
the places where simplifying made it wrong.

Look for:
- Claims that are flatly false.
- Claims that are true only in a common case but stated as universal.
- Analogies that will lead the reader to a wrong prediction about real behaviour.
- Invented APIs, papers, numbers, or history.
- Code that would not compile or run as written.

Repair what you can while keeping the language just as simple. Do not make the writing \
more academic - a fix that adds jargon is a bad fix. Anything you cannot verify goes in \
\"caveats\" rather than being deleted or asserted.

Reply with JSON only, using exactly this shape:

{
  \"summary\": \"corrected\",
  \"plain\": \"corrected\",
  \"analogy\": \"corrected\",
  \"analogy_limits\": [\"corrected and extended\"],
  \"example\": { \"language\": \"...\", \"code\": \"...\", \"commentary\": \"...\" },
  \"corrections\": [\"what you changed and why - empty list if nothing was wrong\"],
  \"caveats\": [\"claims you could not verify\"]
}";

#[derive(Deserialize)]
struct Reviewed {
    summary: String,
    plain: String,
    #[serde(default)]
    analogy: String,
    #[serde(default)]
    analogy_limits: Vec<String>,
    #[serde(default)]
    example: Option<crate::model::CodeExample>,
    #[serde(default)]
    corrections: Vec<String>,
    #[serde(default)]
    caveats: Vec<String>,
}

pub struct LlmCritic {
    llm: Arc<dyn Llm>,
}

impl LlmCritic {
    pub fn new(llm: Arc<dyn Llm>) -> Self {
        Self { llm }
    }
}

#[async_trait]
impl Critic for LlmCritic {
    async fn critique(&self, topic: &Topic, draft: Explanation) -> Result<Explanation> {
        let user = format!(
            "Concept: {}\nField: {}\nAudience level: {}\n\nDraft explanation:\n{}",
            topic.canonical,
            topic.domain,
            draft.level,
            serde_json::to_string_pretty(&draft).unwrap_or_default(),
        );

        let raw = self
            .llm
            .complete(&Prompt {
                stage: "critique",
                system: SYSTEM.to_string(),
                user,
                json: true,
            })
            .await?;

        let reviewed: Reviewed = parse_json("critique", &raw)?;
        Ok(Explanation {
            topic: draft.topic,
            level: draft.level,
            summary: reviewed.summary,
            plain: reviewed.plain,
            analogy: reviewed.analogy,
            analogy_limits: reviewed.analogy_limits,
            example: reviewed.example,
            corrections: reviewed.corrections,
            caveats: reviewed.caveats,
            citations: draft.citations,
        })
    }
}
