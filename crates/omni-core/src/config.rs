//! Configuration loading and validation.
//!
//! Configuration is resolved with the following precedence (highest wins):
//!
//! 1. CLI flags
//! 2. Environment variables (`OMNI_*`)
//! 3. Project config (`.omnicontext/config.toml`)
//! 4. User config (`~/.config/omnicontext/config.toml`)
//! 5. Compiled-in defaults

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{OmniError, OmniResult};

/// Top-level configuration for OmniContext.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Repository root path to index.
    pub repo_path: PathBuf,

    /// Indexing configuration.
    #[serde(default)]
    pub indexing: IndexingConfig,

    /// Search configuration.
    #[serde(default)]
    pub search: SearchConfig,

    /// Embedding configuration.
    #[serde(default)]
    pub embedding: EmbeddingConfig,

    /// Watcher configuration.
    #[serde(default)]
    pub watcher: WatcherConfig,

    /// Logging configuration.
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Indexing-specific settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// File patterns to exclude from indexing (glob syntax).
    #[serde(default = "IndexingConfig::default_excludes")]
    pub exclude_patterns: Vec<String>,

    /// Maximum file size to index (in bytes). Files larger than this are skipped.
    #[serde(default = "IndexingConfig::default_max_file_size")]
    pub max_file_size: u64,

    /// Maximum number of concurrent parse tasks.
    #[serde(default = "IndexingConfig::default_parse_concurrency")]
    pub parse_concurrency: usize,

    /// Maximum chunk size in tokens.
    #[serde(default = "IndexingConfig::default_max_chunk_tokens")]
    pub max_chunk_tokens: u32,

    /// Whether to follow symbolic links.
    #[serde(default)]
    pub follow_symlinks: bool,

    /// Number of backward overlap lines to include before each chunk for CAST context.
    /// These lines provide surrounding context to prevent orphaned chunks.
    #[serde(default = "IndexingConfig::default_overlap_lines")]
    pub overlap_lines: usize,

    /// Target overlap in tokens for CAST context windowing.
    /// When set, takes precedence over `overlap_lines` for determining
    /// how much backward context to capture.
    #[serde(default = "IndexingConfig::default_overlap_tokens")]
    pub overlap_tokens: u32,

    /// Overlap fraction for intra-element splitting (0.0 - 0.5).
    /// Controls how much content is repeated between consecutive chunks
    /// when a single large element is split into multiple chunks.
    #[serde(default = "IndexingConfig::default_overlap_fraction")]
    pub overlap_fraction: f64,

    /// Whether to include module-level declarations (imports, top-level constants,
    /// type definitions) in each chunk's context header regardless of their distance.
    #[serde(default = "IndexingConfig::default_include_module_declarations")]
    pub include_module_declarations: bool,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            exclude_patterns: Self::default_excludes(),
            max_file_size: Self::default_max_file_size(),
            parse_concurrency: Self::default_parse_concurrency(),
            max_chunk_tokens: Self::default_max_chunk_tokens(),
            follow_symlinks: false,
            overlap_lines: Self::default_overlap_lines(),
            overlap_tokens: Self::default_overlap_tokens(),
            overlap_fraction: Self::default_overlap_fraction(),
            include_module_declarations: Self::default_include_module_declarations(),
        }
    }
}

impl IndexingConfig {
    fn default_excludes() -> Vec<String> {
        vec![
            ".git".into(),
            "node_modules".into(),
            "target".into(),
            "__pycache__".into(),
            ".venv".into(),
            "venv".into(),
            "dist".into(),
            "build".into(),
            ".next".into(),
            "*.lock".into(),
            "*.min.js".into(),
            "*.min.css".into(),
            "*.map".into(),
        ]
    }

    fn default_max_file_size() -> u64 {
        5 * 1024 * 1024 // 5MB
    }

    fn default_parse_concurrency() -> usize {
        2
    }

    fn default_max_chunk_tokens() -> u32 {
        512
    }

    fn default_overlap_lines() -> usize { 10 }

    fn default_overlap_tokens() -> u32 { 150 }

    fn default_overlap_fraction() -> f64 { 0.12 }

    fn default_include_module_declarations() -> bool { true }
}

