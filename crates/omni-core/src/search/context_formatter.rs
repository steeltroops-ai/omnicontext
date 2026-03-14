//! Prompt-optimized context formatting for LLM consumption.
//!
//! Transforms [`ContextWindow`] entries into output formats tuned for
//! different LLM providers and use cases:
//!
//! - **Structured XML** — Clear tags with metadata for tool-use-capable models
//! - **Compact** — Minimal framing, maximum code density
//! - **Annotated Markdown** — Rich fenced code blocks with metadata
//!
//! The formatter also supports:
//! - **File grouping** — chunks from the same file are merged under one header
//! - **Priority sections** — critical/high chunks are separated from background context
//! - **Token-efficient headers** — short file indicators instead of verbose paths
//! - **Deduplication hints** — marks overlapping chunks to avoid LLM confusion

#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::must_use_candidate
)]

use crate::types::{ChunkPriority, ContextEntry, ContextWindow};

/// Output format for the context formatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextFormat {
    /// XML-like tags with metadata attributes.
    /// Best for tool-use models (Claude, GPT-4) that parse structured context.
    ///
    /// ```xml
    /// <context_window query="..." entries="5" tokens="3200">
    ///   <file path="src/auth.rs" language="rust">
    ///     <chunk lines="10-45" symbol="auth::validate" relevance="0.95">
    ///       fn validate(...) { ... }
    ///     </chunk>
    ///   </file>
    /// </context_window>
    /// ```
    StructuredXml,

    /// Minimal overhead: just file dividers and raw code.
    /// Best when token budget is tight and the model doesn't need metadata.
    ///
    /// ```text
    /// --- src/auth.rs:10-45 ---
    /// fn validate(...) { ... }
    /// ```
    Compact,

    /// Markdown with fenced code blocks and metadata comments.
    /// Best for chat-based models and human-readable output.
    ///
    /// ````markdown
    /// ## src/auth.rs (lines 10-45, relevance: 0.95)
    /// ```rust
    /// fn validate(...) { ... }
    /// ```
    /// ````
    AnnotatedMarkdown,
}

/// Options for context formatting.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Output format.
    pub format: ContextFormat,
    /// Whether to include relevance scores in output.
    pub show_scores: bool,
    /// Whether to group chunks by file.
    pub group_by_file: bool,
    /// Whether to separate priority sections (critical/high vs medium/low).
    pub priority_sections: bool,
    /// The original query (included in structured output for context).
    pub query: String,
    /// Maximum path depth to show (e.g., 2 = "auth/middleware.rs" instead of full path).
    pub path_depth: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            format: ContextFormat::StructuredXml,
            show_scores: true,
            group_by_file: true,
            priority_sections: true,
            query: String::new(),
            path_depth: 0, // 0 = show full path
        }
    }
}

/// Prompt-optimized context formatter.
pub struct ContextFormatter;

impl ContextFormatter {
    /// Format a context window into a prompt-optimized string.
    pub fn format(window: &ContextWindow, options: &FormatOptions) -> String {
        match options.format {
            ContextFormat::StructuredXml => Self::format_xml(window, options),
            ContextFormat::Compact => Self::format_compact(window, options),
            ContextFormat::AnnotatedMarkdown => Self::format_markdown(window, options),
        }
    }

