//! MCP transport backends.
//!
//! Two transports are supported:
//!
//! - **stdio** (default): line-delimited JSON-RPC over standard input/output.
//!   Used by AI agent launchers (Claude Desktop, Cursor, Windsurf, Zed).
//!
//! - **sse** (feature = "sse"): HTTP Server-Sent Events over axum.
//!   Used for remote/enterprise deployments where the MCP server runs as a
//!   persistent daemon reachable over the network.
//!   Enable with: `cargo build --features sse`

#[cfg(feature = "sse")]
pub mod sse;
