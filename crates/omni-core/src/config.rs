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
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            exclude_patterns: Self::default_excludes(),
            max_file_size: Self::default_max_file_size(),
            parse_concurrency: Self::default_parse_concurrency(),
            max_chunk_tokens: Self::default_max_chunk_tokens(),
            follow_symlinks: false,
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
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: Self::default_limit(),
            max_limit: Self::default_max_limit(),
            rrf_k: Self::default_rrf_k(),
            token_budget: Self::default_token_budget(),
        }
    }
}

impl SearchConfig {
    fn default_limit() -> usize { 10 }
    fn default_max_limit() -> usize { 100 }
    fn default_rrf_k() -> u32 { 60 }
    fn default_token_budget() -> u32 { 4000 }
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
        PathBuf::from("models/all-MiniLM-L6-v2.onnx")
    }
    fn default_dimensions() -> usize { 384 }
    fn default_batch_size() -> usize { 32 }
    fn default_max_seq_length() -> usize { 256 }
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
    fn repo_hash(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(self.repo_path.to_string_lossy().as_bytes());
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
        assert_eq!(config.embedding.dimensions, 384);
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
