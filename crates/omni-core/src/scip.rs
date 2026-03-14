//! SCIP (Source Code Intelligence Protocol) export and import.
//!
//! Implements a JSON-compatible representation of the SCIP index format,
//! allowing OmniContext to interoperate with the Sourcegraph ecosystem.
//!
//! The SCIP format represents a codebase as:
//! - **Documents** — one per file, containing occurrences and symbol information
//! - **Symbols** — fully-qualified names following the SCIP symbol syntax
//! - **Occurrences** — (range, symbol, role) tuples encoding every reference
//! - **Relationships** — typed edges between symbols (reference, implementation, type definition)
//!
//! This module uses a JSON wire format instead of protobuf, preserving full
//! SCIP schema compatibility while avoiding a proto toolchain dependency.
//!
//! Reference: <https://github.com/sourcegraph/scip>

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{OmniError, OmniResult};

// ---------------------------------------------------------------------------
// SCIP schema types
// ---------------------------------------------------------------------------

/// Top-level SCIP index.
///
/// Contains all documents produced by an indexer run and optional information
/// about symbols that are referenced but defined in external packages.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScipIndex {
    /// Index metadata (tool info, project root, encoding).
    pub metadata: ScipMetadata,
    /// One document per indexed source file.
    pub documents: Vec<ScipDocument>,
    /// External symbols referenced but not defined in this index.
    #[serde(default)]
    pub external_symbols: Vec<ScipSymbolInformation>,
}

/// Metadata describing the tool and project that produced the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScipMetadata {
    /// Tool that generated this index.
    pub tool_info: ScipToolInfo,
    /// Project root URI (e.g., `"file:///home/user/project"`).
    pub project_root: String,
    /// Text document encoding — always `"UTF-8"` for this implementation.
    #[serde(default = "ScipMetadata::default_encoding")]
    pub text_document_encoding: String,
}

impl ScipMetadata {
    /// Returns the default text document encoding (`"UTF-8"`).
    pub fn default_encoding() -> String {
        "UTF-8".to_string()
    }
}

impl Default for ScipMetadata {
    fn default() -> Self {
        Self {
            tool_info: ScipToolInfo {
                name: "omnicontext".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                arguments: Vec::new(),
            },
            project_root: String::new(),
            text_document_encoding: Self::default_encoding(),
        }
    }
}

/// Identifies the indexer that produced the SCIP index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScipToolInfo {
    /// Tool name (e.g., `"omnicontext"`).
    pub name: String,
    /// Tool version string.
    pub version: String,
    /// Optional command-line arguments that were passed to the tool.
    #[serde(default)]
    pub arguments: Vec<String>,
}

/// A single source file and all its SCIP information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScipDocument {
    /// Relative path from the project root (e.g., `"src/main.rs"`).
    pub relative_path: String,
    /// Language identifier (e.g., `"rust"`, `"python"`).
    pub language: String,
    /// All symbol occurrences (references + definitions) in this document.
    #[serde(default)]
    pub occurrences: Vec<ScipOccurrence>,
    /// Symbol information (documentation, relationships) for symbols defined
    /// or referenced in this document.
    #[serde(default)]
    pub symbols: Vec<ScipSymbolInformation>,
}

/// A range encoded as a compact integer array.
///
/// The SCIP spec allows two forms:
/// - 4-element: `[start_line, start_char, end_line, end_char]`
/// - 3-element: `[start_line, start_char, end_char]` (same-line shorthand)
///
/// All values are 0-indexed.
pub type ScipRange = Vec<i32>;

/// A single symbol occurrence within a document.
///
/// Each occurrence associates a source range with a SCIP symbol string and
/// a bitmask of [`symbol_role`] flags that describe the access mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScipOccurrence {
    /// Source range, 0-indexed `[start_line, start_char, end_line, end_char]`.
    pub range: ScipRange,
    /// SCIP symbol string identifying the symbol at this location.
    pub symbol: String,
    /// Bitmask of [`symbol_role`] values describing how the symbol is used.
    #[serde(default)]
    pub symbol_roles: i32,
}