/// Search-specific settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Default number of results to return.
    #[serde(default = "SearchConfig::default_limit")]
    pub default_limit: usize,

    /// Maximum number of results to return.
    #[serde(default = "SearchConfig::default_max_limit")]
    pub max_limit: usize,

    /// RRF constant (k parameter).
    #[serde(default = "SearchConfig::default_rrf_k")]
    pub rrf_k: u32,

    /// Default token budget for context building.
    #[serde(default = "SearchConfig::default_token_budget")]
    pub token_budget: u32,

    /// Reranker configuration.
    #[serde(default)]
    pub reranker: RerankerConfig,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: Self::default_limit(),
            max_limit: Self::default_max_limit(),
            rrf_k: Self::default_rrf_k(),
            token_budget: Self::default_token_budget(),
            reranker: RerankerConfig::default(),
        }
    }
}

impl SearchConfig {
    fn default_limit() -> usize { 10 }
    fn default_max_limit() -> usize { 100 }
    fn default_rrf_k() -> u32 { 60 }
    fn default_token_budget() -> u32 { 4000 }
}

/// Cross-encoder reranker configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerConfig {
    /// Weight given to the original RRF score when blending with reranker (0.0 - 1.0).
    /// The reranker weight is `1.0 - rrf_weight`.
    #[serde(default = "RerankerConfig::default_rrf_weight")]
    pub rrf_weight: f64,

    /// Maximum number of candidates to pass to the reranker.
    #[serde(default = "RerankerConfig::default_max_candidates")]
    pub max_candidates: usize,

    /// Batch size for reranker inference.
    #[serde(default = "RerankerConfig::default_batch_size")]
    pub batch_size: usize,

    /// Maximum sequence length for the reranker tokenizer.
    #[serde(default = "RerankerConfig::default_max_seq_length")]
    pub max_seq_length: usize,

    /// Demotion factor applied to items not scored by the reranker (0.0 - 1.0).
    /// Items beyond `max_candidates` have their score multiplied by this factor.
    #[serde(default = "RerankerConfig::default_unranked_demotion")]
    pub unranked_demotion: f64,
}

impl Default for RerankerConfig {
    fn default() -> Self {
        Self {
            rrf_weight: Self::default_rrf_weight(),
            max_candidates: Self::default_max_candidates(),
            batch_size: Self::default_batch_size(),
            max_seq_length: Self::default_max_seq_length(),
            unranked_demotion: Self::default_unranked_demotion(),
        }
    }
}

impl RerankerConfig {
    fn default_rrf_weight() -> f64 { 0.35 }
    fn default_max_candidates() -> usize { 100 }
    fn default_batch_size() -> usize { 16 }
    fn default_max_seq_length() -> usize { 512 }
    fn default_unranked_demotion() -> f64 { 0.5 }
}

/// Embedding model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Path to the ONNX model file.
    #[serde(default = "EmbeddingConfig::default_model_path")]
    pub model_path: PathBuf,

    /// Output embedding dimensions.
    #[serde(default = "EmbeddingConfig::default_dimensions")]
    pub dimensions: usize,

    /// Batch size for embedding inference.
    #[serde(default = "EmbeddingConfig::default_batch_size")]
    pub batch_size: usize,

    /// Maximum sequence length for the tokenizer.
    #[serde(default = "EmbeddingConfig::default_max_seq_length")]
    pub max_seq_length: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_path: Self::default_model_path(),
            dimensions: Self::default_dimensions(),
            batch_size: Self::default_batch_size(),
            max_seq_length: Self::default_max_seq_length(),
        }
    }
}

impl EmbeddingConfig {
    fn default_model_path() -> PathBuf {
        // Default: auto-download cache location for jina-embeddings-v2-base-code.
        // If the model isn't here yet, the embedder will auto-download it.
        // Users can override via config or OMNI_MODEL_PATH env var.
        crate::embedder::model_manager::model_path(&crate::embedder::model_manager::DEFAULT_MODEL)
    }
    fn default_dimensions() -> usize { 768 } // jina-code v2 output dimensions
    fn default_batch_size() -> usize { 32 }
    fn default_max_seq_length() -> usize { 512 } // practical limit for code chunks
}

/// File watcher configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// Debounce interval in milliseconds.
    #[serde(default = "WatcherConfig::default_debounce_ms")]
    pub debounce_ms: u64,

    /// Interval between full scans (in seconds) for catching missed events.
    #[serde(default = "WatcherConfig::default_poll_interval_secs")]
    pub poll_interval_secs: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: Self::default_debounce_ms(),
            poll_interval_secs: Self::default_poll_interval_secs(),
        }
    }
}

impl WatcherConfig {
    fn default_debounce_ms() -> u64 { 100 }
    fn default_poll_interval_secs() -> u64 { 300 }
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level filter (e.g., "info", "debug", "trace").
    #[serde(default = "LoggingConfig::default_level")]
    pub level: String,

