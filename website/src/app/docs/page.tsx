"use client";

import React from "react";
import { Cpu, ChevronRight, Search, GitBranch, Layers } from "lucide-react";
import Link from "next/link";

export default function DocsPage() {
  return (
    <div className="flex-1 flex overflow-hidden h-full">
      {/* Article Content */}
      <div className="flex-1 overflow-y-auto px-10 md:px-20 py-16 flex justify-center bg-[#09090B]">
        <article className="max-w-[760px] w-full">
          <div className="text-[12px] font-semibold tracking-wider uppercase text-zinc-600 mb-6">
            Getting Started
          </div>

          <h1 className="text-4xl md:text-5xl font-semibold text-white tracking-tighter mb-6 leading-tight">
            Introduction
          </h1>
          <p className="text-[18px] text-zinc-400 leading-[1.6] mb-6 tracking-tight">
            OmniContext is a high-performance, locally-runnable code context
            engine that gives AI coding agents deep understanding of any
            codebase. It is{" "}
            <span className="text-zinc-200 font-medium">not</span> an AI
            assistant. It is the intelligence infrastructure that makes AI
            assistants intelligent.
          </p>
          <p className="text-[16px] text-zinc-500 leading-[1.6] mb-14 tracking-tight">
            Every AI coding agent today (Claude Code, Cursor, Copilot, Windsurf,
            Codex) struggles with the same problem -- they do not understand
            your codebase. They see files as text, not as an interconnected
            system. OmniContext solves this by building a live, semantic index
            of the entire codebase and exposing it via the Model Context
            Protocol (MCP).
          </p>

          {/* Terminal Mockup - Actual CLI Output */}
          <div className="w-full bg-[#0E0E11] border border-white/5 rounded-2xl mb-16 overflow-hidden shadow-[0_20px_80px_rgba(0,0,0,0.8)]">
            {/* Window Chrome */}
            <div className="bg-[#141418] px-5 py-3 flex items-center gap-2 border-b border-white/5">
              <div className="flex gap-2">
                <div className="w-2.5 h-2.5 rounded-full bg-[#555]"></div>
                <div className="w-2.5 h-2.5 rounded-full bg-[#555]"></div>
                <div className="w-2.5 h-2.5 rounded-full bg-[#555]"></div>
              </div>
              <span className="ml-4 text-[12px] text-zinc-500 font-mono tracking-wide">
                omnicontext -- local
              </span>
            </div>
            {/* Terminal Content */}
            <div className="p-6 font-mono text-[13px] leading-[1.9] text-zinc-400">
              <div>
                <span className="text-zinc-600">$ </span>
                <span className="text-zinc-100">omnicontext</span> index .
              </div>
              <div className="text-zinc-600">[info] Scanning workspace...</div>
              <div className="text-zinc-600">
                [info] Parsing AST via Tree-sitter (Rust, TypeScript, Python)
              </div>
              <div className="text-zinc-600">
                [info] Generating embeddings (ONNX local, all-MiniLM-L6-v2)
              </div>
              <div className="text-zinc-600">
                [info] Building dependency graph (petgraph)
              </div>
              <div className="text-emerald-400 font-medium mt-2">
                Done. Indexed 42,104 symbols in 2.1s
              </div>
              <div className="mt-4">
                <span className="text-zinc-600">$ </span>
                <span className="text-zinc-100">omnicontext-mcp</span>{" "}
                --transport stdio
              </div>
              <div className="text-emerald-400 font-medium">
                MCP Server listening on stdio...
              </div>
              <div className="text-zinc-600">
                Ready. Connect any MCP-compatible agent.
              </div>
            </div>
          </div>

          <h2
            id="how-it-works"
            className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight"
          >
            How it works
          </h2>
          <p className="text-[16px] text-zinc-400 leading-relaxed mb-8 tracking-tight">
            OmniContext runs as a local process on your machine. It indexes your
            codebase using Tree-sitter for AST parsing, generates embeddings via
            a local ONNX model, and stores everything in an embedded SQLite
            database with HNSW vector search. AI agents connect to it via MCP to
            get rich, ranked context.
          </p>

          {/* Architecture Flow */}
          <div className="bg-[#0E0E11] border border-white/5 rounded-2xl p-8 mb-16">
            <div className="text-[11px] uppercase tracking-widest text-zinc-600 font-semibold mb-6">
              Data Flow
            </div>
            <div className="flex flex-col gap-4 text-[13px] font-mono">
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(34,197,94,0.5)]" />
                <span className="text-zinc-300">
                  Source Files (Rust, TypeScript, Python, Go, +12 more)
                </span>
              </div>
              <div className="border-l border-white/10 ml-1 h-4" />
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-emerald-500/60" />
                <span className="text-zinc-400">
                  Tree-sitter AST Parser (incremental, error-tolerant)
                </span>
              </div>
              <div className="border-l border-white/10 ml-1 h-4" />
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-emerald-500/60" />
                <span className="text-zinc-400">
                  Semantic Chunker (AST-aware, never splits mid-expression)
                </span>
              </div>
              <div className="border-l border-white/10 ml-1 h-4" />
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-emerald-500/60" />
                <span className="text-zinc-400">
                  Embedding Engine (ONNX Runtime, all-MiniLM-L6-v2, 384d)
                </span>
              </div>
              <div className="border-l border-white/10 ml-1 h-4" />
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.5)]" />
                <span className="text-zinc-300">
                  SQLite + FTS5 + usearch HNSW Vector Index
                </span>
              </div>
              <div className="border-l border-white/10 ml-1 h-4" />
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-violet-500 shadow-[0_0_8px_rgba(139,92,246,0.5)]" />
                <span className="text-zinc-300">
                  MCP Server (stdio / SSE) -- serves any agent
                </span>
              </div>
            </div>
          </div>

          <h2
            id="key-capabilities"
            className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight"
          >
            Key capabilities
          </h2>
          <p className="text-[16px] text-zinc-400 leading-relaxed mb-8 tracking-tight">
            OmniContext exposes 8 MCP tools that any compatible agent can call.
            These tools provide semantic search, symbol lookup, dependency
            traversal, architecture summaries, and more.
          </p>

          {/* Capabilities Grid */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-5 mb-16">
            <div className="bg-[#0E0E11] border border-white/5 p-6 rounded-[20px] transition-all duration-300 hover:bg-[#141418] hover:border-white/10 group cursor-pointer flex flex-col">
              <div className="w-9 h-9 rounded-full bg-white/5 flex items-center justify-center mb-4 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <Search size={16} strokeWidth={1.5} />
              </div>
              <div className="text-[15px] font-semibold text-zinc-100 mb-2 tracking-tight group-hover:text-white transition-colors">
                search_code
              </div>
              <p className="text-[13px] text-zinc-500 leading-[1.6] m-0 flex-1 tracking-tight">
                Hybrid semantic + keyword search with Reciprocal Rank Fusion.
                Filter by language, kind, or directory.
              </p>
            </div>
            <div className="bg-[#0E0E11] border border-white/5 p-6 rounded-[20px] transition-all duration-300 hover:bg-[#141418] hover:border-white/10 group cursor-pointer flex flex-col">
              <div className="w-9 h-9 rounded-full bg-white/5 flex items-center justify-center mb-4 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <GitBranch size={16} strokeWidth={1.5} />
              </div>
              <div className="text-[15px] font-semibold text-zinc-100 mb-2 tracking-tight group-hover:text-white transition-colors">
                get_dependencies
              </div>
              <p className="text-[13px] text-zinc-500 leading-[1.6] m-0 flex-1 tracking-tight">
                Traverse the dependency graph upstream or downstream. Discover
                callers, implementors, and type usage chains.
              </p>
            </div>
            <div className="bg-[#0E0E11] border border-white/5 p-6 rounded-[20px] transition-all duration-300 hover:bg-[#141418] hover:border-white/10 group cursor-pointer flex flex-col">
              <div className="w-9 h-9 rounded-full bg-white/5 flex items-center justify-center mb-4 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <Layers size={16} strokeWidth={1.5} />
              </div>
              <div className="text-[15px] font-semibold text-zinc-100 mb-2 tracking-tight group-hover:text-white transition-colors">
                get_architecture
              </div>
              <p className="text-[13px] text-zinc-500 leading-[1.6] m-0 flex-1 tracking-tight">
                Get high-level architecture: modules, relationships, entry
                points. Scope to full repo, module, or directory.
              </p>
            </div>
            <div className="bg-[#0E0E11] border border-white/5 p-6 rounded-[20px] transition-all duration-300 hover:bg-[#141418] hover:border-white/10 group cursor-pointer flex flex-col">
              <div className="w-9 h-9 rounded-full bg-white/5 flex items-center justify-center mb-4 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <Cpu size={16} strokeWidth={1.5} />
              </div>
              <div className="text-[15px] font-semibold text-zinc-100 mb-2 tracking-tight group-hover:text-white transition-colors">
                find_patterns
              </div>
              <p className="text-[13px] text-zinc-500 leading-[1.6] m-0 flex-1 tracking-tight">
                Find code patterns and conventions: error handling,
                authentication, logging, data validation.
              </p>
            </div>
          </div>

          <h2
            id="technology"
            className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight"
          >
            Technology stack
          </h2>
          <p className="text-[16px] text-zinc-400 leading-relaxed mb-8 tracking-tight">
            Written entirely in Rust for maximum performance and minimal
            resource usage. Ships as a single binary with zero runtime
            dependencies.
          </p>

          {/* Tech Stack Table */}
          <div className="bg-[#0E0E11] border border-white/5 rounded-2xl overflow-hidden mb-16">
            <table className="w-full text-[13px]">
              <thead>
                <tr className="border-b border-white/5">
                  <th className="text-left p-4 text-zinc-500 font-semibold tracking-tight">
                    Component
                  </th>
                  <th className="text-left p-4 text-zinc-500 font-semibold tracking-tight">
                    Technology
                  </th>
                  <th className="text-left p-4 text-zinc-500 font-semibold tracking-tight">
                    Purpose
                  </th>
                </tr>
              </thead>
              <tbody className="text-zinc-400">
                <tr className="border-b border-white/5">
                  <td className="p-4 text-zinc-200">Runtime</td>
                  <td className="p-4 font-mono text-emerald-400/80">tokio</td>
                  <td className="p-4">Async event loop, concurrent IO</td>
                </tr>
                <tr className="border-b border-white/5">
                  <td className="p-4 text-zinc-200">AST Parsing</td>
                  <td className="p-4 font-mono text-emerald-400/80">
                    tree-sitter
                  </td>
                  <td className="p-4">
                    Incremental, error-tolerant code parsing
                  </td>
                </tr>
                <tr className="border-b border-white/5">
                  <td className="p-4 text-zinc-200">Embedding</td>
                  <td className="p-4 font-mono text-emerald-400/80">
                    ort (ONNX)
                  </td>
                  <td className="p-4">Local model inference, no API needed</td>
                </tr>
                <tr className="border-b border-white/5">
                  <td className="p-4 text-zinc-200">Database</td>
                  <td className="p-4 font-mono text-emerald-400/80">
                    rusqlite
                  </td>
                  <td className="p-4">
                    Embedded SQLite with FTS5 full-text search
                  </td>
                </tr>
                <tr className="border-b border-white/5">
                  <td className="p-4 text-zinc-200">Vector Index</td>
                  <td className="p-4 font-mono text-emerald-400/80">usearch</td>
                  <td className="p-4">HNSW approximate nearest neighbor</td>
                </tr>
                <tr className="border-b border-white/5">
                  <td className="p-4 text-zinc-200">Graph</td>
                  <td className="p-4 font-mono text-emerald-400/80">
                    petgraph
                  </td>
                  <td className="p-4">Dependency graph operations</td>
                </tr>
                <tr>
                  <td className="p-4 text-zinc-200">MCP</td>
                  <td className="p-4 font-mono text-emerald-400/80">rmcp</td>
                  <td className="p-4">
                    Model Context Protocol server implementation
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <h2
            id="pricing"
            className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight"
          >
            Open-core model
          </h2>
          <p className="text-[16px] text-zinc-400 leading-relaxed mb-8 tracking-tight">
            OmniContext follows an open-core business model. The core engine is
            free and open-source. Advanced features are available on paid tiers.
          </p>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-5 mb-16">
            <div className="bg-[#0E0E11] border border-white/5 p-6 rounded-[20px] flex flex-col">
              <div className="text-[13px] text-emerald-500 font-semibold mb-2 uppercase tracking-widest">
                Free
              </div>
              <div className="text-[24px] font-semibold text-white mb-1 tracking-tight">
                $0
              </div>
              <div className="text-[12px] text-zinc-600 mb-4">forever</div>
              <ul className="flex flex-col gap-2 text-[13px] text-zinc-400">
                <li className="flex items-center gap-2">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />
                  Local MCP server
                </li>
                <li className="flex items-center gap-2">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />
                  Unlimited repos
                </li>
                <li className="flex items-center gap-2">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />
                  Full indexing
                </li>
                <li className="flex items-center gap-2">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />
                  Works offline
                </li>
              </ul>
            </div>
            <div className="bg-[#0E0E11] border border-emerald-500/20 p-6 rounded-[20px] flex flex-col relative">
              <div className="absolute -top-2.5 right-4 bg-emerald-500 text-black px-2 py-0.5 rounded text-[10px] font-bold">
                POPULAR
              </div>
              <div className="text-[13px] text-emerald-500 font-semibold mb-2 uppercase tracking-widest">
                Pro
              </div>
              <div className="text-[24px] font-semibold text-white mb-1 tracking-tight">
                $20
                <span className="text-[14px] text-zinc-500 font-normal">
                  /mo
                </span>
              </div>
              <div className="text-[12px] text-zinc-600 mb-4">
                per developer
              </div>
              <ul className="flex flex-col gap-2 text-[13px] text-zinc-400">
                <li className="flex items-center gap-2">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />
                  Multi-repo workspaces
                </li>
                <li className="flex items-center gap-2">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />
                  Commit lineage indexing
                </li>
                <li className="flex items-center gap-2">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />
                  Priority support
                </li>
              </ul>
            </div>
            <div className="bg-[#0E0E11] border border-white/5 p-6 rounded-[20px] flex flex-col">
              <div className="text-[13px] text-zinc-500 font-semibold mb-2 uppercase tracking-widest">
                Enterprise
              </div>
              <div className="text-[24px] font-semibold text-white mb-1 tracking-tight">
                Custom
              </div>
              <div className="text-[12px] text-zinc-600 mb-4">usage-based</div>
              <ul className="flex flex-col gap-2 text-[13px] text-zinc-400">
                <li className="flex items-center gap-2">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />
                  Hosted REST API
                </li>
                <li className="flex items-center gap-2">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />
                  Team sharing &amp; SSO
                </li>
                <li className="flex items-center gap-2">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />
                  Audit logs &amp; SLA
                </li>
              </ul>
            </div>
          </div>

          <div className="flex justify-end mt-12 pb-16">
            <Link
              href="/docs/quickstart"
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded-full bg-zinc-100 text-black text-[14px] font-semibold text-center hover:scale-105 active:scale-95 transition-all duration-300"
            >
              Go to Quickstart <ChevronRight size={16} strokeWidth={2.5} />
            </Link>
          </div>
        </article>
      </div>

      {/* Right TOC Sidebar */}
      <aside className="w-[240px] shrink-0 p-10 overflow-y-auto border-l border-white/5 hidden xl:block bg-[#09090B]">
        <div className="text-[12px] font-semibold uppercase tracking-wider text-zinc-600 mb-6">
          On this page
        </div>
        <nav className="flex flex-col gap-4 text-[13px] tracking-tight">
          <a
            href="#how-it-works"
            className="text-zinc-200 font-medium hover:text-white transition-colors duration-200"
          >
            How it works
          </a>
          <a
            href="#key-capabilities"
            className="text-zinc-500 hover:text-zinc-300 transition-colors duration-200"
          >
            Key capabilities
          </a>
          <a
            href="#technology"
            className="text-zinc-500 hover:text-zinc-300 transition-colors duration-200"
          >
            Technology stack
          </a>
          <a
            href="#pricing"
            className="text-zinc-500 hover:text-zinc-300 transition-colors duration-200"
          >
            Open-core model
          </a>
        </nav>
      </aside>
    </div>
  );
}
