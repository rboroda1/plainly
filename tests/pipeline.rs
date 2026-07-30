//! End-to-end pipeline tests that never touch the network: the model is scripted.

use plainly::llm::mock::MockLlm;
use plainly::llm::Llm;
use plainly::model::{Level, Request};
use plainly::pipeline::Pipeline;
use std::sync::Arc;

const RESOLVE: &str = r#"{
  "canonical": "CAP theorem (distributed systems)",
  "domain": "distributed systems",
  "other_readings": ["CAP as in Common Agricultural Policy"]
}"#;

const EXPLAIN: &str = r#"{
  "summary": "When the network splits, you choose consistency or availability.",
  "plain": "A distributed database keeps copies of your data.\n\nWhen the machines cannot talk, each copy must either refuse to answer or risk answering with stale data.",
  "analogy": "Two shopkeepers sharing one notebook who lose their phone line.",
  "analogy_limits": ["Real partitions are partial and intermittent, not a clean cut."],
  "example": null
}"#;

const CRITIQUE: &str = r#"{
  "summary": "When the network splits, you choose consistency or availability.",
  "plain": "A distributed database keeps copies of your data.\n\nWhen the machines cannot talk, each copy must either refuse to answer or risk answering with stale data.",
  "analogy": "Two shopkeepers sharing one notebook who lose their phone line.",
  "analogy_limits": [
    "Real partitions are partial and intermittent, not a clean cut.",
    "The choice is per-operation, not one setting for the whole system."
  ],
  "example": null,
  "corrections": ["Dropped the claim that CA systems exist in practice."],
  "caveats": ["Vendor availability numbers were not verified."]
}"#;

const GROUND: &str = r#"{
  "citations": [
    {
      "title": "Brewer's Conjecture and the Feasibility of Consistent, Available, Partition-Tolerant Web Services",
      "url": null,
      "supports": "The formal statement of the trade-off."
    }
  ]
}"#;

fn request() -> Request {
    Request {
        query: "CAP theorem".to_string(),
        context: None,
        level: Level::Fifteen,
    }
}

#[tokio::test]
async fn full_pipeline_runs_every_stage_in_order() {
    let mock = Arc::new(MockLlm::new([RESOLVE, EXPLAIN, CRITIQUE, GROUND]));
    let pipeline = Pipeline::new(mock.clone() as Arc<dyn Llm>);

    let explanation = pipeline.run(&request()).await.unwrap();

    assert_eq!(
        mock.stages(),
        vec!["resolve", "explain", "critique", "ground"]
    );
    assert_eq!(explanation.topic, "CAP theorem (distributed systems)");
    assert_eq!(explanation.level, Level::Fifteen);
    assert_eq!(explanation.corrections.len(), 1);
    assert_eq!(explanation.caveats.len(), 1);
    assert_eq!(explanation.citations.len(), 1);
    // The critic's extra caveat must survive into the final artifact.
    assert_eq!(explanation.analogy_limits.len(), 2);
}

#[tokio::test]
async fn fast_mode_skips_critique_and_grounding() {
    let mock = Arc::new(MockLlm::new([RESOLVE, EXPLAIN]));
    let pipeline = Pipeline::new(mock.clone() as Arc<dyn Llm>)
        .with_critic(None)
        .with_grounder(None);

    let explanation = pipeline.run(&request()).await.unwrap();

    assert_eq!(mock.stages(), vec!["resolve", "explain"]);
    assert!(explanation.corrections.is_empty());
    assert!(explanation.citations.is_empty());
}

#[tokio::test]
async fn level_shapes_the_explain_prompt() {
    let mock = Arc::new(MockLlm::new([RESOLVE, EXPLAIN]));
    let pipeline = Pipeline::new(mock.clone() as Arc<dyn Llm>)
        .with_critic(None)
        .with_grounder(None);

    let mut request = request();
    request.level = Level::Five;
    pipeline.run(&request).await.unwrap();

    let explain_prompt = mock
        .prompts()
        .into_iter()
        .find(|prompt| prompt.stage == "explain")
        .expect("the explain stage ran");
    assert!(explain_prompt.user.contains("5-year-old"));
    assert!(explain_prompt.user.contains("zero jargon"));
}

#[tokio::test]
async fn code_context_reaches_both_resolve_and_explain() {
    let mock = Arc::new(MockLlm::new([RESOLVE, EXPLAIN]));
    let pipeline = Pipeline::new(mock.clone() as Arc<dyn Llm>)
        .with_critic(None)
        .with_grounder(None);

    let mut request = request();
    request.context = Some("fn takes_ownership(s: String) {}".to_string());
    pipeline.run(&request).await.unwrap();

    for prompt in mock.prompts() {
        assert!(
            prompt.user.contains("takes_ownership"),
            "stage '{}' lost the code context",
            prompt.stage
        );
    }
}

#[tokio::test]
async fn malformed_model_output_names_the_stage() {
    let mock = Arc::new(MockLlm::new([RESOLVE, "not json at all"]));
    let pipeline = Pipeline::new(mock as Arc<dyn Llm>)
        .with_critic(None)
        .with_grounder(None);

    let error = pipeline.run(&request()).await.unwrap_err();
    assert!(
        error.to_string().contains("explain"),
        "unhelpful error: {error}"
    );
}

#[tokio::test]
async fn fenced_json_is_tolerated() {
    let fenced = format!("```json\n{RESOLVE}\n```");
    let mock = Arc::new(MockLlm::new([fenced, EXPLAIN.to_string()]));
    let pipeline = Pipeline::new(mock as Arc<dyn Llm>)
        .with_critic(None)
        .with_grounder(None);

    let explanation = pipeline.run(&request()).await.unwrap();
    assert_eq!(explanation.topic, "CAP theorem (distributed systems)");
}
