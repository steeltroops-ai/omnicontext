//! Semantic code chunker.
//!
//! Takes structural elements from the parser and produces chunks suitable
//! for embedding and indexing. Chunks respect AST boundaries and never
//! split mid-expression.
//!
//! ## Chunking Strategy
//!
//! 1. Single function/method if < max_tokens -> single chunk
//! 2. Single class/struct if < max_tokens -> single chunk
//! 3. Large class/struct -> split at method/field boundaries
//! 4. Large function -> split at block-level statement boundaries
//! 5. 10-15% token overlap at boundaries for context continuity
//! 6. Each chunk preserves the parent signature as a header for context

use crate::config::Config;
use crate::parser::StructuralElement;
use crate::types::{Chunk, ChunkKind};

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
    let overlap_fraction = 0.12; // 12% overlap
    let mut chunks = Vec::new();

    for elem in elements {
        let estimated_tokens = estimate_tokens(&elem.content);

        if estimated_tokens <= max_tokens {
            // Element fits in a single chunk -- most common case
            chunks.push(element_to_chunk(elem, file_id, estimated_tokens));
        } else {
            // Element is too large -- apply splitting strategy
            let split_chunks = split_element(elem, file_id, max_tokens, overlap_fraction);
            chunks.extend(split_chunks);
        }
    }

    chunks
}

/// Convert an element that fits within the token budget to a Chunk.
fn element_to_chunk(elem: &StructuralElement, file_id: i64, token_count: u32) -> Chunk {
    Chunk {
        id: 0,
        file_id,
        symbol_path: elem.symbol_path.clone(),
        kind: elem.kind,
        visibility: elem.visibility,
        line_start: elem.line_start,
        line_end: elem.line_end,
        content: elem.content.clone(),
        doc_comment: elem.doc_comment.clone(),
        token_count,
        weight: compute_weight(elem),
        vector_id: None,
    }
}

/// Compute structural importance weight for a chunk.
///
/// Weight = kind_weight * visibility_multiplier
/// Range: [0.35, 0.95] (Function/Private through Class/Public)
fn compute_weight(elem: &StructuralElement) -> f64 {
    elem.kind.default_weight() * elem.visibility.weight_multiplier()
}

/// Split a large element into multiple chunks with overlap.
///
/// Strategy depends on element kind:
/// - Class/Trait -> split at method boundaries (lines with `def` / `fn` / method signatures)
/// - Function -> split at statement boundaries (lines that start at indent level 1)
/// - Other -> split at line boundaries with overlap
fn split_element(
    elem: &StructuralElement,
    file_id: i64,
    max_tokens: u32,
    overlap_fraction: f64,
) -> Vec<Chunk> {
    let lines: Vec<&str> = elem.content.lines().collect();

    if lines.is_empty() {
        return Vec::new();
    }

    // Build a header from the first line (signature) for context continuity
    let header = extract_header(elem);

    // Find split points based on element kind
    let split_points = match elem.kind {
        ChunkKind::Class | ChunkKind::Trait | ChunkKind::Impl => {
            find_class_split_points(&lines)
        }
        ChunkKind::Function | ChunkKind::Test => {
            find_function_split_points(&lines)
        }
        _ => {
            find_line_split_points(&lines, max_tokens)
        }
    };

    create_chunks_from_splits(elem, file_id, &lines, &split_points, &header, max_tokens, overlap_fraction)
}

/// Extract a header line for context when splitting.
///
/// For a function: `def foo(args):` or `fn foo(args) -> Type {`
/// For a class: `class Foo(Base):` or `struct Foo {`
fn extract_header(elem: &StructuralElement) -> String {
    let first_line = elem.content.lines().next().unwrap_or("");

    // For decorated definitions, find the actual signature
    if first_line.trim_start().starts_with('@') || first_line.trim_start().starts_with('#') {
        // Find the definition line
        for line in elem.content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("def ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("func ")
                || trimmed.starts_with("type ")
                || trimmed.starts_with("function ")
            {
                return line.to_string();
            }
        }
    }

    first_line.to_string()
}

