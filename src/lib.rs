//! plainly - explain any software engineering concept in plain language,
//! without losing accuracy.
//!
//! The library half is deliberately independent of the CLI: build a [`pipeline::Pipeline`],
//! hand it a [`model::Request`], and you get an [`model::Explanation`] back. Every stage is
//! a trait object, so a different model - or no model at all, in tests - can be swapped in.

pub mod cache;
pub mod error;
pub mod llm;
pub mod model;
pub mod pipeline;
pub mod render;

pub use error::{Error, Result};
pub use model::{Explanation, Level, Request};