/// Information about a symbol: its documentation and typed relationships to
/// other symbols.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScipSymbolInformation {
    /// SCIP symbol string for this symbol.
    pub symbol: String,
    /// Documentation strings extracted from the source (e.g., doc comments).
    #[serde(default)]
    pub documentation: Vec<String>,
    /// Typed relationships to other symbols.
    #[serde(default)]
    pub relationships: Vec<ScipRelationship>,
}

/// A typed directed relationship between two SCIP symbols.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)] // Four bools mirror the SCIP proto schema exactly.
pub struct ScipRelationship {
    /// The related symbol string (target of this relationship).
    pub symbol: String,
    /// The source symbol references the related symbol.
    #[serde(default)]
    pub is_reference: bool,
    /// The source symbol implements the related symbol (interface/trait).
    #[serde(default)]
    pub is_implementation: bool,
    /// The source symbol is a type definition for the related symbol.
    #[serde(default)]
    pub is_type_definition: bool,
    /// The source symbol defines the related symbol.
    #[serde(default)]
    pub is_definition: bool,
}

/// SCIP symbol role bitmask constants.
///
/// Combine with bitwise OR to describe composite roles.
pub mod symbol_role {
    /// No role specified.
    pub const UNSPECIFIED: i32 = 0;
    /// The symbol is being defined at this location.
    pub const DEFINITION: i32 = 1;
    /// The symbol is being imported at this location.
    pub const IMPORT: i32 = 4;
    /// The symbol is being written (assigned) at this location.
    pub const WRITE_ACCESS: i32 = 8;
    /// The symbol is being read at this location.
    pub const READ_ACCESS: i32 = 16;
    /// The occurrence was generated by a tool, not written by a human.
    pub const GENERATED: i32 = 32;
    /// The occurrence appears in a test file.
    pub const TEST: i32 = 64;
}

// ---------------------------------------------------------------------------
// SCIP symbol string encoding
// ---------------------------------------------------------------------------

/// Convert a repository-relative file path and symbol name into a SCIP symbol
/// string.
///
/// Format: `omnicontext <escaped_path> <escaped_name>.`
///
/// Spaces are escaped as `\ `, and parentheses in the symbol name are escaped
/// as `\(` / `\)` to comply with the SCIP descriptor grammar.
fn to_scip_symbol(file_path: &str, symbol_name: &str) -> String {
    let escaped_path = file_path.replace(' ', "\\ ");
    let escaped_name = symbol_name
        .replace(' ', "\\ ")
        .replace('(', "\\(")
        .replace(')', "\\)");
    format!("omnicontext {escaped_path} {escaped_name}.")
}

// ---------------------------------------------------------------------------
// ScipExporter
// ---------------------------------------------------------------------------

/// Exports the OmniContext index as a SCIP-compatible JSON index.
///
/// The exporter reads all indexed files and symbols from the engine's
/// `MetadataIndex`, converts them to SCIP documents with definition
/// occurrences, and encodes file-level dependency edges from the
/// `FileDependencyGraph` as `ScipRelationship` entries.
pub struct ScipExporter<'a> {
    engine: &'a crate::pipeline::Engine,
}

impl<'a> ScipExporter<'a> {
    /// Create an exporter bound to the given engine.
    pub fn new(engine: &'a crate::pipeline::Engine) -> Self {
        Self { engine }
    }