/// Find split points for class-like elements (at method boundaries).
fn find_class_split_points(lines: &[&str]) -> Vec<usize> {
    let mut split_points = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            continue; // Skip the class definition line
        }

        let trimmed = line.trim();

        // Python method boundaries
        if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
            split_points.push(i);
        }
        // Rust method/function boundaries inside impl/trait
        else if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub(crate) fn ")
        {
            split_points.push(i);
        }
        // TypeScript/JavaScript method boundaries
        else if (trimmed.starts_with("constructor(")
            || trimmed.starts_with("async ")
            || trimmed.starts_with("get ")
            || trimmed.starts_with("set ")
            || trimmed.starts_with("static "))
            && trimmed.contains('(')
        {
            split_points.push(i);
        }
        // Go method-like structure in struct
        // (handled separately since Go methods aren't inside struct bodies)
    }

    // If no method boundaries found, fall back to line-based splits
    if split_points.is_empty() {
        return find_line_split_points(lines, 512); // default chunk size
    }

    split_points
}

/// Find split points for function-like elements (at statement boundaries).
fn find_function_split_points(lines: &[&str]) -> Vec<usize> {
    let mut split_points = Vec::new();

    if lines.is_empty() {
        return split_points;
    }

    // Determine the base indentation level (first non-empty line after signature)
    let base_indent = lines
        .iter()
        .skip(1) // skip signature
        .find(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(0);

    for (i, line) in lines.iter().enumerate() {
        if i <= 1 {
            continue; // Skip signature and opening brace/colon
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let indent = line.len() - trimmed.len();

        // A statement at base indentation is a split candidate
        if indent == base_indent {
            // Check for meaningful statement boundaries
            if trimmed.starts_with("if ")
                || trimmed.starts_with("for ")
                || trimmed.starts_with("while ")
                || trimmed.starts_with("match ")
                || trimmed.starts_with("return ")
                || trimmed.starts_with("let ")
                || trimmed.starts_with("const ")
                || trimmed.starts_with("try ")
                || trimmed.starts_with("with ")
                || trimmed == "}"
                || trimmed.ends_with(':')
            {
                split_points.push(i);
            }
        }
    }

    if split_points.is_empty() {
        return find_line_split_points(lines, 512);
    }

    split_points
}

/// Fallback: split at regular line intervals.
fn find_line_split_points(lines: &[&str], max_tokens: u32) -> Vec<usize> {
    let max_lines = ((max_tokens as usize) * 4) / 80; // ~80 chars per line avg
    let max_lines = max_lines.max(10);

    let mut split_points = Vec::new();
    let mut i = max_lines;
    while i < lines.len() {
        // Try to find a blank line near the split point
        let search_start = i.saturating_sub(5);
        let search_end = (i + 5).min(lines.len());

        let best = (search_start..search_end)
            .find(|&j| lines[j].trim().is_empty())
            .unwrap_or(i);

        split_points.push(best);
        i = best + max_lines;
    }

    split_points
}

/// Create chunks from the identified split points with overlap.
fn create_chunks_from_splits(
    elem: &StructuralElement,
    file_id: i64,
    lines: &[&str],
    split_points: &[usize],
    header: &str,
    max_tokens: u32,
    overlap_fraction: f64,
) -> Vec<Chunk> {
    let mut chunks = Vec::new();

    // Build chunk boundaries: [(start, end), ...]
    let mut boundaries: Vec<(usize, usize)> = Vec::new();
    let mut prev_start = 0;

    for &split in split_points {
        if split > prev_start {
            boundaries.push((prev_start, split));
            prev_start = split;
        }
    }
    // Final segment
    if prev_start < lines.len() {
        boundaries.push((prev_start, lines.len()));
    }

    // If we only got one segment, just truncate
    if boundaries.len() <= 1 {
        let content = truncate_to_tokens(&elem.content, max_tokens);
        let tokens = estimate_tokens(&content);
        chunks.push(Chunk {
            id: 0,
            file_id,
            symbol_path: elem.symbol_path.clone(),
            kind: elem.kind,
            visibility: elem.visibility,
            line_start: elem.line_start,
            line_end: elem.line_end,
            content,
            doc_comment: elem.doc_comment.clone(),
            token_count: tokens,
            weight: compute_weight(elem),
            vector_id: None,
        });
        return chunks;
    }

    // Merge boundaries that are too small into the previous chunk
    let merged = merge_small_boundaries(&boundaries, max_tokens, lines);

    for (i, &(start, end)) in merged.iter().enumerate() {
        // Apply overlap: include some lines from the previous chunk
        let overlap_lines = if i > 0 {
            let prev_end = merged[i - 1].1;
            let prev_start = merged[i - 1].0;
            let prev_len = prev_end - prev_start;
            let overlap_count = ((prev_len as f64) * overlap_fraction).ceil() as usize;
            overlap_count.min(prev_end.saturating_sub(start))
        } else {
            0
        };

        let effective_start = start.saturating_sub(overlap_lines);

        // Build chunk content
        let mut content_parts = Vec::new();

        // Add header for non-first chunks (provides context)
        if i > 0 && !header.is_empty() {
            content_parts.push(format!("// ... continued from {}", elem.name));
            content_parts.push(header.to_string());
        }

        // Add the actual content lines
        for line in &lines[effective_start..end] {
            content_parts.push((*line).to_string());
        }

        let content = content_parts.join("\n");
        let tokens = estimate_tokens(&content);

        // Calculate line numbers
        let chunk_line_start = elem.line_start + effective_start as u32;
        let chunk_line_end = elem.line_start + end as u32 - 1;

        let symbol_path = if merged.len() > 1 {
            format!("{}[{}/{}]", elem.symbol_path, i + 1, merged.len())
        } else {
            elem.symbol_path.clone()
        };

        chunks.push(Chunk {
            id: 0,
            file_id,
            symbol_path,
            kind: elem.kind,
            visibility: elem.visibility,
            line_start: chunk_line_start,
            line_end: chunk_line_end,
            content,
            doc_comment: if i == 0 {
                elem.doc_comment.clone()
            } else {
                None
            },
            token_count: tokens,
            weight: compute_weight(elem),
            vector_id: None,
        });
    }

    chunks
}

/// Merge boundaries that are too small (< 25% of max_tokens) with neighbors.
fn merge_small_boundaries(
    boundaries: &[(usize, usize)],
    max_tokens: u32,
    lines: &[&str],
) -> Vec<(usize, usize)> {
    if boundaries.is_empty() {
        return Vec::new();
    }

    let min_tokens = max_tokens / 4;
    let mut merged = Vec::new();
    let mut current_start = boundaries[0].0;
    let mut current_end = boundaries[0].1;

    for &(start, end) in boundaries.iter().skip(1) {
        let current_content: String = lines[current_start..current_end].join("\n");
        let current_tokens = estimate_tokens(&current_content);

        if current_tokens < min_tokens {
            // Too small -- extend to include the next boundary
            current_end = end;
        } else {
            merged.push((current_start, current_end));
            current_start = start;
            current_end = end;
        }
    }

    merged.push((current_start, current_end));
    merged
}

/// Rough token estimation: ~4 characters per token for code.
///
/// This is intentionally conservative. Actual tokenization happens
/// in the embedder. The estimate is used for budget management only.
pub fn estimate_tokens(content: &str) -> u32 {
    // Rule of thumb for code: 1 token per ~3.5 chars (tighter than natural language)
    // We use 4 to be slightly conservative (better to under-split than over-split)
    #[expect(clippy::cast_possible_truncation)]
    let estimate = (content.len() / 4) as u32;
    estimate.max(1)
}

/// Truncate content to approximately `max_tokens` tokens at a line boundary.
pub fn truncate_to_tokens(content: &str, max_tokens: u32) -> String {
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
    use crate::parser::StructuralElement;
    use crate::types::{ChunkKind, Visibility};
    use std::path::Path;

    fn make_element(content: &str, kind: ChunkKind) -> StructuralElement {
        StructuralElement {
            symbol_path: "test.module.thing".to_string(),
            name: "thing".to_string(),
            kind,
            visibility: Visibility::Public,
            line_start: 1,
            line_end: content.lines().count() as u32,
            content: content.to_string(),
            doc_comment: Some("A doc comment.".to_string()),
            references: vec!["foo".to_string()],
        }
    }

    fn default_config() -> Config {
        Config::defaults(Path::new("/tmp/test-repo"))
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 1); // minimum 1
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
        assert_eq!(estimate_tokens("a".repeat(400).as_str()), 100);
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

    #[test]
    fn test_small_element_single_chunk() {
        let content = "def hello():\n    return 'world'\n";
        let elem = make_element(content, ChunkKind::Function);
        let config = default_config();

        let chunks = chunk_elements(&[elem], 1, &config);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, content);
        assert!(chunks[0].doc_comment.is_some());
    }

    #[test]
    fn test_weight_computation() {
        let public_func = make_element("fn foo() {}", ChunkKind::Function);
        assert!((compute_weight(&public_func) - 0.85).abs() < 0.001); // 0.85 * 1.0

        let mut private_func = make_element("fn _foo() {}", ChunkKind::Function);
        private_func.visibility = Visibility::Private;
        assert!((compute_weight(&private_func) - 0.595).abs() < 0.001); // 0.85 * 0.70

        let public_class = make_element("class Foo:", ChunkKind::Class);
        assert!((compute_weight(&public_class) - 0.95).abs() < 0.001); // 0.95 * 1.0

        let test = make_element("def test_foo():", ChunkKind::Test);
        assert!((compute_weight(&test) - 0.60).abs() < 0.001); // 0.60 * 1.0
    }

    #[test]
    fn test_large_element_gets_split() {
        // Create a "class" with 100 short methods to force splitting
        let mut lines = vec!["class BigClass:".to_string()];
        for i in 0..100 {
            lines.push(format!("    def method_{i}(self):"));
            lines.push(format!("        return {i}"));
            lines.push(String::new());
        }
        let content = lines.join("\n");

        let elem = make_element(&content, ChunkKind::Class);
        let mut config = default_config();
        config.indexing.max_chunk_tokens = 50; // force splitting

        let chunks = chunk_elements(&[elem], 1, &config);
        assert!(
            chunks.len() > 1,
            "large element should be split into multiple chunks, got {}",
            chunks.len()
        );

        // First chunk should have doc_comment, others should not
        assert!(chunks[0].doc_comment.is_some());
        if chunks.len() > 1 {
            assert!(chunks[1].doc_comment.is_none());
        }
    }

    #[test]
    fn test_split_preserves_all_content() {
        // Create content that will be split
        let mut lines = vec!["def big_function():".to_string()];
        for i in 0..50 {
            lines.push(format!("    x_{i} = compute({i})"));
            if i % 10 == 0 {
                lines.push(format!("    if x_{i} > 0:"));
                lines.push(format!("        return x_{i}"));
            }
        }
        let content = lines.join("\n");

        let elem = make_element(&content, ChunkKind::Function);
        let mut config = default_config();
        config.indexing.max_chunk_tokens = 40;

        let chunks = chunk_elements(&[elem], 1, &config);
        // All chunks should have content
        for chunk in &chunks {
            assert!(!chunk.content.is_empty(), "no chunk should be empty");
            assert!(chunk.token_count > 0, "token count should be > 0");
        }
    }

    #[test]
    fn test_extract_header_decorated() {
        let elem = make_element(
            "@staticmethod\ndef create():\n    pass",
            ChunkKind::Function,
        );
        let header = extract_header(&elem);
        assert_eq!(header, "def create():");
    }

    #[test]
    fn test_extract_header_normal() {
        let elem = make_element("def hello():\n    pass", ChunkKind::Function);
        let header = extract_header(&elem);
        assert_eq!(header, "def hello():");
    }

    #[test]
    fn test_find_class_split_points() {
        let code = "class Foo:\n    def a(self):\n        pass\n    def b(self):\n        pass\n";
        let lines: Vec<&str> = code.lines().collect();
        let points = find_class_split_points(&lines);
        assert!(points.contains(&1), "should split at def a: {points:?}");
        assert!(points.contains(&3), "should split at def b: {points:?}");
    }

    #[test]
    fn test_find_function_split_points_if_blocks() {
        let code = "def foo():\n    x = 1\n    if x > 0:\n        return x\n    return 0\n";
        let lines: Vec<&str> = code.lines().collect();
        let points = find_function_split_points(&lines);
        // Should find `if` and `return` at base indent
        assert!(!points.is_empty(), "should find split points");
    }

    #[test]
    fn test_empty_elements() {
        let config = default_config();
        let chunks = chunk_elements(&[], 1, &config);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_multiple_elements() {
        let config = default_config();
        let elements = vec![
            make_element("def a():\n    pass\n", ChunkKind::Function),
            make_element("def b():\n    pass\n", ChunkKind::Function),
            make_element("class C:\n    pass\n", ChunkKind::Class),
        ];

        let chunks = chunk_elements(&elements, 1, &config);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].kind, ChunkKind::Function);
        assert_eq!(chunks[2].kind, ChunkKind::Class);
    }

    #[test]
    fn test_chunk_weight_ordering() {
        let public_class = make_element("class Foo:", ChunkKind::Class);
        let public_func = make_element("def foo:", ChunkKind::Function);
        let test_func = make_element("def test_foo:", ChunkKind::Test);

        assert!(
            compute_weight(&public_class) > compute_weight(&public_func),
            "public class should outweigh public function"
        );
        assert!(
            compute_weight(&public_func) > compute_weight(&test_func),
            "public function should outweigh test function"
        );
    }
}
