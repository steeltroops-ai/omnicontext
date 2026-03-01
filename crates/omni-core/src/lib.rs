//! # omni-core
//!
//! Core indexing, search, and code intelligence engine for OmniContext.
//!
//! This crate provides the foundational components for building a semantic
//! code understanding engine. It is designed as a library with clear module
//! boundaries so that each subsystem can be developed, tested, and debugged
//! independently.
//!
//! ## Architecture
//!
//! The engine is split into decoupled subsystems:
//!
//! - **`config`** -- Configuration loading and validation
//! - **`parser`** -- Tree-sitter AST parsing with per-language analyzers
//! - **`chunker`** -- AST-aware semantic code chunking
//! - **`embedder`** -- ONNX-based local embedding inference
//! - **`index`** -- SQLite metadata store + FTS5 full-text search
//! - **`vector`** -- Flat vector index with disk persistence (HNSW planned)
//! - **`graph`** -- Dependency graph construction and traversal (petgraph)
//! - **`search`** -- Hybrid retrieval engine (RRF fusion + ranking)
//! - **`watcher`** -- File system watcher with debouncing
//! - **`pipeline`** -- Orchestrates the ingestion pipeline
//! - **`workspace`** -- Multi-repo workspace management (Pro)
//! - **`commits`** -- Git commit lineage indexing (Pro)
//! - **`patterns`** -- Convention and pattern recognition (Pro)
//! - **`server`** -- REST API server for enterprise deployment
//!
//! Each module exposes a public trait or struct that the pipeline wires together.
//! Modules communicate via well-defined types in the `types` module.
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::doc_link_with_quotes,
    clippy::doc_markdown,
    clippy::format_push_string,
    clippy::if_not_else,
    clippy::if_same_then_else,
    clippy::inefficient_to_string,
    clippy::items_after_statements,
    clippy::let_and_return,
    clippy::manual_let_else,
    clippy::manual_pattern_char_comparison,
    clippy::manual_strip,
    clippy::map_entry,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::non_canonical_partial_ord_impl,
    clippy::redundant_closure_for_method_calls,
    clippy::self_only_used_in_recursion,
    clippy::single_char_pattern,
    clippy::single_match,
    clippy::struct_field_names,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::unnecessary_wraps,
    clippy::uninlined_format_args,
    clippy::unnecessary_literal_bound,
    clippy::unused_self
)]

// Workspace lints are inherited from Cargo.toml

pub mod config;
pub mod error;
pub mod types;

// Core subsystems
pub mod parser;
pub mod chunker;
pub mod embedder;
#[allow(missing_docs)]
pub mod reranker;
pub mod index;
pub mod vector;
pub mod graph;
pub mod search;
pub mod watcher;
pub mod pipeline;

// Pro features (Phase 7)
pub mod workspace;
pub mod commits;
pub mod patterns;

// Enterprise features (Phase 8)
pub mod server;

/// Re-export the primary engine interface.
pub use pipeline::Engine;
pub use config::Config;
pub use error::OmniError;