    /// Export the full index as a [`ScipIndex`].
    ///
    /// Steps:
    /// 1. Build index metadata from engine configuration.
    /// 2. For each file in `MetadataIndex`, create a [`ScipDocument`] with
    ///    occurrence + symbol information entries for every indexed symbol.
    /// 3. Encode file-level dependency edges from `FileDependencyGraph` as
    ///    `ScipRelationship` entries on the source file's first symbol.
    pub fn export(&self) -> OmniResult<ScipIndex> {
        let repo_path = self.engine.repo_path();
        let project_root = format!("file://{}", repo_path.display());

        let metadata = ScipMetadata {
            tool_info: ScipToolInfo {
                name: "omnicontext".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                arguments: Vec::new(),
            },
            project_root,
            text_document_encoding: ScipMetadata::default_encoding(),
        };

        let index = self.engine.metadata_index();
        let files = index.get_all_files()?;

        tracing::info!(file_count = files.len(), "exporting SCIP index");

        let mut documents = Vec::with_capacity(files.len());

        for file in &files {
            let rel_path = file
                .path
                .strip_prefix(repo_path)
                .unwrap_or(&file.path)
                .to_string_lossy()
                // Normalise Windows backslashes to forward slashes for SCIP
                .replace('\\', "/");

            let language = format!("{:?}", file.language).to_lowercase();

            let symbols_in_file = index.get_all_symbols_for_file(file.id)?;

            let mut occurrences = Vec::with_capacity(symbols_in_file.len());
            let mut symbol_infos = Vec::with_capacity(symbols_in_file.len());

            for sym in &symbols_in_file {
                let scip_sym = to_scip_symbol(&rel_path, &sym.fqn);
                let line = sym.line as i32;

                // Emit a single-line occurrence starting at column 0 using
                // the 3-element short form [line, start_char, end_char].
                let symbol_len = sym.name.len() as i32;
                let occurrence = ScipOccurrence {
                    range: vec![line, 0, symbol_len],
                    symbol: scip_sym.clone(),
                    symbol_roles: symbol_role::DEFINITION,
                };

                let doc_strings = if sym.fqn.is_empty() {
                    Vec::new()
                } else {
                    // Include the kind as lightweight documentation.
                    vec![format!("{:?}", sym.kind)]
                };

                let sym_info = ScipSymbolInformation {
                    symbol: scip_sym,
                    documentation: doc_strings,
                    relationships: Vec::new(),
                };

                occurrences.push(occurrence);
                symbol_infos.push(sym_info);
            }

            documents.push(ScipDocument {
                relative_path: rel_path,
                language,
                occurrences,
                symbols: symbol_infos,
            });
        }

        // Encode file-level dependency edges from the FileDependencyGraph as
        // SCIP relationships between the first symbols of the respective files.
        // We iterate all edge types that are structurally meaningful.
        let edge_types = [
            crate::graph::dependencies::EdgeType::Imports,
            crate::graph::dependencies::EdgeType::Inherits,
            crate::graph::dependencies::EdgeType::Calls,
            crate::graph::dependencies::EdgeType::Instantiates,
        ];

        // Build a lookup: normalised relative path → document index.
        let mut path_to_doc: std::collections::HashMap<String, usize> =
            std::collections::HashMap::with_capacity(documents.len());
        for (i, doc) in documents.iter().enumerate() {
            path_to_doc.insert(doc.relative_path.clone(), i);
        }

        let mut total_relationships: usize = 0;

        for edge_type in &edge_types {
            let edges = self.engine.file_dep_graph().all_edges_of_type(*edge_type);
            for edge in edges {
                let src_rel = edge
                    .source
                    .strip_prefix(repo_path)
                    .unwrap_or(&edge.source)
                    .to_string_lossy()
                    .replace('\\', "/");

                let tgt_rel = edge
                    .target
                    .strip_prefix(repo_path)
                    .unwrap_or(&edge.target)
                    .to_string_lossy()
                    .replace('\\', "/");

                // Attach relationship to the first occurrence symbol of the
                // source document, if it exists.
                if let Some(&src_idx) = path_to_doc.get(&src_rel) {
                    if let Some(first_sym) = documents[src_idx].symbols.first().cloned() {
                        let tgt_symbol = to_scip_symbol(&tgt_rel, &tgt_rel);
                        let rel = ScipRelationship {
                            symbol: tgt_symbol,
                            is_reference: matches!(
                                edge_type,
                                crate::graph::dependencies::EdgeType::Imports
                                    | crate::graph::dependencies::EdgeType::Calls
                            ),
                            is_implementation: matches!(
                                edge_type,
                                crate::graph::dependencies::EdgeType::Inherits
                            ),
                            is_type_definition: false,
                            is_definition: matches!(
                                edge_type,
                                crate::graph::dependencies::EdgeType::Instantiates
                            ),
                        };

                        // Find the matching ScipSymbolInformation and append.
                        if let Some(sym_info) = documents[src_idx]
                            .symbols
                            .iter_mut()
                            .find(|s| s.symbol == first_sym.symbol)
                        {
                            sym_info.relationships.push(rel);
                            total_relationships += 1;
                        }
                    }
                }
            }
        }

        tracing::info!(
            documents = documents.len(),
            relationships = total_relationships,
            "SCIP export complete"
        );

        Ok(ScipIndex {
            metadata,
            documents,
            external_symbols: Vec::new(),
        })
    }

