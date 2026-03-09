import { MarkdownRenderer } from "@/components/markdown-renderer";

export default function IntroductionPage() {
  const content = `# Introduction

OmniContext is a natively-compiled semantic code search engine that provides AI agents with structured codebase context through the Model Context Protocol (MCP). All processing runs locally without external APIs.

## What is OmniContext?

OmniContext indexes your codebase using AST parsing, semantic chunking, and vector embeddings to enable fast, accurate code search. It exposes 16 MCP tools that AI agents can use to understand your code, navigate dependencies, and assemble relevant context.

## Key features

- **Hybrid search**: Combines keyword matching with semantic vector search for accurate retrieval
- **AST-aware parsing**: Understands code structure across 16+ languages using tree-sitter
- **Graph reranking**: Boosts results based on dependency relationships and import patterns
- **Local execution**: All embeddings and indexing run on your machine (no cloud APIs)
- **MCP integration**: Native support for Claude Desktop, Cursor, Windsurf, and other MCP clients
- **Real-time updates**: File watcher detects changes and incrementally updates the index

## How it works

1. **Parse**: Tree-sitter extracts AST structure from source files
2. **Chunk**: Semantic chunking splits code into meaningful units
3. **Embed**: ONNX-based model generates vector embeddings locally
4. **Index**: SQLite + HNSW vector index stores chunks for fast retrieval
5. **Search**: Hybrid engine combines keyword and vector search with graph reranking
6. **Serve**: MCP server exposes tools to AI agents via stdio or SSE transport

## Performance

OmniContext is optimized for speed:

- **Indexing**: > 500 files/sec
- **Embedding**: > 800 chunks/sec on CPU
- **Search**: < 50ms P99 latency (100k chunk index)
- **Memory**: < 2KB per indexed chunk

## Supported languages

Full AST parsing support for:

- JavaScript, TypeScript, JSX, TSX
- Python, Ruby, PHP
- Rust, Go, C, C++, C#
- Java, Kotlin, Swift
- CSS, HTML, Markdown

## Architecture

OmniContext is a Cargo workspace with four crates:

- \`omni-core\`: Core library (indexing, search, embeddings)
- \`omni-cli\`: Command-line interface
- \`omni-daemon\`: Background process with IPC
- \`omni-mcp\`: MCP server for AI agent integration

## Get started

Ready to index your first codebase? Follow the [Quickstart Guide](/docs/quickstart) to get up and running in 5 minutes.`;

  // Extract headings for TOC
  const headings = content.match(/^#{2,3}\s+(.+)$/gm)?.map(heading => {
    const level = heading.match(/^#{2,3}/)?.[0].length || 2;
    const text = heading.replace(/^#{2,3}\s+/, '');
    const id = text.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    return { id, text, level };
  }) || [];

  return (
    <div className="flex-1 flex h-full">
      <div className="flex-1 px-10 md:px-20 py-16 flex justify-center bg-[#09090B] xl:mr-[240px]">
        <article className="max-w-[760px] w-full">
          <div className="text-[12px] font-semibold tracking-wider uppercase text-zinc-600 mb-6">
            Getting Started
          </div>
          <MarkdownRenderer content={content} />
        </article>
      </div>

      {/* Right TOC Sidebar */}
      {headings.length > 0 && (
        <aside
          className="w-[240px] shrink-0 p-10 overflow-y-auto custom-scrollbar border-l border-white/5 hidden xl:block bg-[#09090B] xl:fixed xl:right-0 xl:top-14 xl:bottom-0"
          data-lenis-prevent
        >
          <div className="text-[12px] font-semibold uppercase tracking-wider text-zinc-600 mb-6">
            On this page
          </div>
          <nav className="flex flex-col gap-4 text-[13px] tracking-tight">
            {headings
              .filter((h) => h.level <= 3)
              .map((heading, idx) => (
                <a
                  key={heading.id}
                  href={`#${heading.id}`}
                  className={`hover:text-zinc-300 transition-colors duration-200 ${heading.level === 2
                      ? idx === 0
                        ? "text-zinc-200 font-medium"
                        : "text-zinc-500"
                      : "text-zinc-600 pl-4"
                    }`}
                >
                  {heading.text}
                </a>
              ))}
          </nav>
        </aside>
      )}
    </div>
  );
}
