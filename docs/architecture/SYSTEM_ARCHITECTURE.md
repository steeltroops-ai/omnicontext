# OmniContext System Architecture

> **Version**: 1.0  
> **Last Updated**: 2026-03-01  
> **Status**: Living Document

## Executive Summary

OmniContext is a high-performance, local-first code intelligence engine that provides AI agents with deep semantic understanding of codebases. Built in Rust, it combines AST parsing, semantic embeddings, and graph analysis to deliver sub-50ms search latency with zero cloud dependencies.

## System Overview

```mermaid
graph TB
    subgraph "Client Layer"
        MCP[MCP Client<br/>Claude/Cursor/Copilot]
        CLI[CLI Interface<br/>omnicontext]
        VSCode[VS Code Extension]
    end
    
    subgraph "API Layer"
        MCPS[MCP Server<br/>stdio/SSE]
        REST[REST API<br/>Enterprise]
    end
    
    subgraph "Core Engine"
        Parser[Parser<br/>tree-sitter]
        Chunker[Chunker<br/>Semantic]
        Embedder[Embedder<br/>ONNX]
        Search[Search Engine<br/>Hybrid]
        Graph[Dependency Graph<br/>petgraph]
    end
    
    subgraph "Storage Layer"
        SQLite[(SQLite<br/>Metadata + FTS5)]
        Vector[(usearch<br/>Vector Index)]
        Files[(File System<br/>Source Code)]
    end
    
    subgraph "Background Services"
        Watcher[File Watcher<br/>notify]
        Daemon[Daemon<br/>Incremental Updates]
    end
    
    MCP --> MCPS
    CLI --> Search
    VSCode --> MCPS
    REST --> Search
    
    MCPS --> Search
    Search --> Parser
    Search --> Graph
    Parser --> Chunker
    Chunker --> Embedder
    Embedder --> Vector
    Parser --> SQLite
    Chunker --> SQLite
    Graph --> SQLite
    
    Watcher --> Files
    Watcher --> Daemon
    Daemon --> Parser
    
    style MCP fill:#e1f5ff
    style CLI fill:#e1f5ff
    style VSCode fill:#e1f5ff
    style Search fill:#fff4e1
    style SQLite fill:#f0f0f0
    st