    /// Write the SCIP index to a JSON file at `output_path`.
    pub fn write_to_file(&self, output_path: &Path) -> OmniResult<()> {
        let index = self.export()?;
        save_scip_index(&index, output_path)
    }
}

// ---------------------------------------------------------------------------
// ScipImporter
// ---------------------------------------------------------------------------

/// Statistics gathered from a SCIP import operation.
#[derive(Debug, Default)]
pub struct ScipImportStats {
    /// Number of documents successfully parsed.
    pub documents_imported: usize,
    /// Total number of symbol information entries across all documents.
    pub symbols_imported: usize,
    /// Total number of relationships across all documents.
    pub relationships_imported: usize,
    /// Number of documents or symbols that could not be processed.
    pub errors: usize,
}

/// Imports a SCIP JSON index into OmniContext.
///
/// The current implementation is a **dry-run v1**: it parses and validates the
/// full SCIP index and counts all entities into [`ScipImportStats`], but does
/// not write to the SQLite metadata store.
///
/// Actual upsert is gated behind `MetadataIndex` write APIs that do not yet
/// exist (tracked in the roadmap as P1.3).  Each import site is annotated with
/// `// TODO: wire to MetadataIndex.upsert_file()` so the upgrade is mechanical.
pub struct ScipImporter<'a> {
    engine: &'a mut crate::pipeline::Engine,
}

impl<'a> ScipImporter<'a> {
    /// Create an importer bound to the given engine.
    pub fn new(engine: &'a mut crate::pipeline::Engine) -> Self {
        Self { engine }
    }

    /// Import a SCIP JSON index from `input_path`.
    pub fn import_from_file(&mut self, input_path: &Path) -> OmniResult<ScipImportStats> {
        let content = std::fs::read_to_string(input_path)?;
        self.import_from_str(&content)
    }

    /// Import a SCIP JSON index from a JSON string.
    pub fn import_from_str(&mut self, json_str: &str) -> OmniResult<ScipImportStats> {
        let index: ScipIndex =
            serde_json::from_str(json_str).map_err(|e| OmniError::Serialization(e.to_string()))?;
        self.import_index(&index)
    }

    /// Parse, validate, and count a [`ScipIndex`].
    ///
    /// For each document the importer:
    /// 1. Resolves the absolute path from the repo root.
    /// 2. Validates that the document has a non-empty `relative_path`.
    /// 3. Counts symbols and relationships into [`ScipImportStats`].
    /// 4. Logs what _would_ be imported at `info` level.
    ///
    /// Actual database writes are marked `// TODO: wire to MetadataIndex.upsert_file()`
    /// and will be activated in a follow-up once `MetadataIndex` gains upsert APIs.
    pub fn import_index(&mut self, index: &ScipIndex) -> OmniResult<ScipImportStats> {
        let mut stats = ScipImportStats::default();
        let repo_root: PathBuf = self.engine.repo_path().to_path_buf();

        tracing::info!(
            project_root = %index.metadata.project_root,
            tool = %index.metadata.tool_info.name,
            tool_version = %index.metadata.tool_info.version,
            documents = index.documents.len(),
            external_symbols = index.external_symbols.len(),
            "importing SCIP index (dry-run)"
        );

        for doc in &index.documents {
            // Validate document has a path.
            if doc.relative_path.is_empty() {
                tracing::warn!("skipping SCIP document with empty relative_path");
                stats.errors += 1;
                continue;
            }

            let abs_path = repo_root.join(&doc.relative_path);

            tracing::debug!(
                path = %abs_path.display(),
                language = %doc.language,
                occurrences = doc.occurrences.len(),
                symbols = doc.symbols.len(),
                "would import document"
            );

            // TODO: wire to MetadataIndex.upsert_file() once write APIs exist.
            // let file_info = FileInfo {
            //     id: 0,
            //     path: abs_path,
            //     language: Language::from_str(&doc.language),
            //     content_hash: String::new(),
            //     size_bytes: 0,
            // };
            // self.engine.metadata_index_mut().upsert_file(&file_info)?;

            let doc_relationships: usize = doc.symbols.iter().map(|s| s.relationships.len()).sum();

            stats.documents_imported += 1;
            stats.symbols_imported += doc.symbols.len();
            stats.relationships_imported += doc_relationships;

            // TODO: wire symbol upsert once MetadataIndex.upsert_symbols() exists.
            // for sym_info in &doc.symbols {
            //     // Parse SCIP symbol string back to name components.
            //     // Insert into metadata index.
            // }
        }

        // Count external symbol relationships too.
        for ext in &index.external_symbols {
            stats.relationships_imported += ext.relationships.len();
        }

        tracing::info!(
            documents_imported = stats.documents_imported,
            symbols_imported = stats.symbols_imported,
            relationships_imported = stats.relationships_imported,
            errors = stats.errors,
            "SCIP import dry-run complete"
        );

        Ok(stats)
    }
}

