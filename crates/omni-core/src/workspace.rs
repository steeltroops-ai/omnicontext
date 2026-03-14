//! Multi-repository workspace management.
//!
//! Enables linking multiple repositories into a unified search space.
//! Each repo has its own index, but queries can span all repos.
#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{OmniError, OmniResult};
use crate::pipeline::Engine;
use crate::types::SearchResult;

/// A workspace containing multiple linked repositories.
pub struct Workspace {
    /// Name of this workspace.
    name: String,
    /// Engines for each linked repository, keyed by repo path.
    engines: HashMap<PathBuf, Engine>,
    /// Priority weights per repo path, persisted alongside the config.
    priorities: HashMap<PathBuf, f32>,
    /// Config file path for this workspace.
    config_path: PathBuf,
}

/// Metadata for a linked repository.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LinkedRepo {
    /// Absolute path to the repository root.
    pub path: PathBuf,
    /// Optional alias for this repo (used in search results).
    pub alias: Option<String>,
    /// Whether to auto-index on workspace open.
    pub auto_index: bool,
    /// Search result priority weight [0.0, 1.0]. Higher values boost this
    /// repo's results in merged multi-repo search. Defaults to 0.5.
    #[serde(default = "default_priority")]
    pub priority: f32,
}

fn default_priority() -> f32 {
    0.5
}

/// Workspace configuration persisted to disk.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceConfig {
    /// Human-readable workspace name.
    pub name: String,
    /// Linked repositories.
    pub repos: Vec<LinkedRepo>,
}

impl Workspace {
    /// Create or open a workspace at the given config path.
    pub fn open(config_path: &Path) -> OmniResult<Self> {
        let ws_config = if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            toml::from_str::<WorkspaceConfig>(&content).map_err(|e| OmniError::Config {
                details: format!("invalid workspace config: {e}"),
            })?
        } else {
            WorkspaceConfig {
                name: "default".into(),
                repos: Vec::new(),
            }
        };

