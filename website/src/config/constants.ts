/**
 * Website Constants
 * 
 * All hardcoded values that may change over time should be defined here.
 * This includes version numbers, performance metrics, feature lists, etc.
 */

// Version Information
export const VERSION = {
    current: "1.0.1",
    displayFormat: "v1.0.1",
} as const;

// Performance Metrics
export const PERFORMANCE_METRICS = {
    languages: "16+",
    searchLatency: "< 50ms",
    throughput: "800+",
    throughputUnit: "Chunks/sec",
    deployment: "Local-Only",

    // Detailed metrics for search section
    search: {
        p50: "12ms",
        p95: "28ms",
        p99: "41ms",
        throughput: "847 chunks/sec",
        modelLatency: "14ms (CPU)",
    },
} as const;

// Supported AI Clients
export const SUPPORTED_AI_CLIENTS = [
    "Claude Desktop",
    "Cursor",
    "Continue.dev",
    "Kiro",
    "Windsurf",
    "Cline",
    "RooCode",
    "Trae",
    "Antigravity",
] as const;

// MCP Tools Count
export const MCP_TOOLS_COUNT = 16;

// Supported Languages
export const SUPPORTED_LANGUAGES = [
    "Python",
    "TypeScript",
    "JavaScript",
    "Rust",
    "Go",
    "Java",
    "C",
    "C++",
    "C#",
    "CSS",
    "Ruby",
    "PHP",
    "Swift",
    "Kotlin",
    "Markdown",
    "TOML",
] as const;

export const SUPPORTED_LANGUAGES_COUNT = SUPPORTED_LANGUAGES.length;

// Feature Lists
export const FEATURES = {
    zeroConfig: [
        "Tree-sitter AST parsing for 16+ languages",
        "Semantic chunking with token counting",
        "Contextual chunking for better semantic boundaries",
        "ONNX embedding (jina-embeddings-v2-base-code)",
        "Batch embedding with intelligent backpressure",
        "SQLite FTS5 + HNSW vector index",
        "Connection pooling for concurrent access",
    ],

    hybridSearch: [
        "BM25 keyword search",
        "Semantic vector search",
        "Graph-boosted reranking",
        "Query result caching for instant responses",
        "HyDE (Hypothetical Document Embeddings)",
        "Intent classification and synonym expansion",
    ],

    mcpIntegration: [
        `Provides ${MCP_TOOLS_COUNT} powerful MCP tools for code intelligence`,
        `Connects to ${SUPPORTED_AI_CLIENTS.slice(0, 3).join(", ")}, and more`,
        "Stdio and HTTP SSE transports",
        "Zero-config MCP sync: auto-configures all detected AI clients",
    ],

    ideIntegration: [
        "Real-time performance metrics in sidebar",
        "Activity log and cache statistics",
        "One-click environment repair",
        "Pre-fetch caching for instant context",
        "Chat participant for seamless AI integration",
    ],

    enterpriseArchitecture: [
        "Background daemon with IPC",
        "Connection pooling for concurrent access",
        "Health monitoring with circuit breaker",
        "Dependency graph analysis",
        "Git-aware indexing",
    ],
} as const;

// Platform Support
export const PLATFORMS = {
    windows: {
        name: "Windows",
        architectures: ["x86_64"],
    },
    macos: {
        name: "macOS",
        architectures: ["x86_64", "ARM64"],
    },
    linux: {
        name: "Linux",
        architectures: ["x86_64"],
    },
} as const;

// Installation Commands
export const INSTALLATION = {
    curl: "curl -fsSL https://omnicontext.dev/install.sh | sh",
    index: "omnicontext index .",
    search: 'omnicontext search "authentication logic"',
} as const;

// Hero Section Content
export const HERO = {
    title: "The context engine your codebase deserves.",
    subtitle: "OmniContext represents a fundamental shift in AI coding. Universal dependency awareness, written in Rust, and executed flawlessly on your local machine.",
    cta: {
        primary: {
            text: "Install Extension",
            href: "https://marketplace.visualstudio.com/items?itemName=steeltroops.omnicontext",
        },
        secondary: {
            text: "Read Docs",
            href: "/docs",
        },
    },
} as const;

// Section Titles
export const SECTIONS = {
    zeroConfig: {
        badge: "Intelligence",
        title: "Zero-Config Intelligence.",
        description: "One command indexes your entire codebase with semantic understanding. No configuration files, no complex setup—just point and index.",
    },

    contextEngine: {
        badge: "Context Engine",
        title: "Enterprise Context Engine.",
        description: "Production-grade semantic understanding that transforms raw code into structured, queryable intelligence. Built for scale, designed for precision.",
    },

    hybridSearch: {
        badge: "Search Engine",
        title: "Hybrid Search Engine.",
        description: "BM25 keyword search combined with semantic vector search and graph-boosted reranking. Get the best of all worlds.",
    },

    mcpIntegration: {
        badge: "Agent Protocol",
        title: "Native MCP Server Integration.",
        description: "OmniContext does not try to be an AI agent; it empowers the ones you already use. It runs fully locally as a standard Model Context Protocol (MCP) server over `stdio` or `sse`.",
    },

    ideIntegration: {
        badge: "IDE Integration",
        title: "Built for Your IDE.",
        description: "Native VS Code extension with professional UI, real-time metrics, and seamless AI integration.",
    },

    installation: {
        badge: "Installation",
        title: "Get Started in Seconds.",
        description: "One command to production-ready search. No complex setup, no configuration files.",
    },

    enterpriseArchitecture: {
        badge: "Architecture",
        title: "Enterprise-Grade Architecture.",
        description: "Built for scale with background daemon, connection pooling, health monitoring, and dependency graph analysis.",
    },

    crossPlatform: {
        badge: "Cross-Platform",
        title: "Works Everywhere.",
        description: "Native binaries for all major platforms with consistent performance and behavior.",
    },
} as const;

// MCP Configuration Example
export const MCP_CONFIG_EXAMPLE = {
    language: "json",
    filename: "claude_desktop_config.json",
    code: `{
  "mcpServers": {
    "omnicontext": {
      "command": "omnicontext-mcp",
      "args": ["--transport", "stdio", "--repo", "."],
      "env": {}
    }
  }
}`,
} as const;

// Type exports for TypeScript
export type SupportedLanguage = typeof SUPPORTED_LANGUAGES[number];
export type SupportedAIClient = typeof SUPPORTED_AI_CLIENTS[number];
export type Platform = keyof typeof PLATFORMS;

// Context Engine Visualization Data
export const CONTEXT_ENGINE_LEFT_COLUMN = [
    { label: "Code", y: 0 },
    { label: "Dependencies", y: 60 },
    { label: "Documentation", y: 120 },
    { label: "Style", y: 180 },
    { label: "Recent changes", y: 240 },
    { label: "Issues", y: 300 },
] as const;

export const CONTEXT_ENGINE_RIGHT_COLUMN = [
    { label: "Completions", y: 0 },
    { label: "Code Review", y: 80 },
    { label: "Agents", y: 160 },
    { label: "Intent", y: 240 },
] as const;

// SVG positioning constants
export const CONTEXT_ENGINE_SVG = {
    leftColumnTop: 142, // Container top position for left column
    rightColumnTop: 172, // Container top position for right column
    leftPathOffset: 200, // SVG path y-offset for left connections
    rightPathOffset: 230, // SVG path y-offset for right connections
    centerConnectionY: 350, // Y position where all paths meet at center
} as const;

