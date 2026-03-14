//! Token counting abstraction for accurate chunk budget management.
//!
//! Provides a trait-based approach with two implementations:
//! - `ActualTokenCounter`: Uses the HuggingFace `tokenizers` crate for
//!   exact BPE/WordPiece token counts. Required for production accuracy.
//! - `EstimateTokenCounter`: Heuristic estimator (chars / 4) for environments
//!   without a loaded tokenizer (CI, unit tests, fallback).
//!
//! ## Why not just pass `&Tokenizer` directly?
//!
//! A trait abstraction is more extensible:
//! 1. Tests run without downloading a 500MB model
//! 2. CI environments can use the estimator with `OMNI_SKIP_MODEL_DOWNLOAD=1`
//! 3. Future token counters (tiktoken, sentencepiece) slot in seamlessly
//! 4. The estimator remains available as a zero-cost fallback

use std::sync::Arc;

/// Trait for counting tokens in text content.
///
/// Implementations MUST be deterministic: the same input produces
/// the same count on every call. Thread-safety via `Send + Sync` is
/// required for concurrent chunking across files.
pub trait TokenCounter: Send + Sync {
    /// Count the number of tokens in the given text.
    ///
    /// Returns a count >= 1 (empty strings still consume at least 1 token
    /// because the embedder always produces at least a [CLS] token).
    fn count(&self, text: &str) -> u32;

    /// Human-readable name of this counter implementation (for logging).
    fn name(&self) -> &'static str;
}

/// Accurate token counter backed by a HuggingFace tokenizer.
///
/// Uses the same tokenizer that the embedding model uses, ensuring chunk
/// sizes exactly match model input constraints. This eliminates the
/// silent truncation bug where chunks exceed model limits.
pub struct ActualTokenCounter {
    tokenizer: Arc<tokenizers::Tokenizer>,
}

impl ActualTokenCounter {
    /// Create a new counter from a loaded tokenizer.
    pub fn new(tokenizer: Arc<tokenizers::Tokenizer>) -> Self {
        Self { tokenizer }
    }

    /// Create from a tokenizer file path.
    ///
    /// Returns `None` if the file doesn't exist or can't be loaded.
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let tokenizer = tokenizers::Tokenizer::from_file(path).ok()?;
        Some(Self {
            tokenizer: Arc::new(tokenizer),
        })
    }
}

impl TokenCounter for ActualTokenCounter {
    fn count(&self, text: &str) -> u32 {
        if text.is_empty() {
            return 1;
        }
        // encode_fast avoids special token handling overhead.
        // We only care about the count, not the actual token IDs.
        match self.tokenizer.encode_fast(text, false) {
            Ok(encoding) => {
                let len = encoding.len();
                if len == 0 {
                    1
                } else {
                    len as u32
                }
            }
            Err(_) => {
                // Fallback on encoding failure (should be extremely rare).
                // Use the heuristic so we don't panic or return garbage.
                estimate_tokens_heuristic(text)
            }
        }
    }

    fn name(&self) -> &'static str {
        "actual"
    }
}

/// Heuristic-based token estimator.
///
/// Uses ~4 characters per token for code, which is a reasonable approximation
/// for BPE tokenizers operating on source code (which has shorter average
/// tokens than natural language due to operators, single-char variables, etc).
///
/// This is intentionally conservative (over-estimates slightly) to avoid
/// chunks that exceed model limits. It's better to under-utilize a chunk
/// than to silently truncate embedded content.
pub struct EstimateTokenCounter;

impl TokenCounter for EstimateTokenCounter {
    fn count(&self, text: &str) -> u32 {
        estimate_tokens_heuristic(text)
    }

    fn name(&self) -> &'static str {
        "estimate"
    }
}

/// Shared heuristic: ~4 chars per token for code, minimum 1.
#[inline]
fn estimate_tokens_heuristic(text: &str) -> u32 {
    #[expect(clippy::cast_possible_truncation)]
    let estimate = (text.len() / 4) as u32;
    estimate.max(1)
}

/// Create the best available token counter.
///
/// Tries to load the tokenizer from the model directory. Falls back to
/// the heuristic estimator if the tokenizer isn't available.
pub fn create_token_counter(tokenizer_path: Option<&std::path::Path>) -> Arc<dyn TokenCounter> {
    if let Some(path) = tokenizer_path {
        if let Some(counter) = ActualTokenCounter::from_path(path) {
            tracing::info!(
                path = %path.display(),
                "loaded tokenizer for accurate token counting"
            );
            return Arc::new(counter);
        }
        tracing::warn!(
            path = %path.display(),
            "tokenizer file not found or invalid, falling back to heuristic estimator"
        );
    }
    Arc::new(EstimateTokenCounter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_counter_empty() {
        let counter = EstimateTokenCounter;
        assert_eq!(counter.count(""), 1);
    }

    #[test]
    fn test_estimate_counter_short() {
        let counter = EstimateTokenCounter;
        assert_eq!(counter.count("abcd"), 1);
        assert_eq!(counter.count("abcdefgh"), 2);
    }

    #[test]
    fn test_estimate_counter_code() {
        let counter = EstimateTokenCounter;
        let code = "fn hello() -> String { \"world\".to_string() }";
        let tokens = counter.count(code);
        // 44 chars / 4 = 11 tokens
        assert_eq!(tokens, 11);
    }

    #[test]
    fn test_estimate_counter_long() {
        let counter = EstimateTokenCounter;
        let text = "a".repeat(400);
        assert_eq!(counter.count(&text), 100);
    }

    #[test]
    fn test_estimate_counter_minimum_one() {
        let counter = EstimateTokenCounter;
        assert_eq!(counter.count("ab"), 1);
        assert_eq!(counter.count(""), 1);
    }

    #[test]
    fn test_estimate_counter_name() {
        let counter = EstimateTokenCounter;
        assert_eq!(counter.name(), "estimate");
    }

    #[test]
    fn test_create_counter_fallback() {
        let counter = create_token_counter(None);
        assert_eq!(counter.name(), "estimate");
    }

    #[test]
    fn test_create_counter_bad_path() {
        let counter =
            create_token_counter(Some(std::path::Path::new("/nonexistent/tokenizer.json")));
        assert_eq!(counter.name(), "estimate");
    }

    #[test]
    fn test_counter_is_deterministic() {
        let counter = EstimateTokenCounter;
        let text = "fn process(data: Vec<u8>) -> Result<(), Error> { Ok(()) }";
        let count1 = counter.count(text);
        let count2 = counter.count(text);
        assert_eq!(count1, count2, "token counter must be deterministic");
    }
}
