use serde::{Deserialize, Serialize};
use std::fmt;

/// How deep the explanation should go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Explain it to a curious 5-year-old: pure intuition, no jargon.
    Five,
    /// Explain it to a sharp 15-year-old: intuition plus the real mechanics.
    Fifteen,
    /// Explain it to a working engineer: precise, with trade-offs and edge cases.
    Expert,
}

impl Level {
    pub fn audience(self) -> &'static str {
        match self {
            Level::Five => "a curious 5-year-old who knows nothing about computers",
            Level::Fifteen => "a sharp 15-year-old who has written a little code",
            Level::Expert => "a working software engineer who wants precision and trade-offs",
        }
    }

    pub fn jargon_policy(self) -> &'static str {
        match self {
            Level::Five => "Use zero jargon. Every word must be one the audience already knows.",
            Level::Fifteen => {
                "Introduce at most three technical terms, and define each one the first time it appears."
            }
            Level::Expert => {
                "Use correct technical vocabulary freely, but never hide behind it - each term must do real work."
            }
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Level::Five => "5",
            Level::Fifteen => "15",
            Level::Expert => "expert",
        };
        f.write_str(s)
    }
}

/// What the user asked for, before any model has looked at it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// The raw concept string, e.g. "CAP theorem".
    pub query: String,
    /// Optional extra context, e.g. a snippet of source code to ground the explanation in.
    pub context: Option<String>,
    pub level: Level,
}

/// The concept after disambiguation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Topic {
    /// The canonical name of the concept, e.g. "CAP theorem (distributed systems)".
    pub canonical: String,
    /// The field the concept belongs to, used to keep analogies on-domain.
    pub domain: String,
    /// Readings that were considered and rejected, so the user can correct us.
    #[serde(default)]
    pub other_readings: Vec<String>,
    #[serde(skip)]
    pub request: Option<Box<Request>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    pub language: String,
    pub code: String,
    pub commentary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub title: String,
    pub url: Option<String>,
    /// Which claim in the explanation this source backs up.
    pub supports: String,
}

/// The finished artifact. This is what `--json` prints, and what the cache stores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub topic: String,
    pub level: Level,
    /// One sentence a reader could repeat back from memory.
    pub summary: String,
    /// The main explanation, in plain language.
    pub plain: String,
    /// A single concrete analogy. Empty string means the explainer chose not to use one.
    #[serde(default)]
    pub analogy: String,
    /// Where the analogy stops being true. This is the whole point of the tool.
    #[serde(default)]
    pub analogy_limits: Vec<String>,
    #[serde(default)]
    pub example: Option<CodeExample>,
    /// Inaccuracies the critic found and fixed, kept for transparency.
    #[serde(default)]
    pub corrections: Vec<String>,
    /// Claims the critic could not verify. Read these with suspicion.
    #[serde(default)]
    pub caveats: Vec<String>,
    #[serde(default)]
    pub citations: Vec<Citation>,
}