    /// Whether to output logs as JSON.
    #[serde(default)]
    pub json: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            json: false,
        }
    }
}

impl LoggingConfig {
    fn default_level() -> String {
        "info".into()
    }
}

impl Config {
    /// Load configuration from defaults, then overlay user config, then project config.
    pub fn load(repo_path: &Path) -> OmniResult<Self> {
        let mut config = Self::defaults(repo_path);

        // User config: ~/.config/omnicontext/config.toml
        if let Some(user_config_dir) = dirs::config_dir() {
            let user_config_path = user_config_dir.join("omnicontext").join("config.toml");
            if user_config_path.exists() {
                config.merge_from_file(&user_config_path)?;
            }
        }

        // Project config: <repo>/.omnicontext/config.toml
        let project_config_path = repo_path.join(".omnicontext").join("config.toml");
        if project_config_path.exists() {
            config.merge_from_file(&project_config_path)?;
        }

        // Environment overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Create a default configuration for the given repo path.
    pub fn defaults(repo_path: &Path) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
            indexing: IndexingConfig::default(),
            search: SearchConfig::default(),
            embedding: EmbeddingConfig::default(),
            watcher: WatcherConfig::default(),
            logging: LoggingConfig::default(),
        }
    }

    /// Returns the data directory for this repo's index files.
    pub fn data_dir(&self) -> PathBuf {
        let hash = self.repo_hash();
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("omnicontext")
            .join("repos")
            .join(&hash);
        base
    }

    /// Merge values from a TOML config file (non-destructive overlay).
    fn merge_from_file(&mut self, path: &Path) -> OmniResult<()> {
        let content = std::fs::read_to_string(path)?;
        let overlay: toml::Value = toml::from_str(&content)
            .map_err(|e| OmniError::Config { details: format!("invalid TOML in {}: {e}", path.display()) })?;

        // Override individual sections if present
        if let Some(indexing) = overlay.get("indexing") {
            if let Ok(parsed) = indexing.clone().try_into::<IndexingConfig>() {
                self.indexing = parsed;
            }
        }
        if let Some(search) = overlay.get("search") {
            if let Ok(parsed) = search.clone().try_into::<SearchConfig>() {
                self.search = parsed;
            }
        }
        if let Some(embedding) = overlay.get("embedding") {
            if let Ok(parsed) = embedding.clone().try_into::<EmbeddingConfig>() {
                self.embedding = parsed;
            }
        }
        if let Some(watcher) = overlay.get("watcher") {
            if let Ok(parsed) = watcher.clone().try_into::<WatcherConfig>() {
                self.watcher = parsed;
            }
        }
        if let Some(logging) = overlay.get("logging") {
            if let Ok(parsed) = logging.clone().try_into::<LoggingConfig>() {
                self.logging = parsed;
            }
        }

        Ok(())
    }

    /// Apply environment variable overrides (OMNI_* prefix).
    fn apply_env_overrides(&mut self) {
        if let Ok(level) = std::env::var("OMNI_LOG_LEVEL") {
            self.logging.level = level;
        }
        if let Ok(model) = std::env::var("OMNI_MODEL_PATH") {
            self.embedding.model_path = PathBuf::from(model);
        }
    }

    /// Compute a short hash of the repo path for the data directory name.
    ///
    /// Normalizes the path to avoid Windows `\\?\` extended path prefix
    /// causing different hashes for the same physical directory.
    fn repo_hash(&self) -> String {
        use sha2::{Sha256, Digest};
        let path_str = self.repo_path.to_string_lossy();
        // Strip Windows extended path prefix for consistent hashing
        let normalized = path_str
            .strip_prefix(r"\\?\")
            .unwrap_or(&path_str);
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        let result = hasher.finalize();
        hex::encode(&result[..4])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = Config::defaults(Path::new("/tmp/test-repo"));
        assert_eq!(config.indexing.max_file_size, 5 * 1024 * 1024);
        assert_eq!(config.search.default_limit, 10);
        assert_eq!(config.embedding.dimensions, 768);
        assert_eq!(config.watcher.debounce_ms, 100);
    }

    #[test]
    fn test_language_from_extension() {
        use crate::types::Language;
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
        assert_eq!(Language::from_extension("go"), Language::Go);
        assert_eq!(Language::from_extension("xyz"), Language::Unknown);
    }

    #[test]
    fn test_chunk_kind_weights() {
        use crate::types::ChunkKind;
        assert!(ChunkKind::Class.default_weight() > ChunkKind::Test.default_weight());
        assert!(ChunkKind::Function.default_weight() > ChunkKind::TopLevel.default_weight());
    }
}
