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

pub mod contextual;
pub mod token_counter;

use crate::config::Config;
use crate::parser::StructuralElement;
use crate::types::{Chunk, ChunkKind, FileInfo, ImportStatement};

use self::token_counter::TokenCounter;

/// Chunk structural elements into embedding-sized pieces.
///
/// Each chunk is annotated with metadata for the index:
/// symbol path, kind, visibility, line range, weight.
///
/// CAST (Chunking via Abstract Syntax Trees) implementation:
/// - Backward overlap captures preceding context (configurable via `overlap_tokens`/`overlap_lines`)
/// - Module-level declarations (imports, type defs) are injected even if far from the element
/// - Intra-element splits use configurable `overlap_fraction` for context continuity
///
/// The `counter` parameter controls how tokens are counted:
/// - `ActualTokenCounter` for production (uses the same tokenizer as the embedder)
/// - `EstimateTokenCounter` for tests and environments without a loaded model
pub fn chunk_elements(
    elements: &[StructuralElement],
    file_info: &FileInfo,
    imports: &[ImportStatement],
    file_id: i64,
    config: &Config,
    source_code: &str,
    counter: &dyn TokenCounter,
) -> Vec<Chunk> {
    let max_tokens = config.indexing.max_chunk_tokens;
    let overlap_fraction = config.indexing.overlap_fraction;
    let target_overlap_tokens = config.indexing.overlap_tokens;
    let fallback_overlap_lines = config.indexing.overlap_lines;
    let include_module_decls = config.indexing.include_module_declarations;
    let mut chunks = Vec::new();

    let source_lines: Vec<&str> = source_code.lines().collect();

    let module_declarations = if include_module_decls {
        extract_module_declarations(&source_lines)
    } else {
        String::new()
    };

    for elem in elements {
        let start_line_idx = elem.line_start.saturating_sub(1) as usize;

        let backward_context = compute_backward_context(
            &source_lines,
            start_line_idx,
            target_overlap_tokens,
            fallback_overlap_lines,
            counter,
        );

        let estimated_tokens = counter.count(&elem.content) + counter.count(&backward_context);
        let mut context_header =
            build_context_header(elem, file_info, imports, &module_declarations);

        if !backward_context.is_empty() {
            context_header.push_str("// -- surrounding context --\n");
            context_header.push_str(&backward_context);
        }

        let total_tokens = estimated_tokens + counter.count(&context_header);

        if total_tokens <= max_tokens {
            chunks.push(element_to_chunk(
                elem, file_id, total_tokens, &context_header,
            ));
        } else {
            let split_chunks = split_element(
                elem, file_id, max_tokens, overlap_fraction, &context_header, counter,
            );
            chunks.extend(split_chunks);
        }
    }

    chunks
}

/// Compute backward context using token-based targeting with line-based fallback.
///
/// Grabs lines preceding the element until either `target_tokens` is reached
/// or `max_lines` is exhausted, whichever comes first.
fn compute_backward_context(
    source_lines: &[&str],
    start_line_idx: usize,
    target_tokens: u32,
    max_lines: usize,
    counter: &dyn TokenCounter,
) -> String {
    if start_line_idx == 0 {
        return String::new();
    }

    let earliest = start_line_idx.saturating_sub(max_lines);
    let mut selected_start = start_line_idx;
    let mut accumulated_tokens: u32 = 0;

    for idx in (earliest..start_line_idx).rev() {
        let line_tokens = counter.count(source_lines[idx]);
        if accumulated_tokens + line_tokens > target_tokens {
            break;
        }
        accumulated_tokens += line_tokens;
        selected_start = idx;
    }

    if selected_start < start_line_idx {
        source_lines[selected_start..start_line_idx].join("\n") + "\n"
    } else {
        String::new()
    }
}

