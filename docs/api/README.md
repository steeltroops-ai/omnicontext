# API Documentation

This directory will contain API documentation and integration guides for OmniContext.

## Coming Soon

- **MCP Protocol Reference** - Model Context Protocol tool specifications
- **CLI Reference** - Command-line interface documentation
- **Configuration API** - Configuration file format and options
- **Embedding API** - Embedding model integration guide
- **Search API** - Search and ranking API reference

## Current Integration Points

### MCP Server

The MCP server exposes OmniContext functionality through the Model Context Protocol.

**Available Tools**:
- `search_code` - Hybrid search (keyword + semantic)
- `get_symbol` - Symbol lookup by name
- `get_file_summary` - File structure overview
- `get_dependencies` - Dependency graph traversal
- `find_patterns` - Code pattern detection
- `get_architecture` - Architecture overview
- `explain_codebase` - Comprehensive project explanation
- `get_status` - Engine status and statistics

See `crates/omni-mcp/src/tools.rs` for implementation details.

### CLI

Command-line interface for indexing and searching.

**Commands**:
- `omnicontext index <path>` - Index a repository
- `omnicontext search <query>` - Search indexed code
- `omnicontext status` - Show index statistics
- `omnicontext config` - Manage configuration

See `crates/omni-cli/src/main.rs` for implementation details.

## Contributing

API documentation contributions are welcome! Please follow the documentation standards in the main [docs README](../README.md).
