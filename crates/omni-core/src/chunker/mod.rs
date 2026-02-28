//! Semantic code chunker.
//!
//! Takes structural elements from the parser and produces chunks suitable
//! for embedding and indexing. Chunks respect AST boundaries and never
//! split mid-expression.
//!
//! ## Chunking Strategy
//!
//! 1. Single function/method if < max_tokens
//! 2. Single class/struct if < max_tokens
//! 3. Class split at method boundaries if too large
//! 4. Large function split at block boundaries as last resort
//! 5. 10-15% token overlap at boundaries for context continuity

use crate::config::Config;
use crate::parser::StructuralElement;
use crate::types::Chunk;

/// Chunk structural elements into embedding-sized pieces.
///
/// Each chunk is annotated with metadata for the index:
/// symbol path, kind, visibility, line range, weight.
pub fn chunk_elements(
    elements: &[StructuralElement],
    file_id: i64,
    config: &Config,
) -> Vec<Chunk> {
    let max_tokens = config.indexing.max_chunk_tokens;
    let mut chunks = Vec::new();

    for elem in elements {
        let estimated_tokens = estimate_tokens(&elem.content);

        if estimated_tokens <= max_tokens {
            // Element fits in a single chunk
            chunks.push(Chunk {
                id: 0,
                file_id,
                symbol_path: elem.symbol_path.clone(),
                kind: elem.kind,
                visibility: elem.visibility,
                line_start: elem.line_start,
                line_end: elem.line_end,
                content: elem.content.clone(),
                doc_comment: elem.doc_comment.clone(),
                token_count: estimated_tokens,
                weight: elem.kind.default_weight() * elem.visibility.weight_multiplier(),
                vector_id: None,
            });
        } else {
            // TODO: Implement splitting strategies
            // For now, truncate to max_tokens
            let truncated = truncate_to_tokens(&elem.content, max_tokens);
            chunks.push(Chunk {
                id: 0,
                file_id,
                symbol_path: elem.symbol_path.clone(),
                kind: elem.kind,
                visibility: elem.visibility,
                line_start: elem.line_start,
                line_end: elem.line_end,
                content: truncated,
                doc_comment: elem.doc_comment.clone(),
                token_count: max_tokens,
                weight: elem.kind.default_weight() * elem.visibility.weight_multiplier(),
                vector_id: None,
            });
        }
    }

    chunks
}

/// Rough token estimation: ~4 characters per token for code.
/// This is conservative; actual tokenization happens in the embedder.
fn estimate_tokens(content: &str) -> u32 {
    #[expect(clippy::cast_possible_truncation)]
    let estimate = (content.len() / 4) as u32;
    estimate.max(1)
}

/// Truncate content to approximately `max_tokens` tokens.
fn truncate_to_tokens(content: &str, max_tokens: u32) -> String {
    let max_chars = (max_tokens as usize) * 4;
    if content.len() <= max_chars {
        return content.to_string();
    }

    // Find the last newline before max_chars to avoid mid-line truncation
    let truncated = &content[..max_chars];
    if let Some(last_newline) = truncated.rfind('\n') {
        content[..last_newline].to_string()
    } else {
        truncated.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 1); // minimum 1
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn test_truncate_to_tokens_short_content() {
        let content = "hello\nworld";
        assert_eq!(truncate_to_tokens(content, 100), content);
    }

    #[test]
    fn test_truncate_to_tokens_at_line_boundary() {
        let content = "line1\nline2\nline3\nline4\n";
        let result = truncate_to_tokens(content, 2); // ~8 chars
        assert!(result.ends_with("line1")); // truncates at first newline within 8 chars
    }
}
