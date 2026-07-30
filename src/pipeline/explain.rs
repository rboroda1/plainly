use super::Explainer;
use crate::error::Result;
use crate::llm::{parse_json, Llm, Prompt};
use crate::model::{Explanation, Level, Request, Topic};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

const SYSTEM: &str = "\
You explain software engineering concepts in plain language. Simplicity is the goal, but \
accuracy is the constraint: it is always better to admit something is complicated than to \
say something false.

Rules:
- One analogy, not three. Make it concrete and everyday.
- State plainly where the analogy stops being true. This section is mandatory and must be \
  specific - 'it is not perfect' is useless.
- Prefer short sentences. Cut every word that is not doing work.
- Never invent APIs, papers, numbers, or history. If you are unsure, say so in the text.

Reply with JSON only, using exactly this shape:

{
  \"summary\": \"one sentence the reader could repeat from memory\",
  \"plain\": \"the explanation, 2-5 short paragraphs separated by \\n\\n\",
  \"analogy\": \"one concrete everyday analogy\",
  \"analogy_limits\": [\"specific ways the analogy misleads\"],
  \"example\": { \"language\": \"rust\", \"code\": \"...\", \"commentary\": \"what to notice\" }
}

The \"example\" field may be null when code would not help.";

#[derive(Deserialize)]
struct Draft {
    summary: String,
    plain: String,
    #[serde(default)]
    analogy: String,
    #[serde(default)]
    analogy_limits: Vec<String>,
    #[serde(default)]
    example: Option<crate::model::CodeExample>,
}

pub struct LlmExplainer {
    llm: Arc<dyn Llm>,
}

impl LlmExplainer {
    pub fn new(llm: Arc<dyn Llm>) -> Self {
        Self { llm }
    }
}

#[async_trait]
impl Explainer for LlmExplainer {
    async fn explain(&self, topic: &Topic, request: &Request) -> Result<Explanation> {
        let level: Level = request.level;
        let mut user = format!(
            "Concept: {}\nField: {}\n\nExplain it to {}.\n{}",
            topic.canonical,
            topic.domain,
            level.audience(),
            level.jargon_policy(),
        );
        if let Some(context) = &request.context {
            user.push_str("\n\nGround the explanation in this code:\n");
            user.push_str(context);
        }

        let raw = self
            .llm
            .complete(&Prompt {
                stage: "explain",
                system: SYSTEM.to_string(),
                user,
                json: true,
            })
            .await?;

        let draft: Draft = parse_json("explain", &raw)?;
        Ok(Explanation {
            topic: topic.canonical.clone(),
            level,
            summary: draft.summary,
            plain: draft.plain,
            analogy: draft.analogy,
            analogy_limits: draft.analogy_limits,
            example: draft.example,
            corrections: Vec::new(),
            caveats: Vec::new(),
            citations: Vec::new(),
        })
    }
}
