pub mod critique;
pub mod explain;
pub mod ground;
pub mod resolve;

use crate::error::Result;
use crate::llm::Llm;
use crate::model::{Explanation, Request, Topic};
use async_trait::async_trait;
use std::sync::Arc;

/// Stage 1: work out which concept the user actually meant.
#[async_trait]
pub trait Resolver: Send + Sync {
    async fn resolve(&self, request: &Request) -> Result<Topic>;
}

/// Stage 2: say it simply.
#[async_trait]
pub trait Explainer: Send + Sync {
    async fn explain(&self, topic: &Topic, request: &Request) -> Result<Explanation>;
}

/// Stage 3: attack the explanation for oversimplification, then repair it.
#[async_trait]
pub trait Critic: Send + Sync {
    async fn critique(&self, topic: &Topic, draft: Explanation) -> Result<Explanation>;
}

/// Stage 4 (optional): attach sources for the load-bearing claims.
#[async_trait]
pub trait Grounder: Send + Sync {
    async fn ground(&self, topic: &Topic, draft: Explanation) -> Result<Explanation>;
}

/// Called as each stage starts, so the CLI can show progress.
pub type StageHook = Box<dyn Fn(&'static str) + Send + Sync>;

pub struct Pipeline {
    resolver: Box<dyn Resolver>,
    explainer: Box<dyn Explainer>,
    critic: Option<Box<dyn Critic>>,
    grounder: Option<Box<dyn Grounder>>,
    on_stage: Option<StageHook>,
}

impl Pipeline {
    /// The default wiring: every stage backed by the same model.
    pub fn new(llm: Arc<dyn Llm>) -> Self {
        Self {
            resolver: Box::new(resolve::LlmResolver::new(llm.clone())),
            explainer: Box::new(explain::LlmExplainer::new(llm.clone())),
            critic: Some(Box::new(critique::LlmCritic::new(llm.clone()))),
            grounder: Some(Box::new(ground::LlmGrounder::new(llm))),
            on_stage: None,
        }
    }

    pub fn with_resolver(mut self, resolver: Box<dyn Resolver>) -> Self {
        self.resolver = resolver;
        self
    }

    pub fn with_explainer(mut self, explainer: Box<dyn Explainer>) -> Self {
        self.explainer = explainer;
        self
    }

    pub fn with_critic(mut self, critic: Option<Box<dyn Critic>>) -> Self {
        self.critic = critic;
        self
    }

    pub fn with_grounder(mut self, grounder: Option<Box<dyn Grounder>>) -> Self {
        self.grounder = grounder;
        self
    }

    pub fn on_stage(mut self, hook: StageHook) -> Self {
        self.on_stage = Some(hook);
        self
    }

    fn announce(&self, stage: &'static str) {
        if let Some(hook) = &self.on_stage {
            hook(stage);
        }
    }

    pub async fn run(&self, request: &Request) -> Result<Explanation> {
        self.announce("resolve");
        let topic = self.resolver.resolve(request).await?;

        self.announce("explain");
        let mut explanation = self.explainer.explain(&topic, request).await?;

        if let Some(critic) = &self.critic {
            self.announce("critique");
            explanation = critic.critique(&topic, explanation).await?;
        }

        if let Some(grounder) = &self.grounder {
            self.announce("ground");
            explanation = grounder.ground(&topic, explanation).await?;
        }

        Ok(explanation)
    }
}
