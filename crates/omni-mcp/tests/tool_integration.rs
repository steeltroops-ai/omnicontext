//! Integration tests for MCP tool handlers.
//!
//! These tests create a real Engine backed by a tempdir, index test files,
//! and then exercise each MCP tool to verify correct behavior.
//!
//! Uses multi_thread flavor because Engine internally uses blocking I/O
//! (SQLite, file I/O) that must not run on the async executor.
//!
//! OMNI_SKIP_MODEL_DOWNLOAD is set to prevent the 550MB model download
//! during tests. The engine operates in keyword-only mode.

use omni_core::Engine;
use std::io::Write;
use std::sync::Once;
use tempfile::TempDir;

static INIT: Once = Once::new();

/// Ensure env vars are set before any test runs.
fn init() {
    INIT.call_once(|| {
        std::env::set_var("OMNI_SKIP_MODEL_DOWNLOAD", "1");
    });
}

/// Create a test engine with some sample files in a tempdir.
/// Engine creation must happen on a blocking thread because it
/// internally initializes SQLite and other blocking resources.
async fn create_test_engine() -> (Engine, TempDir) {
    init();
    let dir = TempDir::new().expect("create temp dir");
    let dir_path = dir.path().to_path_buf();

    // Create sample Python file
    let py_path = dir_path.join("auth.py");
    let mut f = std::fs::File::create(&py_path).unwrap();
    writeln!(
        f,
        r#""""Authentication module."""

class AuthService:
    """Handles user authentication and authorization."""

    def __init__(self, db):
        self.db = db
        self.token_expiry = 3600

    def validate_token(self, token: str) -> bool:
        """Validate a JWT token.

        Args:
            token: The JWT token string to validate.

        Returns:
            True if the token is valid, False otherwise.
        """
        if not token:
            return False
        return self.db.check_token(token)

    def login(self, username: str, password: str) -> str:
        """Authenticate a user and return a session token."""
        user = self.db.find_user(username)
        if user and user.check_password(password):
            return self.db.create_token(user)
        raise ValueError("Invalid credentials")

def create_auth_middleware(config):
    """Factory function for auth middleware."""
    return AuthService(config.db)
"#
    )
    .unwrap();

    // Create sample Rust file
    let rs_path = dir_path.join("config.rs");
    let mut f = std::fs::File::create(&rs_path).unwrap();
    writeln!(
        f,
        r#"//! Configuration loading.

use std::path::PathBuf;

/// Application configuration.
pub struct Config {{
    pub db_path: PathBuf,
    pub port: u16,
    pub debug: bool,
}}

impl Config {{
    /// Create default configuration.
    pub fn default() -> Self {{
        Self {{
            db_path: PathBuf::from("data.db"),
            port: 8080,
            debug: false,
        }}
    }}

    /// Load from environment variables.
    pub fn from_env() -> Result<Self, String> {{
        Ok(Self {{
            db_path: std::env::var("DB_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("data.db")),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            debug: std::env::var("DEBUG").is_ok(),
        }})
    }}
}}
"#
    )
    .unwrap();

    // Create the engine on a blocking thread to avoid nested runtime issues
    let engine =
        tokio::task::spawn_blocking(move || Engine::new(&dir_path).expect("create engine"))
            .await
            .expect("spawn_blocking join");

    (engine, dir)
}

/// Create an engine and run indexing on the test files.
async fn create_indexed_engine() -> (Engine, TempDir) {
    let (mut engine, dir) = create_test_engine().await;
    engine.run_index().await.expect("index test files");
    (engine, dir)
}

// ---- Tests ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_engine_indexes_test_files() {
    let (engine, _dir) = create_indexed_engine().await;
    let status = engine.status().expect("get status");
    assert!(
        status.files_indexed >= 2,
        "should index at least 2 files, got {}",
        status.files_indexed
    );
    assert!(status.chunks_indexed > 0, "should create chunks");
    assert!(status.symbols_indexed > 0, "should extract symbols");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_search_finds_relevant_code() {
    let (engine, _dir) = create_indexed_engine().await;

    let results = engine.search("authentication", 5).expect("search");
    assert!(
        !results.is_empty(),
        "should find results for 'authentication'"
    );

    // The auth.py file should be in the results
    let has_auth_file = results
        .iter()
        .any(|r| r.file_path.to_string_lossy().contains("auth"));
    assert!(has_auth_file, "should find auth.py in results");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_search_by_symbol_name() {
    let (engine, _dir) = create_indexed_engine().await;

    let results = engine.search("validate_token", 5).expect("search");
    assert!(
        !results.is_empty(),
        "should find results for 'validate_token'"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_search_empty_query() {
    let (engine, _dir) = create_indexed_engine().await;

    // Empty query should not crash
    let results = engine.search("", 5).expect("empty search");
    let _ = results;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_search_no_match() {
    let (engine, _dir) = create_indexed_engine().await;

    let results = engine
        .search("xyzzy_nonexistent_symbol_12345", 5)
        .expect("search");
    assert!(
        results.is_empty(),
        "should find no results for gibberish query"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_status_reports_correct_counts() {
    let (engine, _dir) = create_indexed_engine().await;
    let status = engine.status().expect("get status");

    assert_eq!(status.search_mode, "keyword-only"); // no ONNX model in tests
    assert!(status.files_indexed >= 2);
    assert!(status.chunks_indexed > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_status_on_empty_engine() {
    let (engine, _dir) = create_test_engine().await;
    let status = engine.status().expect("get status");

    assert_eq!(status.files_indexed, 0);
    assert_eq!(status.chunks_indexed, 0);
    assert_eq!(status.symbols_indexed, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_index_produces_symbols() {
    let (engine, _dir) = create_indexed_engine().await;
    let status = engine.status().expect("get status");

    // Should extract symbols like AuthService, validate_token, Config, etc.
    assert!(
        status.symbols_indexed >= 3,
        "should extract at least 3 symbols, got {}",
        status.symbols_indexed
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_reindex_is_idempotent() {
    let (mut engine, _dir) = create_indexed_engine().await;
    let status1 = engine.status().expect("first status");

    // Re-index -- should not duplicate
    engine.run_index().await.expect("re-index");
    let status2 = engine.status().expect("second status");

    assert_eq!(
        status1.files_indexed, status2.files_indexed,
        "re-index should not duplicate files"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_search_respects_limit() {
    let (engine, _dir) = create_indexed_engine().await;

    let results = engine.search("token", 1).expect("search with limit 1");
    assert!(results.len() <= 1, "should respect limit of 1");
}
