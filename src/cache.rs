use crate::error::{Error, Result};
use crate::model::{Explanation, Request};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// A content-addressed cache of finished explanations.
///
/// The key covers the model as well as the question, so switching models or changing
/// the pipeline shape gives you a fresh answer instead of a stale one.
pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The per-user cache directory, or `None` if the OS will not tell us where it is.
    pub fn default_dir() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "plainly")
            .map(|dirs| dirs.cache_dir().join("explanations"))
    }

    pub fn key(request: &Request, model_id: &str, pipeline_tag: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"plainly/v1");
        hasher.update(model_id.as_bytes());
        hasher.update(pipeline_tag.as_bytes());
        hasher.update(request.level.to_string().as_bytes());
        hasher.update(request.query.trim().to_lowercase().as_bytes());
        hasher.update(request.context.as_deref().unwrap_or("").as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn path_for(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.json"))
    }

    pub fn get(&self, key: &str) -> Option<Explanation> {
        let bytes = std::fs::read(self.path_for(key)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn put(&self, key: &str, explanation: &Explanation) -> Result<()> {
        std::fs::create_dir_all(&self.dir).map_err(|source| Error::Io {
            path: self.dir.display().to_string(),
            source,
        })?;
        let path = self.path_for(key);
        let json = serde_json::to_vec_pretty(explanation).expect("Explanation is serializable");
        std::fs::write(&path, json).map_err(|source| Error::Io {
            path: path.display().to_string(),
            source,
        })
    }

    /// Delete every cached explanation. Returns how many were removed.
    pub fn clear(&self) -> Result<usize> {
        if !self.dir.exists() {
            return Ok(0);
        }
        let entries = std::fs::read_dir(&self.dir).map_err(|source| Error::Io {
            path: self.dir.display().to_string(),
            source,
        })?;
        let mut removed = 0;
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|ext| ext == "json")
                && std::fs::remove_file(entry.path()).is_ok()
            {
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

#[cfg(test)]
mod tests {
    use super::Cache;
    use crate::model::{Explanation, Level, Request};

    fn request(query: &str, level: Level) -> Request {
        Request {
            query: query.to_string(),
            context: None,
            level,
        }
    }

    #[test]
    fn key_ignores_case_and_surrounding_space() {
        let a = Cache::key(&request("CAP theorem", Level::Fifteen), "m", "p");
        let b = Cache::key(&request("  cap Theorem ", Level::Fifteen), "m", "p");
        assert_eq!(a, b);
    }

    #[test]
    fn key_changes_with_level_model_and_pipeline() {
        let base = Cache::key(&request("monads", Level::Fifteen), "m", "p");
        assert_ne!(base, Cache::key(&request("monads", Level::Five), "m", "p"));
        assert_ne!(
            base,
            Cache::key(&request("monads", Level::Fifteen), "other", "p")
        );
        assert_ne!(
            base,
            Cache::key(&request("monads", Level::Fifteen), "m", "no-critic")
        );
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path());
        let explanation = Explanation {
            topic: "CAP theorem".into(),
            level: Level::Fifteen,
            summary: "You cannot have it all.".into(),
            plain: "...".into(),
            analogy: "...".into(),
            analogy_limits: vec!["...".into()],
            example: None,
            corrections: vec![],
            caveats: vec![],
            citations: vec![],
        };

        assert!(cache.get("k").is_none());
        cache.put("k", &explanation).unwrap();
        assert_eq!(cache.get("k").unwrap().summary, explanation.summary);
        assert_eq!(cache.clear().unwrap(), 1);
        assert!(cache.get("k").is_none());
    }
}