    /// Structured XML format with metadata attributes.
    fn format_xml(window: &ContextWindow, options: &FormatOptions) -> String {
        let mut out = String::with_capacity(window.total_tokens as usize * 4);

        // Root element
        out.push_str("<context_window");
        if !options.query.is_empty() {
            out.push_str(&format!(" query=\"{}\"", escape_xml(&options.query)));
        }
        out.push_str(&format!(
            " entries=\"{}\" tokens=\"{}\" budget=\"{}\">",
            window.entries.len(),
            window.total_tokens,
            window.token_budget
        ));
        out.push('\n');

        if options.priority_sections {
            // Split into priority sections
            let (critical, rest): (Vec<_>, Vec<_>) = window.entries.iter().partition(|e| {
                matches!(
                    e.priority,
                    Some(ChunkPriority::Critical | ChunkPriority::High)
                )
            });

            if !critical.is_empty() {
                out.push_str("  <primary_context>\n");
                Self::format_xml_entries(&mut out, &critical, options);
                out.push_str("  </primary_context>\n");
            }

            if !rest.is_empty() {
                out.push_str("  <supporting_context>\n");
                Self::format_xml_entries(&mut out, &rest, options);
                out.push_str("  </supporting_context>\n");
            }
        } else {
            let entries: Vec<_> = window.entries.iter().collect();
            Self::format_xml_entries(&mut out, &entries, options);
        }

        out.push_str("</context_window>");
        out
    }

    fn format_xml_entries(out: &mut String, entries: &[&ContextEntry], options: &FormatOptions) {
        if options.group_by_file {
            let grouped = group_entries_by_file(entries);
            for (file_path, file_entries) in &grouped {
                let display_path = truncate_path(file_path, options.path_depth);
                let lang = detect_language(file_path);
                out.push_str(&format!("    <file path=\"{display_path}\""));
                if !lang.is_empty() {
                    out.push_str(&format!(" language=\"{lang}\""));
                }
                out.push_str(">\n");
                for entry in file_entries {
                    Self::format_xml_chunk(out, entry, options, "      ");
                }
                out.push_str("    </file>\n");
            }
        } else {
            for entry in entries {
                let display_path =
                    truncate_path(&entry.file_path.to_string_lossy(), options.path_depth);
                let lang = detect_language(&entry.file_path.to_string_lossy());
                out.push_str(&format!("    <file path=\"{display_path}\""));
                if !lang.is_empty() {
                    out.push_str(&format!(" language=\"{lang}\""));
                }
                out.push_str(">\n");
                Self::format_xml_chunk(out, entry, options, "      ");
                out.push_str("    </file>\n");
            }
        }
    }

    fn format_xml_chunk(
        out: &mut String,
        entry: &ContextEntry,
        options: &FormatOptions,
        indent: &str,
    ) {
        out.push_str(&format!(
            "{indent}<chunk lines=\"{}-{}\"",
            entry.chunk.line_start, entry.chunk.line_end
        ));
        if !entry.chunk.symbol_path.is_empty() {
            out.push_str(&format!(
                " symbol=\"{}\"",
                escape_xml(&entry.chunk.symbol_path)
            ));
        }
        if options.show_scores {
            out.push_str(&format!(" relevance=\"{:.2}\"", entry.score));
        }
        if entry.is_graph_neighbor {
            out.push_str(" source=\"graph\"");
        }
        out.push_str(">\n");

        // Shadow header if present
        if let Some(ref header) = entry.shadow_header {
            out.push_str(&format!("{indent}  {}\n", escape_xml(header)));
        }

        // Code content — each line indented and XML-escaped
        for line in entry.chunk.content.lines() {
            out.push_str(indent);
            out.push_str("  ");
            out.push_str(&escape_xml(line));
            out.push('\n');
        }
        out.push_str(&format!("{indent}</chunk>\n"));
    }

    /// Compact format: minimal framing, maximum code density.
    fn format_compact(window: &ContextWindow, options: &FormatOptions) -> String {
        let mut out = String::with_capacity(window.total_tokens as usize * 4);
        let mut current_file: Option<String> = None;

        for entry in &window.entries {
            let path = truncate_path(&entry.file_path.to_string_lossy(), options.path_depth);

            if current_file.as_deref() != Some(&path) {
                if current_file.is_some() {
                    out.push('\n');
                }
                out.push_str(&format!(
                    "--- {path}:{}-{} ---\n",
                    entry.chunk.line_start, entry.chunk.line_end
                ));
                current_file = Some(path);
            } else {
                out.push_str(&format!(
                    "--- :{}-{} ---\n",
                    entry.chunk.line_start, entry.chunk.line_end
                ));
            }

            if let Some(ref header) = entry.shadow_header {
                out.push_str(header);
                out.push('\n');
            }
            out.push_str(&entry.chunk.content);
            out.push('\n');
        }

        out
    }

