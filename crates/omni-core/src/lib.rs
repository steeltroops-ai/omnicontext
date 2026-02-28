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
//! - **`vector`** -- usearch HNSW approximate nearest neighbor index
//! - **`graph`** -- Dependency graph construction and traversal (petgraph)
//! - **`search`** -- Hybrid retrieval engine (RRF fusion + ranking)
//! - **`watcher`** -- File system watcher with debouncing
//! - **`pipeline`** -- Orchestrates the ingestion pipeline (watcher -> parser -> chunker -> embedder -> index)
//!
//! Each module exposes a public trait or struct that the pipeline wires together.
//! Modules communicate via well-defined types in the `types` module.

// Workspace lints are inherited from Cargo.toml

pub mod config;
pub mod error;
pub mod types;

// Subsystems -- each is an independent module with its own error types
pub mod parser;
pub mod chunker;
pub mod embedder;
pub mod index;
pub mod vector;
pub mod graph;
pub mod search;
pub mod watcher;
pub mod pipeline;

/// Re-export the primary engine interface.
pub use pipeline::Engine;
pub use config::Config;
pub use error::OmniError;
