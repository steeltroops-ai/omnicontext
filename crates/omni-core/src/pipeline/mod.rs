//! Pipeline orchestrator.
//!
//! Wires together all subsystems into a coherent indexing + query engine.
//! This is the top-level public API of omni-core.
//!
//! ## Responsibilities
//!
//! - Initialize all subsystems (config, index, vector, graph, embedder, watcher)
//! - Run the indexing pipeline (watcher -> parser -> chunker -> embedder -> store)
//! - Handle search queries (delegating to SearchEngine)
//! - Manage graceful shutdown

use std::path::Path;

use crate::config::Config;
use crate::error::OmniResult;
use crate::types::SearchResult;

/// The main OmniContext engine.
///
/// This is the primary entry point for the library. It owns all subsystems
/// and coordinates their lifecycle.
pub struct Engine {
    config: Config,
    // index: MetadataIndex,
    // vector: VectorIndex,
    // graph: DependencyGraph,
    // embedder: Embedder,
    // search: SearchEngine,
}

impl Engine {
    /// Create a new engine for the given repository.
    pub fn new(repo_path: &Path) -> OmniResult<Self> {
        let config = Config::load(repo_path)?;

        tracing::info!(
            repo = %repo_path.display(),
            data_dir = %config.data_dir().display(),
            "engine initialized"
        );

        Ok(Self {
            config,
        })
    }

    /// Create an engine with explicit configuration (for testing).
    pub fn with_config(config: Config) -> OmniResult<Self> {
        Ok(Self { config })
    }

    /// Start the indexing pipeline.
    ///
    /// This performs an initial full scan, then watches for changes.
    pub async fn start(&self) -> OmniResult<()> {
        tracing::info!("starting indexing pipeline");
        // TODO: Wire up watcher -> parser -> chunker -> embedder -> store
        Ok(())
    }

    /// Execute a search query.
    pub async fn search(&self, query: &str, limit: usize) -> OmniResult<Vec<SearchResult>> {
        let _ = (query, limit);
        // TODO: Delegate to SearchEngine
        Ok(Vec::new())
    }

    /// Get engine status information.
    pub fn status(&self) -> EngineStatus {
        EngineStatus {
            repo_path: self.config.repo_path.display().to_string(),
            data_dir: self.config.data_dir().display().to_string(),
            files_indexed: 0,   // TODO
            chunks_indexed: 0,  // TODO
            symbols_indexed: 0, // TODO
            vectors_indexed: 0, // TODO
            search_mode: if true { "keyword-only" } else { "hybrid" }.into(), // TODO
        }
    }

    /// Shut down the engine gracefully.
    pub async fn shutdown(&self) -> OmniResult<()> {
        tracing::info!("engine shutting down");
        // TODO: Signal watcher to stop, drain channels, flush index
        Ok(())
    }
}

/// Status information about the engine.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EngineStatus {
    /// Repository path being indexed.
    pub repo_path: String,
    /// Data directory for index files.
    pub data_dir: String,
    /// Number of files in the index.
    pub files_indexed: usize,
    /// Number of chunks in the index.
    pub chunks_indexed: usize,
    /// Number of symbols in the index.
    pub symbols_indexed: usize,
    /// Number of vectors in the index.
    pub vectors_indexed: usize,
    /// Current search mode (hybrid or keyword-only).
    pub search_mode: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_status_default() {
        let config = Config::defaults(Path::new("/tmp/test-repo"));
        let engine = Engine::with_config(config).expect("create engine");
        let status = engine.status();
        assert_eq!(status.files_indexed, 0);
    }
}
