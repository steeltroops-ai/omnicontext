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
        for repo in &ws_config.repos {
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
            config_path: config_path.to_path_buf(),
        })
    }

    /// Link a new repository to this workspace.
    pub fn link_repo(&mut self, path: &Path, alias: Option<String>) -> OmniResult<()> {
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

        // Persist
        self.save_config()?;

        tracing::info!(
            path = %canonical.display(),
            alias = ?alias,
            "linked repository to workspace"
        );

        Ok(())
    }

    /// Unlink a repository from this workspace.
    pub fn unlink_repo(&mut self, path: &Path) -> OmniResult<bool> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let removed = self.engines.remove(&canonical).is_some();
        if removed {
            self.save_config()?;
        }
        Ok(removed)
    }

    /// Search across all linked repositories.
    pub fn search(&self, query: &str, limit: usize) -> OmniResult<Vec<(PathBuf, SearchResult)>> {
        let per_repo_limit = limit * 2; // fetch more per repo, then merge
        let mut all_results: Vec<(PathBuf, SearchResult)> = Vec::new();

        for (repo_path, engine) in &self.engines {
            match engine.search(query, per_repo_limit) {
                Ok(results) => {
                    for r in results {
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

        // Sort by score descending and truncate
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
                },
                LinkedRepo {
                    path: PathBuf::from("/repo/b"),
                    alias: None,
                    auto_index: false,
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