/// Extract module-level declarations from the top of a source file.
///
/// Captures import statements, use declarations, type aliases, constants,
/// and other top-level declarations that provide essential context for
/// understanding any chunk in the file.
fn extract_module_declarations(source_lines: &[&str]) -> String {
    let mut declarations = Vec::new();
    let mut in_multiline = false;
    let mut multiline_buf = String::new();

    for line in source_lines {
        let trimmed = line.trim();

        // Skip empty lines and comments between declarations
        if (trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with('#') && trimmed.starts_with("#!"))
            && !in_multiline
        {
            continue;
        }

        // Handle multi-line imports: `use std::{`  or  `from x import (`
        if in_multiline {
            multiline_buf.push('\n');
            multiline_buf.push_str(line);
            // Check for closing delimiter
            if trimmed.ends_with(';')
                || trimmed.ends_with('}')
                || trimmed == "}"
                || trimmed == "};"
                || trimmed.ends_with(')')
            {
                declarations.push(multiline_buf.clone());
                multiline_buf.clear();
                in_multiline = false;
            }
            continue;
        }

        let is_declaration = trimmed.starts_with("import ")
            || trimmed.starts_with("from ")
            || trimmed.starts_with("use ")
            || trimmed.starts_with("pub use ")
            || trimmed.starts_with("pub(crate) use ")
            || trimmed.starts_with("extern crate ")
            || trimmed.starts_with("mod ")
            || trimmed.starts_with("pub mod ")
            || trimmed.starts_with("pub(crate) mod ")
            || trimmed.starts_with("package ")
            || trimmed.starts_with("require ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("pub const ")
            || trimmed.starts_with("pub(crate) const ")
            || trimmed.starts_with("static ")
            || trimmed.starts_with("pub static ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("pub type ")
            || trimmed.starts_with("typedef ")
            || trimmed.starts_with("using ")
            || trimmed.starts_with("#include ")
            || trimmed.starts_with("#define ")
            || trimmed.starts_with("var ")   // Go top-level var
            || (trimmed.starts_with("export ") && (trimmed.contains("type ") || trimmed.contains("interface ") || trimmed.contains("const ")));

        if is_declaration {
            // Check if this is a multi-line declaration (ends with `{` but no `;`)
            if (trimmed.contains('{') && !trimmed.contains('}') && !trimmed.ends_with(';'))
                || (trimmed.contains('(') && !trimmed.contains(')') && !trimmed.ends_with(';'))
            {
                in_multiline = true;
                multiline_buf = line.to_string();
                continue;
            }
            declarations.push(line.to_string());
        }

        // Stop scanning after we hit the first non-declaration code body
        let is_code_body = trimmed.starts_with("def ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub(crate) fn ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("pub(crate) struct ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("pub enum ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("pub(crate) enum ")
            || trimmed.starts_with("func ")
            || trimmed.starts_with("function ");

        if is_code_body {
            break;
        }
    }

    // If we were in a multi-line import that never closed, include what we have
    if in_multiline && !multiline_buf.is_empty() {
        declarations.push(multiline_buf);
    }

    if declarations.is_empty() {
        return String::new();
    }

    declarations.join("\n")
}

/// Build a contextual header for a chunk, enriching it with surrounding logic.
///
/// Includes module-level declarations so the LLM always has visibility
/// into the file's imports, types, and constants even when viewing an
/// isolated function deep in the file.
fn build_context_header(
    elem: &StructuralElement,
    file_info: &FileInfo,
    imports: &[ImportStatement],
    module_declarations: &str,
) -> String {
    let mut header = String::new();
    header.push_str(&format!(
        "[{}] {}\n",
        file_info.language.as_str(),
        elem.symbol_path
    ));
    header.push_str(&format!(
        "Kind: {:?} | Visibility: {:?} | File: {}\n",
        elem.kind,
        elem.visibility,
        file_info.path.display()
    ));

    if let Some(parent) = elem
        .symbol_path
        .rsplit_once("::")
        .map(|x| x.0)
        .or_else(|| elem.symbol_path.rsplit_once('.').map(|x| x.0))
    {
        if !parent.is_empty() {
            header.push_str(&format!("Parent: {}\n", parent));
        }
    }

    if !module_declarations.is_empty() {
        header.push_str("// -- module declarations --\n");
        header.push_str(module_declarations);
        header.push('\n');
    }

    if !imports.is_empty() {
        let import_list: Vec<&str> = imports
            .iter()
            .take(8)
            .map(|i| i.import_path.as_str())
            .collect();
        let mut import_str = import_list.join(", ");
        if imports.len() > 8 {
            import_str.push_str(", ...");
        }
        header.push_str(&format!("Imports: {}\n", import_str));
    }
    if !elem.references.is_empty() {
        let refs: Vec<&str> = elem
            .references
            .iter()
            .take(10)
            .map(|r| r.as_str())
            .collect();
        let mut ref_str = refs.join(", ");
        if elem.references.len() > 10 {
            ref_str.push_str(", ...");
        }
        header.push_str(&format!("References: {}\n", ref_str));
    }
    header.push_str("---\n");
    header
}

/// Enrich chunk content with dependency-graph context (callers/callees).
///
/// Implements Anthropic Contextual Retrieval (2024): prepend a context
/// sentence describing where this chunk fits in the codebase. The context
/// is derived from the dependency graph, not an LLM, so it has zero cost.
///
/// Call this after the dependency graph is built (post-indexing).
#[allow(clippy::similar_names)]
pub fn enrich_chunk_with_graph_context(
    chunk_content: &str,
    callers: &[String],
    callees: &[String],
) -> String {
    let mut context = String::new();

    if !callers.is_empty() {
        let caller_list: Vec<&str> = callers.iter().take(5).map(|s| s.as_str()).collect();
        context.push_str(&format!("Called by: {}\n", caller_list.join(", ")));
    }

    if !callees.is_empty() {
        let callee_list: Vec<&str> = callees.iter().take(5).map(|s| s.as_str()).collect();
        context.push_str(&format!("Calls: {}\n", callee_list.join(", ")));
    }

    if context.is_empty() {
        return chunk_content.to_string();
    }

    // Prepend graph context before the chunk body, after any existing header
    if let Some(sep_pos) = chunk_content.find("---\n") {
        let after_sep = sep_pos + 4;
        format!(
            "{}{}{}",
            &chunk_content[..after_sep],
            context,
            &chunk_content[after_sep..],
        )
    } else {
        format!("{context}{chunk_content}")
    }
}

/// Convert an element that fits within the token budget to a Chunk.
fn element_to_chunk(
    elem: &StructuralElement,
    file_id: i64,
    token_count: u32,
    context_header: &str,
) -> Chunk {
    let content = format!("{}{}", context_header, elem.content);
    Chunk {
        id: 0,
        file_id,
        symbol_path: elem.symbol_path.clone(),
        kind: elem.kind,
        visibility: elem.visibility,
        line_start: elem.line_start,
        line_end: elem.line_end,
        content,
        doc_comment: elem.doc_comment.clone(),
        token_count,
        weight: compute_weight(elem),
        vector_id: None,
        is_summary: false,
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
    context_header: &str,
    counter: &dyn TokenCounter,
) -> Vec<Chunk> {
    let lines: Vec<&str> = elem.content.lines().collect();

    if lines.is_empty() {
        return Vec::new();
    }

    // Build a header from the first line (signature) for context continuity
    let header = extract_header(elem);

    // Find split points based on element kind
    let split_points = match elem.kind {
        ChunkKind::Class | ChunkKind::Trait | ChunkKind::Impl => find_class_split_points(&lines),
        ChunkKind::Function | ChunkKind::Test => find_function_split_points(&lines),
        _ => find_line_split_points(&lines, max_tokens),
    };

    create_chunks_from_splits(
        elem, file_id, &lines, &split_points, &header, max_tokens, overlap_fraction,
        context_header, counter,
    )
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
    context_header: &str,
    counter: &dyn TokenCounter,
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
        let mut content = format!("{}{}", context_header, elem.content);
        if counter.count(&content) > max_tokens {
            content = truncate_to_tokens(&content, max_tokens);
        }
        let tokens = counter.count(&content);
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
            is_summary: false,
        });
        return chunks;
    }

    // Merge boundaries that are too small into the previous chunk
    let merged = merge_small_boundaries(&boundaries, max_tokens, lines, counter);

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

        // Add the cross-chunk contextual header FIRST
        content_parts.push(context_header.trim_end().to_string());

        // Add code header for non-first chunks (provides context)
        if i > 0 && !header.is_empty() {
            content_parts.push(format!("// ... continued from {}", elem.name));
            content_parts.push(header.to_string());
        }

        // Add the actual content lines
        for line in &lines[effective_start..end] {
            content_parts.push((*line).to_string());
        }

        let content = content_parts.join("\n");
        let tokens = counter.count(&content);

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
            is_summary: false,
        });
    }

    chunks
}