        let mut engines = HashMap::new();
        let mut priorities = HashMap::new();
        for repo in &ws_config.repos {
            priorities.insert(repo.path.clone(), repo.priority);
            if repo.path.exists() {
                match Engine::new(&repo.path) {
                    Ok(engine) => {
                        engines.insert(repo.path.clone(), engine);
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %repo.path.display(),
                            error = %e,
                            "failed to open repo, skipping"
                        );
                    }
                }
            }
        }

        Ok(Self {
            name: ws_config.name,
            engines,
            priorities,
            config_path: config_path.to_path_buf(),
        })
    }

    /// Link a new repository to this workspace with an optional priority weight.
    ///
    /// Returns `Err` if the path cannot be canonicalized, the engine fails to
    /// open, or the repo is already linked.
    pub fn link_repo(
        &mut self,
        path: &Path,
        alias: Option<String>,
        priority: f32,
    ) -> OmniResult<()> {
        let canonical = path.canonicalize().map_err(|e| {
            OmniError::Internal(format!("cannot resolve path {}: {e}", path.display()))
        })?;

        if self.engines.contains_key(&canonical) {
            return Err(OmniError::Config {
                details: format!("repo already linked: {}", canonical.display()),
            });
        }

        let engine = Engine::new(&canonical)?;
        self.engines.insert(canonical.clone(), engine);
        self.priorities
            .insert(canonical.clone(), priority.clamp(0.0, 1.0));

        // Persist immediately so the config is durable on crash.
        self.save_config()?;

        tracing::info!(
            path = %canonical.display(),
            alias = ?alias,
            priority,
            "linked repository to workspace"
        );

        Ok(())
    }

    /// Update the search priority weight for a linked repository.
    ///
    /// Returns `false` if the path is not currently linked.
    pub fn set_priority(&mut self, path: &Path, priority: f32) -> OmniResult<bool> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !self.engines.contains_key(&canonical) {
            return Ok(false);
        }
        self.priorities.insert(canonical, priority.clamp(0.0, 1.0));
        self.save_config()?;
        Ok(true)
    }

    /// Unlink a repository from this workspace.
    pub fn unlink_repo(&mut self, path: &Path) -> OmniResult<bool> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let removed = self.engines.remove(&canonical).is_some();
        self.priorities.remove(&canonical);
        if removed {
            self.save_config()?;
        }
        Ok(removed)
    }

    /// Return metadata for all linked repos sorted by priority descending.
    #[must_use]
    pub fn list_linked_repos(&self) -> Vec<LinkedRepo> {
        let mut repos: Vec<LinkedRepo> = self
            .engines
            .keys()
            .map(|p| LinkedRepo {
                path: p.clone(),
                alias: None,
                auto_index: true,
                priority: self
                    .priorities
                    .get(p)
                    .copied()
                    .unwrap_or(default_priority()),
            })
            .collect();
        // Highest priority first for deterministic ordering.
        repos.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.path.cmp(&b.path))
        });
        repos
    }

    /// Search across all linked repositories, boosting results from higher-priority repos.
    pub fn search(&self, query: &str, limit: usize) -> OmniResult<Vec<(PathBuf, SearchResult)>> {
        let per_repo_limit = limit * 2; // fetch more per repo, then merge + truncate
        let mut all_results: Vec<(PathBuf, SearchResult)> = Vec::new();

        for (repo_path, engine) in &self.engines {
            let boost = self
                .priorities
                .get(repo_path)
                .copied()
                .unwrap_or(default_priority());
            match engine.search(query, per_repo_limit) {
                Ok(results) => {
                    for mut r in results {
                        // Scale score by priority weight so higher-priority repos
                        // surface above lower-priority ones for equal relevance.
                        r.score *= f64::from(boost);
                        all_results.push((repo_path.clone(), r));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        repo = %repo_path.display(),
                        error = %e,
                        "search failed for repo"
                    );
                }
            }
        }

        // Sort by boosted score descending and truncate.
        all_results.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all_results.truncate(limit);

        Ok(all_results)
    }

    /// Get the number of linked repos.
    #[must_use]
    pub fn repo_count(&self) -> usize {
        self.engines.len()
    }

    /// Get the workspace name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get all linked repo paths.
    #[must_use]
    pub fn repo_paths(&self) -> Vec<&PathBuf> {
        self.engines.keys().collect()
    }

    /// Persist workspace config to disk.
    fn save_config(&self) -> OmniResult<()> {
        let config = WorkspaceConfig {
            name: self.name.clone(),
            repos: self
                .engines
                .keys()
                .map(|p| LinkedRepo {
                    path: p.clone(),
                    alias: None,
                    auto_index: true,
                    priority: self
                        .priorities
                        .get(p)
                        .copied()
                        .unwrap_or(default_priority()),
                })
                .collect(),
        };

        let content = toml::to_string_pretty(&config).map_err(|e| {
            OmniError::Internal(format!("failed to serialize workspace config: {e}"))
        })?;

        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.config_path, content)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_config_roundtrip() {
        let config = WorkspaceConfig {
            name: "test-workspace".into(),
            repos: vec![
                LinkedRepo {
                    path: PathBuf::from("/repo/a"),
                    alias: Some("frontend".into()),
                    auto_index: true,
                    priority: 0.8,
                },
                LinkedRepo {
                    path: PathBuf::from("/repo/b"),
                    alias: None,
                    auto_index: false,
                    priority: 0.5,
                },
            ],
        };

        let serialized = toml::to_string_pretty(&config).expect("serialize");
        let deserialized: WorkspaceConfig = toml::from_str(&serialized).expect("deserialize");

        assert_eq!(deserialized.name, "test-workspace");
        assert_eq!(deserialized.repos.len(), 2);
        assert_eq!(deserialized.repos[0].alias, Some("frontend".into()));
        assert!(deserialized.repos[0].auto_index);
        assert!(!deserialized.repos[1].auto_index);
        assert!((deserialized.repos[0].priority - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_workspace_open_nonexistent() {
        let dir = tempfile::tempdir().expect("tmp");
        let config_path = dir.path().join("workspace.toml");
        let ws = Workspace::open(&config_path).expect("open");
        assert_eq!(ws.repo_count(), 0);
        assert_eq!(ws.name(), "default");
    }
}