// ---------------------------------------------------------------------------
// Standalone file I/O helpers
// ---------------------------------------------------------------------------

/// Load a SCIP index from a JSON file.
///
/// Does not require an `Engine` — useful for tooling and tests that only need
/// to read or inspect an index file.
pub fn load_scip_index(path: &Path) -> OmniResult<ScipIndex> {
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|e| OmniError::Serialization(e.to_string()))
}

/// Save a SCIP index to a JSON file, creating parent directories as needed.
pub fn save_scip_index(index: &ScipIndex, path: &Path) -> OmniResult<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let json =
        serde_json::to_string_pretty(index).map_err(|e| OmniError::Serialization(e.to_string()))?;
    std::fs::write(path, json)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    // -----------------------------------------------------------------------
    // Schema / serialization round-trips
    // -----------------------------------------------------------------------

    #[test]
    fn test_scip_index_serializes_to_json() {
        let index = ScipIndex::default();
        let json = serde_json::to_string(&index).expect("serialize");
        assert!(json.contains("metadata"));
        assert!(json.contains("documents"));
    }

    #[test]
    fn test_scip_index_roundtrip() {
        let mut index = ScipIndex::default();
        index.metadata.project_root = "file:///home/user/project".to_string();
        index.documents.push(ScipDocument {
            relative_path: "src/main.rs".to_string(),
            language: "rust".to_string(),
            occurrences: vec![ScipOccurrence {
                range: vec![0, 0, 4],
                symbol: "omnicontext src/main.rs main.".to_string(),
                symbol_roles: symbol_role::DEFINITION,
            }],
            symbols: vec![ScipSymbolInformation {
                symbol: "omnicontext src/main.rs main.".to_string(),
                documentation: vec!["Function".to_string()],
                relationships: Vec::new(),
            }],
        });

        let json = serde_json::to_string_pretty(&index).expect("serialize");
        let recovered: ScipIndex = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(recovered.metadata.project_root, "file:///home/user/project");
        assert_eq!(recovered.documents.len(), 1);
        assert_eq!(recovered.documents[0].relative_path, "src/main.rs");
        assert_eq!(recovered.documents[0].occurrences.len(), 1);
        assert_eq!(recovered.documents[0].occurrences[0].range, vec![0, 0, 4]);
    }

    #[test]
    fn test_scip_metadata_default_encoding() {
        let enc = ScipMetadata::default_encoding();
        assert_eq!(enc, "UTF-8");
    }

    #[test]
    fn test_scip_metadata_default_impl() {
        let meta = ScipMetadata::default();
        assert_eq!(meta.text_document_encoding, "UTF-8");
        assert_eq!(meta.tool_info.name, "omnicontext");
        assert!(meta.project_root.is_empty());
    }

    // -----------------------------------------------------------------------
    // Symbol string conversion
    // -----------------------------------------------------------------------

    #[test]
    fn test_scip_symbol_conversion() {
        let sym = to_scip_symbol("src/lib.rs", "my_function");
        assert_eq!(sym, "omnicontext src/lib.rs my_function.");
    }

    #[test]
    fn test_scip_symbol_escapes_spaces() {
        let sym = to_scip_symbol("src/my file.rs", "my function");
        assert_eq!(sym, "omnicontext src/my\\ file.rs my\\ function.");
    }

    #[test]
    fn test_scip_symbol_escapes_parens() {
        let sym = to_scip_symbol("src/lib.rs", "op(T)");
        assert_eq!(sym, "omnicontext src/lib.rs op\\(T\\).");
    }

    #[test]
    fn test_scip_symbol_escapes_both() {
        let sym = to_scip_symbol("src/my module.rs", "fn (args)");
        assert_eq!(sym, "omnicontext src/my\\ module.rs fn\\ \\(args\\).");
    }

    // -----------------------------------------------------------------------
    // File I/O helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_save_roundtrip() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("index.scip.json");

        let mut index = ScipIndex::default();
        index.metadata.project_root = "file:///tmp/project".to_string();
        index.documents.push(ScipDocument {
            relative_path: "foo.rs".to_string(),
            language: "rust".to_string(),
            occurrences: Vec::new(),
            symbols: Vec::new(),
        });

        save_scip_index(&index, &path).expect("save");
        let loaded = load_scip_index(&path).expect("load");

        assert_eq!(loaded.metadata.project_root, "file:///tmp/project");
        assert_eq!(loaded.documents.len(), 1);
        assert_eq!(loaded.documents[0].relative_path, "foo.rs");
    }

    #[test]
    fn test_load_from_nonexistent_file_returns_err() {
        let path = PathBuf::from("/nonexistent/path/index.scip.json");
        let result = load_scip_index(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_creates_parent_dirs() {
        let tmp = TempDir::new().expect("tempdir");
        let nested = tmp.path().join("a").join("b").join("index.scip.json");

        let index = ScipIndex::default();
        save_scip_index(&index, &nested).expect("save with nested dirs");
        assert!(nested.exists());
    }

    // -----------------------------------------------------------------------
    // Import dry-run
    // -----------------------------------------------------------------------

    #[test]
    fn test_import_stats_default() {
        let stats = ScipImportStats::default();
        assert_eq!(stats.documents_imported, 0);
        assert_eq!(stats.symbols_imported, 0);
        assert_eq!(stats.relationships_imported, 0);
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn test_import_from_str_valid_json() {
        let tmp = TempDir::new().expect("tempdir");
        let mut engine = crate::pipeline::Engine::new(tmp.path()).expect("engine");
        let mut importer = ScipImporter::new(&mut engine);

        let json = serde_json::json!({
            "metadata": {
                "tool_info": { "name": "test-tool", "version": "1.0.0" },
                "project_root": "file:///tmp/proj",
                "text_document_encoding": "UTF-8"
            },
            "documents": [
                {
                    "relative_path": "src/lib.rs",
                    "language": "rust",
                    "occurrences": [],
                    "symbols": [
                        {
                            "symbol": "omnicontext src/lib.rs MyStruct.",
                            "documentation": ["Struct"],
                            "relationships": []
                        }
                    ]
                }
            ],
            "external_symbols": []
        })
        .to_string();

        let stats = importer.import_from_str(&json).expect("import");
        assert_eq!(stats.documents_imported, 1);
        assert_eq!(stats.symbols_imported, 1);
        assert_eq!(stats.relationships_imported, 0);
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn test_import_from_str_invalid_json_returns_err() {
        let tmp = TempDir::new().expect("tempdir");
        let mut engine = crate::pipeline::Engine::new(tmp.path()).expect("engine");
        let mut importer = ScipImporter::new(&mut engine);

        let result = importer.import_from_str("{ not valid json }}}");
        assert!(result.is_err());
    }

    #[test]
    fn test_import_empty_relative_path_counts_as_error() {
        let tmp = TempDir::new().expect("tempdir");
        let mut engine = crate::pipeline::Engine::new(tmp.path()).expect("engine");
        let mut importer = ScipImporter::new(&mut engine);

        let json = serde_json::json!({
            "metadata": {
                "tool_info": { "name": "test-tool", "version": "1.0.0" },
                "project_root": "file:///tmp/proj",
                "text_document_encoding": "UTF-8"
            },
            "documents": [
                {
                    "relative_path": "",
                    "language": "rust",
                    "occurrences": [],
                    "symbols": []
                }
            ],
            "external_symbols": []
        })
        .to_string();

        let stats = importer.import_from_str(&json).expect("import");
        assert_eq!(stats.documents_imported, 0);
        assert_eq!(stats.errors, 1);
    }

    #[test]
    fn test_import_counts_relationships() {
        let tmp = TempDir::new().expect("tempdir");
        let mut engine = crate::pipeline::Engine::new(tmp.path()).expect("engine");
        let mut importer = ScipImporter::new(&mut engine);

        let json = serde_json::json!({
            "metadata": {
                "tool_info": { "name": "test-tool", "version": "1.0.0" },
                "project_root": "file:///tmp/proj",
                "text_document_encoding": "UTF-8"
            },
            "documents": [
                {
                    "relative_path": "src/a.rs",
                    "language": "rust",
                    "occurrences": [],
                    "symbols": [
                        {
                            "symbol": "omnicontext src/a.rs A.",
                            "documentation": [],
                            "relationships": [
                                {
                                    "symbol": "omnicontext src/b.rs B.",
                                    "is_reference": true,
                                    "is_implementation": false,
                                    "is_type_definition": false,
                                    "is_definition": false
                                },
                                {
                                    "symbol": "omnicontext src/c.rs C.",
                                    "is_reference": false,
                                    "is_implementation": true,
                                    "is_type_definition": false,
                                    "is_definition": false
                                }
                            ]
                        }
                    ]
                }
            ],
            "external_symbols": []
        })
        .to_string();

        let stats = importer.import_from_str(&json).expect("import");
        assert_eq!(stats.documents_imported, 1);
        assert_eq!(stats.symbols_imported, 1);
        assert_eq!(stats.relationships_imported, 2);
        assert_eq!(stats.errors, 0);
    }

    // -----------------------------------------------------------------------
    // Structural / type constants
    // -----------------------------------------------------------------------

    #[test]
    fn test_scip_document_default() {
        let doc = ScipDocument::default();
        assert!(doc.relative_path.is_empty());
        assert!(doc.language.is_empty());
        assert!(doc.occurrences.is_empty());
        assert!(doc.symbols.is_empty());
    }

    #[test]
    fn test_symbol_role_constants() {
        assert_eq!(symbol_role::UNSPECIFIED, 0);
        assert_eq!(symbol_role::DEFINITION, 1);
        assert_eq!(symbol_role::IMPORT, 4);
        assert_eq!(symbol_role::WRITE_ACCESS, 8);
        assert_eq!(symbol_role::READ_ACCESS, 16);
        assert_eq!(symbol_role::GENERATED, 32);
        assert_eq!(symbol_role::TEST, 64);

        // Bit isolation: no two constants share a bit.
        let all = [
            symbol_role::DEFINITION,
            symbol_role::IMPORT,
            symbol_role::WRITE_ACCESS,
            symbol_role::READ_ACCESS,
            symbol_role::GENERATED,
            symbol_role::TEST,
        ];
        for (i, &a) in all.iter().enumerate() {
            for (j, &b) in all.iter().enumerate() {
                if i != j {
                    assert_eq!(a & b, 0, "roles {i} and {j} share bits");
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Exporter with empty engine
    // -----------------------------------------------------------------------

    #[test]
    fn test_exporter_on_empty_engine_returns_empty_index() {
        let tmp = TempDir::new().expect("tempdir");
        let engine = crate::pipeline::Engine::new(tmp.path()).expect("engine");
        let exporter = ScipExporter::new(&engine);

        let index = exporter.export().expect("export");
        assert_eq!(index.documents.len(), 0);
        assert_eq!(index.external_symbols.len(), 0);
        assert_eq!(index.metadata.tool_info.name, "omnicontext");
    }

    #[test]
    fn test_exporter_write_to_file() {
        let tmp = TempDir::new().expect("tempdir");
        let engine = crate::pipeline::Engine::new(tmp.path()).expect("engine");
        let exporter = ScipExporter::new(&engine);

        let output = tmp.path().join("out.scip.json");
        exporter.write_to_file(&output).expect("write");
        assert!(output.exists());

        let loaded = load_scip_index(&output).expect("reload");
        assert_eq!(loaded.documents.len(), 0);
    }
}