/// Merge boundaries that are too small (< 25% of max_tokens) with neighbors.
fn merge_small_boundaries(
    boundaries: &[(usize, usize)],
    max_tokens: u32,
    lines: &[&str],
    counter: &dyn TokenCounter,
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
        let current_tokens = counter.count(&current_content);

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

/// Minimum number of leaf chunks in a file before a summary chunk is generated.
const SUMMARY_MIN_CHUNKS: usize = 3;

/// Maximum tokens for a summary chunk content.
const SUMMARY_MAX_TOKENS: u32 = 512;

/// Generate RAPTOR-style hierarchical summary chunks for a set of leaf chunks.
///
/// For each file that has > `SUMMARY_MIN_CHUNKS` leaf chunks, this produces a
/// single `ChunkKind::Summary` chunk that concatenates:
/// 1. File-level header (module path, language)
/// 2. Exported symbol signatures (pub functions, structs, traits)
/// 3. First-line doc comments for each exported symbol
/// 4. Structural import summary
///
/// Summary chunks are embedded alongside leaf chunks, giving the vector index
/// a high-level representation that answers architectural queries without
/// requiring aggregation across dozens of function-level chunks.
pub fn generate_summary_chunks(
    leaf_chunks: &[Chunk],
    file_info: &FileInfo,
    counter: &dyn TokenCounter,
) -> Vec<Chunk> {
    if leaf_chunks.len() < SUMMARY_MIN_CHUNKS {
        return Vec::new();
    }

    let mut summaries = Vec::new();

    // Build the summary content from leaf chunk signatures
    let mut content = String::with_capacity(2048);

    // File header
    content.push_str(&format!(
        "// File summary: {}\n// Language: {}\n// Symbols: {}\n\n",
        file_info.path.display(),
        file_info.language.as_str(),
        leaf_chunks.len()
    ));

    // Add exported symbols first (most important for architectural queries)
    let mut exported_count = 0;
    for chunk in leaf_chunks {
        if chunk.is_summary {
            continue; // skip existing summaries
        }

        let is_exported = matches!(
            chunk.visibility,
            crate::types::Visibility::Public | crate::types::Visibility::Crate
        );
        if !is_exported && !matches!(chunk.kind, ChunkKind::Trait | ChunkKind::Class) {
            continue;
        }

        // Extract signature (first non-empty line of content)
        let signature = extract_signature_from_content(&chunk.content);
        if signature.is_empty() {
            continue;
        }

        // Add doc summary if available
        if let Some(ref doc) = chunk.doc_comment {
            let first_line = doc.lines().next().unwrap_or("");
            if !first_line.is_empty() {
                content.push_str(&format!("/// {first_line}\n"));
            }
        }

        content.push_str(&format!("{}\n", signature));
        exported_count += 1;

        // Budget check: don't exceed summary token limit
        if counter.count(&content) >= SUMMARY_MAX_TOKENS {
            break;
        }
    }

    // If we have room, add private symbols as brief declarations
    if counter.count(&content) < SUMMARY_MAX_TOKENS {
        for chunk in leaf_chunks {
            if chunk.is_summary {
                continue;
            }
            let is_private = matches!(
                chunk.visibility,
                crate::types::Visibility::Private | crate::types::Visibility::Protected
            );
            if !is_private {
                continue;
            }

            let signature = extract_signature_from_content(&chunk.content);
            if signature.is_empty() {
                continue;
            }

            content.push_str(&format!("{}\n", signature));

            if counter.count(&content) >= SUMMARY_MAX_TOKENS {
                break;
            }
        }
    }

    // Only generate if we actually captured meaningful content
    if exported_count == 0 && leaf_chunks.len() < 5 {
        return Vec::new();
    }

    let line_start = leaf_chunks.iter().map(|c| c.line_start).min().unwrap_or(1);
    let line_end = leaf_chunks.iter().map(|c| c.line_end).max().unwrap_or(1);
    let file_id = leaf_chunks[0].file_id;

    // Construct the module-level symbol path
    let module_path = file_info
        .path
        .to_string_lossy()
        .replace(['/', '\\'], "::")
        .replace(".rs", "")
        .replace(".py", "")
        .replace(".ts", "")
        .replace(".js", "");

    let token_count = counter.count(&content);

    summaries.push(Chunk {
        id: 0,
        file_id,
        symbol_path: format!("{module_path}::__summary__"),
        kind: ChunkKind::Summary,
        visibility: crate::types::Visibility::Public,
        line_start,
        line_end,
        content,
        doc_comment: Some(format!(
            "File-level summary of {}",
            file_info.path.display()
        )),
        token_count,
        weight: ChunkKind::Summary.default_weight(),
        vector_id: None,
        is_summary: true,
    });

    summaries
}

/// Extract the signature line from chunk content.
///
/// Skips decorators (@...), doc comments (///), and blank lines
/// to find the actual declaration line.
fn extract_signature_from_content(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('@')
            || trimmed.starts_with('#')
            || trimmed.starts_with("///")
            || trimmed.starts_with("//!")
            || trimmed.starts_with("/**")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("*/")
        {
            continue;
        }
        return trimmed.to_string();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::StructuralElement;
    use crate::types::{ChunkKind, Visibility};
    use std::path::Path;
    use token_counter::EstimateTokenCounter;

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
            extends: Vec::new(),
            implements: Vec::new(),
        }
    }

    fn default_config() -> Config {
        Config::defaults(Path::new("/tmp/test-repo"))
    }

    fn dummy_file_info() -> FileInfo {
        FileInfo {
            id: 1,
            path: std::path::PathBuf::from("test.py"),
            language: crate::types::Language::Python,
            content_hash: "dummyhash".to_string(),
            size_bytes: 100,
        }
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
        let source = "import sys\n\ndef hello():\n    return 'world'\n";
        let content = "def hello():\n    return 'world'\n";
        let elem = make_element(content, ChunkKind::Function);
        let config = default_config();
        let file_info = dummy_file_info();

        let chunks = chunk_elements(
            &[elem],
            &file_info,
            &[],
            1,
            &config,
            source,
            &EstimateTokenCounter,
        );
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].content.contains(content)); // includes context header now
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
        let source = content.clone();

        let elem = make_element(&content, ChunkKind::Class);
        let mut config = default_config();
        config.indexing.max_chunk_tokens = 50; // force splitting
        let file_info = dummy_file_info();

        let chunks = chunk_elements(
            &[elem],
            &file_info,
            &[],
            1,
            &config,
            &source,
            &EstimateTokenCounter,
        );
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
        let source = content.clone();

        let elem = make_element(&content, ChunkKind::Function);
        let mut config = default_config();
        config.indexing.max_chunk_tokens = 40;
        let file_info = dummy_file_info();

        let chunks = chunk_elements(
            &[elem],
            &file_info,
            &[],
            1,
            &config,
            &source,
            &EstimateTokenCounter,
        );
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
        let file_info = dummy_file_info();
        let chunks = chunk_elements(&[], &file_info, &[], 1, &config, "", &EstimateTokenCounter);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_multiple_elements() {
        let config = default_config();
        let file_info = dummy_file_info();
        let source = "def a():\n    pass\ndef b():\n    pass\nclass C:\n    pass\n";
        let elements = vec![
            make_element("def a():\n    pass\n", ChunkKind::Function),
            make_element("def b():\n    pass\n", ChunkKind::Function),
            make_element("class C:\n    pass\n", ChunkKind::Class),
        ];

        let chunks = chunk_elements(
            &elements,
            &file_info,
            &[],
            1,
            &config,
            source,
            &EstimateTokenCounter,
        );
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

    #[test]
    fn test_enrich_no_context() {
        let content = "fn foo() {}";
        let result = enrich_chunk_with_graph_context(content, &[], &[]);
        assert_eq!(result, content);
    }

    #[test]
    fn test_enrich_with_callers() {
        let content = "fn foo() {}";
        let callers = vec!["bar".to_string(), "baz".to_string()];
        let result = enrich_chunk_with_graph_context(content, &callers, &[]);
        assert!(result.contains("Called by: bar, baz"));
        assert!(result.contains("fn foo() {}"));
    }

    #[test]
    fn test_enrich_with_callees() {
        let content = "fn foo() {}";
        let callees = vec!["init".to_string()];
        let result = enrich_chunk_with_graph_context(content, &[], &callees);
        assert!(result.contains("Calls: init"));
    }

    #[test]
    fn test_enrich_with_header_separator() {
        let content = "[rust] module::foo\n---\nfn foo() {}";
        let callers = vec!["main".to_string()];
        let result = enrich_chunk_with_graph_context(content, &callers, &[]);
        // Context should be inserted AFTER the --- separator
        let sep_pos = result.find("---\n").unwrap();
        let caller_pos = result.find("Called by:").unwrap();
        assert!(
            caller_pos > sep_pos,
            "caller context should be after header separator"
        );
    }

    #[test]
    fn test_enrich_caps_at_five() {
        let callers: Vec<String> = (0..10).map(|i| format!("fn_{i}")).collect();
        let result = enrich_chunk_with_graph_context("body", &callers, &[]);
        // Should only include first 5
        assert!(result.contains("fn_0"));
        assert!(result.contains("fn_4"));
        assert!(!result.contains("fn_5"));
    }
}