    /// Annotated Markdown: fenced code blocks with metadata.
    fn format_markdown(window: &ContextWindow, options: &FormatOptions) -> String {
        let mut out = String::with_capacity(window.total_tokens as usize * 5);

        if !options.query.is_empty() {
            out.push_str(&format!("# Context for: {}\n\n", options.query));
        }

        out.push_str(&format!(
            "__{} code chunks, {} tokens used of {} budget__\n\n",
            window.entries.len(),
            window.total_tokens,
            window.token_budget
        ));

        if options.group_by_file {
            let entries: Vec<_> = window.entries.iter().collect();
            let grouped = group_entries_by_file(&entries);

            for (file_path, file_entries) in &grouped {
                let display_path = truncate_path(file_path, options.path_depth);
                let lang = detect_language(file_path);

                out.push_str(&format!("## {display_path}\n\n"));

                for entry in file_entries {
                    let mut meta_parts = vec![format!(
                        "lines {}-{}",
                        entry.chunk.line_start, entry.chunk.line_end
                    )];
                    if !entry.chunk.symbol_path.is_empty() {
                        meta_parts.push(format!("`{}`", entry.chunk.symbol_path));
                    }
                    if options.show_scores {
                        meta_parts.push(format!("relevance: {:.2}", entry.score));
                    }
                    if entry.is_graph_neighbor {
                        meta_parts.push("via graph".to_string());
                    }

                    out.push_str(&format!("_{}_\n", meta_parts.join(" | ")));

                    if let Some(ref header) = entry.shadow_header {
                        out.push_str(&format!("> {header}\n"));
                    }

                    out.push_str(&format!("```{lang}\n"));
                    out.push_str(&entry.chunk.content);
                    if !entry.chunk.content.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str("```\n\n");
                }
            }
        } else {
            for entry in &window.entries {
                let display_path =
                    truncate_path(&entry.file_path.to_string_lossy(), options.path_depth);
                let lang = detect_language(&entry.file_path.to_string_lossy());

                out.push_str(&format!("### {display_path}\n"));
                out.push_str(&format!(
                    "_lines {}-{}_\n",
                    entry.chunk.line_start, entry.chunk.line_end
                ));
                out.push_str(&format!("```{lang}\n"));
                out.push_str(&entry.chunk.content);
                if !entry.chunk.content.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str("```\n\n");
            }
        }

        out
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Group entries by file path, maintaining order of first appearance.
fn group_entries_by_file<'a>(entries: &[&'a ContextEntry]) -> Vec<(String, Vec<&'a ContextEntry>)> {
    let mut groups: Vec<(String, Vec<&'a ContextEntry>)> = Vec::new();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for entry in entries {
        let path = entry.file_path.to_string_lossy().to_string();
        if let Some(&idx) = seen.get(&path) {
            groups[idx].1.push(entry);
        } else {
            let idx = groups.len();
            seen.insert(path.clone(), idx);
            groups.push((path, vec![entry]));
        }
    }

    groups
}

/// Truncate a file path to show only the last N components.
/// If `depth` is 0, return the full path.
fn truncate_path(path: &str, depth: usize) -> String {
    if depth == 0 {
        return path.to_string();
    }
    let parts: Vec<&str> = path.split(['/', '\\']).collect();
    if parts.len() <= depth {
        return path.to_string();
    }
    parts[parts.len() - depth..].join("/")
}

