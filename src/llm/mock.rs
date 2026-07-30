use super::{Llm, Prompt};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Mutex;

/// A scripted `Llm` for tests: hand it replies, and it returns them in order while
/// recording every prompt it was given.
pub struct MockLlm {
    replies: Mutex<VecDeque<String>>,
    seen: Mutex<Vec<Prompt>>,
}

impl MockLlm {
    pub fn new<I, S>(replies: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            replies: Mutex::new(replies.into_iter().map(Into::into).collect()),
            seen: Mutex::new(Vec::new()),
        }
    }

    /// Stages, in the order they were called.
    pub fn stages(&self) -> Vec<&'static str> {
        self.seen.lock().unwrap().iter().map(|p| p.stage).collect()
    }

    pub fn prompts(&self) -> Vec<Prompt> {
        self.seen.lock().unwrap().clone()
    }
}

#[async_trait]
impl Llm for MockLlm {
    async fn complete(&self, prompt: &Prompt) -> Result<String> {
        self.seen.lock().unwrap().push(prompt.clone());
        self.replies.lock().unwrap().pop_front().ok_or_else(|| {
            Error::Input(format!(
                "MockLlm ran out of replies at stage '{}'",
                prompt.stage
            ))
        })
    }

    fn id(&self) -> String {
        "mock".to_string()
    }
}
