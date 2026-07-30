use super::Resolver;
use crate::error::Result;
use crate::llm::{parse_json, Llm, Prompt};
use crate::model::{Request, Topic};
use async_trait::async_trait;
use std::sync::Arc;

const SYSTEM: &str = "\
You disambiguate software engineering terms. Many terms mean different things in \
different fields - 'closure' in JavaScript is not 'closure' in mathematics, and \
'transaction' in a database is not 'transaction' in a message queue.

Pick the single reading the user most likely meant. If they supplied code context, let \
that decide. Reply with JSON only, no prose, using exactly this shape:

{
  \"canonical\": \"the concept's precise name, qualified by field if ambiguous\",
  \"domain\": \"the field it belongs to, e.g. distributed systems\",
  \"other_readings\": [\"readings you rejected, so the user can correct you\"]
}";

pub struct LlmResolver {
    llm: Arc<dyn Llm>,
}

impl LlmResolver {
    pub fn new(llm: Arc<dyn Llm>) -> Self {
        Self { llm }
    }
}

#[async_trait]
impl Resolver for LlmResolver {
    async fn resolve(&self, request: &Request) -> Result<Topic> {
        let mut user = format!("Term: {}", request.query);
        if let Some(context) = &request.context {
            user.push_str("\n\nThe user is asking about this code:\n");
            user.push_str(context);
        }

        let raw = self
            .llm
            .complete(&Prompt {
                stage: "resolve",
                system: SYSTEM.to_string(),
                user,
                json: true,
            })
            .await?;

        let mut topic: Topic = parse_json("resolve", &raw)?;
        topic.request = Some(Box::new(request.clone()));
        Ok(topic)
    }
}