/// Detect language from file extension for code fencing.
fn detect_language(path: &str) -> String {
    if let Some(ext) = path.rsplit('.').next() {
        match ext {
            "rs" => "rust",
            "py" => "python",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "go" => "go",
            "java" => "java",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" => "cpp",
            "cs" => "csharp",
            "rb" => "ruby",
            "php" => "php",
            "swift" => "swift",
            "kt" | "kts" => "kotlin",
            "sql" => "sql",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "json" => "json",
            "md" => "markdown",
            "sh" | "bash" => "bash",
            "css" | "scss" => "css",
            "html" | "htm" => "html",
            _ => ext,
        }
        .to_string()
    } else {
        String::new()
    }
}

/// Escape XML special characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Chunk, ChunkKind, Visibility};
    use std::path::PathBuf;

    fn test_chunk(
        file_id: i64,
        symbol: &str,
        content: &str,
        line_start: u32,
        line_end: u32,
    ) -> Chunk {
        Chunk {
            id: file_id,
            file_id,
            symbol_path: symbol.to_string(),
            kind: ChunkKind::Function,
            visibility: Visibility::Public,
            line_start,
            line_end,
            content: content.to_string(),
            doc_comment: None,
            token_count: content.len() as u32 / 4,
            weight: 1.0,
            vector_id: None,
            is_summary: false,
            content_hash: 0,
        }
    }

    fn test_entry(
        path: &str,
        symbol: &str,
        content: &str,
        score: f64,
        priority: Option<ChunkPriority>,
    ) -> ContextEntry {
        ContextEntry {
            file_path: PathBuf::from(path),
            chunk: test_chunk(1, symbol, content, 10, 20),
            score,
            is_graph_neighbor: false,
            priority,
            shadow_header: None,
        }
    }

    fn test_window() -> ContextWindow {
        ContextWindow {
            entries: vec![
                test_entry(
                    "src/auth.rs",
                    "auth::validate",
                    "fn validate(token: &str) -> bool {\n    true\n}",
                    0.95,
                    Some(ChunkPriority::Critical),
                ),
                test_entry(
                    "src/auth.rs",
                    "auth::refresh",
                    "fn refresh(session: &Session) {\n    // refresh logic\n}",
                    0.8,
                    Some(ChunkPriority::High),
                ),
                test_entry(
                    "src/db.rs",
                    "db::connect",
                    "fn connect(url: &str) -> Pool {\n    Pool::new(url)\n}",
                    0.6,
                    Some(ChunkPriority::Medium),
                ),
            ],
            total_tokens: 120,
            token_budget: 4000,
        }
    }

    #[test]
    fn test_xml_format_contains_tags() {
        let window = test_window();
        let options = FormatOptions {
            query: "how does auth work".to_string(),
            ..Default::default()
        };
        let output = ContextFormatter::format(&window, &options);

        assert!(output.contains("<context_window"), "should have root tag");
        assert!(
            output.contains("</context_window>"),
            "should close root tag"
        );
        assert!(output.contains("<file path="), "should have file tags");
        assert!(output.contains("<chunk lines="), "should have chunk tags");
        assert!(output.contains("auth::validate"), "should include symbol");
        assert!(
            output.contains("query=\"how does auth work\""),
            "should include query"
        );
    }

    #[test]
    fn test_xml_priority_sections() {
        let window = test_window();
        let options = FormatOptions {
            priority_sections: true,
            ..Default::default()
        };
        let output = ContextFormatter::format(&window, &options);

        assert!(
            output.contains("<primary_context>"),
            "should have primary section"
        );
        assert!(
            output.contains("<supporting_context>"),
            "should have supporting section"
        );
    }

    #[test]
    fn test_compact_format() {
        let window = test_window();
        let options = FormatOptions {
            format: ContextFormat::Compact,
            ..Default::default()
        };
        let output = ContextFormatter::format(&window, &options);

        assert!(
            output.contains("--- src/auth.rs:"),
            "should have file divider"
        );
        assert!(output.contains("fn validate"), "should have code");
        // Should NOT have XML tags
        assert!(!output.contains("<context_window"));
        assert!(!output.contains("```"));
    }

    #[test]
    fn test_markdown_format() {
        let window = test_window();
        let options = FormatOptions {
            format: ContextFormat::AnnotatedMarkdown,
            query: "authentication".to_string(),
            ..Default::default()
        };
        let output = ContextFormatter::format(&window, &options);

        assert!(
            output.contains("# Context for: authentication"),
            "should have header"
        );
        assert!(output.contains("```rust"), "should have rust code fence");
        assert!(
            output.contains("## src/auth.rs"),
            "should have file heading"
        );
        assert!(output.contains("relevance: 0.95"), "should show score");
    }

    #[test]
    fn test_compact_groups_same_file() {
        let window = test_window();
        let options = FormatOptions {
            format: ContextFormat::Compact,
            ..Default::default()
        };
        let output = ContextFormatter::format(&window, &options);

        // The second chunk from auth.rs should use abbreviated header
        let auth_count = output.matches("src/auth.rs").count();
        assert_eq!(
            auth_count, 1,
            "file path should appear only once for grouped chunks"
        );
    }

    #[test]
    fn test_xml_file_grouping() {
        let window = test_window();
        let options = FormatOptions {
            format: ContextFormat::StructuredXml,
            group_by_file: true,
            priority_sections: false,
            ..Default::default()
        };
        let output = ContextFormatter::format(&window, &options);

        // auth.rs file tag should appear once (grouping two chunks)
        let file_tag_count = output.matches("<file path=\"src/auth.rs\"").count();
        assert_eq!(file_tag_count, 1, "grouped file should appear once");
        // But should have two chunk tags under it
        let chunk_count = output.matches("<chunk lines=").count();
        assert_eq!(chunk_count, 3, "should have all 3 chunks");
    }

    #[test]
    fn test_no_scores() {
        let window = test_window();
        let options = FormatOptions {
            show_scores: false,
            ..Default::default()
        };
        let output = ContextFormatter::format(&window, &options);
        assert!(
            !output.contains("relevance="),
            "should not show scores when disabled"
        );
    }

    #[test]
    fn test_empty_window() {
        let window = ContextWindow {
            entries: vec![],
            total_tokens: 0,
            token_budget: 4000,
        };
        for format in [
            ContextFormat::StructuredXml,
            ContextFormat::Compact,
            ContextFormat::AnnotatedMarkdown,
        ] {
            let options = FormatOptions {
                format,
                ..Default::default()
            };
            let output = ContextFormatter::format(&window, &options);
            assert!(!output.is_empty() || format == ContextFormat::Compact);
        }
    }

    #[test]
    fn test_truncate_path() {
        assert_eq!(
            truncate_path("src/core/auth/middleware.rs", 2),
            "auth/middleware.rs"
        );
        assert_eq!(truncate_path("src/auth.rs", 2), "src/auth.rs");
        assert_eq!(truncate_path("auth.rs", 2), "auth.rs");
        assert_eq!(truncate_path("src/core/auth.rs", 0), "src/core/auth.rs");
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("src/main.rs"), "rust");
        assert_eq!(detect_language("app.py"), "python");
        assert_eq!(detect_language("index.tsx"), "typescript");
        assert_eq!(detect_language("utils.go"), "go");
        assert_eq!(detect_language("noext"), "noext"); // falls through
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("<script>"), "&lt;script&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }

    #[test]
    fn test_path_depth_in_output() {
        let window = test_window();
        let options = FormatOptions {
            format: ContextFormat::StructuredXml,
            path_depth: 1, // show only filename
            priority_sections: false,
            ..Default::default()
        };
        let output = ContextFormatter::format(&window, &options);

        assert!(
            output.contains("path=\"auth.rs\""),
            "should show only filename"
        );
        assert!(
            !output.contains("path=\"src/auth.rs\""),
            "should not show full path"
        );
    }
}
