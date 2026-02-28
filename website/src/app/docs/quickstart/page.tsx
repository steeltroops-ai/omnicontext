"use client";

import React from "react";
import { ChevronRight, Copy, Check } from "lucide-react";
import { siteConfig } from "@/config/site";
import Link from "next/link";

function CodeBlock({
  code,
  id,
  lang = "bash",
  copied,
  onCopy,
}: {
  code: string;
  id: string;
  lang?: string;
  copied: string | null;
  onCopy: (text: string, id: string) => void;
}) {
  return (
    <div className="relative bg-[#0E0E11] border border-white/5 rounded-xl p-5 font-mono text-[13px] text-zinc-300 mb-6 group">
      <div className="absolute top-3 right-3">
        <button
          onClick={() => onCopy(code, id)}
          className="text-zinc-600 hover:text-zinc-300 transition-colors p-1"
        >
          {copied === id ? <Check size={14} /> : <Copy size={14} />}
        </button>
      </div>
      <div className="text-[10px] text-zinc-600 uppercase tracking-widest mb-3 font-sans font-semibold">
        {lang}
      </div>
      <pre className="whitespace-pre-wrap leading-[1.8]">{code}</pre>
    </div>
  );
}

export default function QuickstartPage() {
  const [copied, setCopied] = React.useState<string | null>(null);

  const copyToClipboard = (text: string, id: string) => {
    navigator.clipboard.writeText(text);
    setCopied(id);
    setTimeout(() => setCopied(null), 2000);
  };

  return (
    <div className="flex-1 flex overflow-hidden h-full">
      <div className="flex-1 overflow-y-scroll custom-scrollbar px-10 md:px-20 py-16 flex justify-center bg-[#09090B]">
        <article className="max-w-[760px] w-full">
          <div className="text-[12px] font-semibold tracking-wider uppercase text-zinc-600 mb-6">
            Getting Started
          </div>

          <h1 className="text-4xl md:text-5xl font-semibold text-white tracking-tighter mb-6 leading-tight">
            Quickstart
          </h1>
          <p className="text-[18px] text-zinc-400 leading-[1.6] mb-14 tracking-tight">
            Get {siteConfig.name} running on your machine in under two minutes.
            Index your codebase and start serving context to your AI agents.
          </p>

          <h2 className="text-[22px] text-white mt-12 mb-4 font-semibold tracking-tight">
            1. Install from source
          </h2>
          <p className="text-[15px] text-zinc-400 leading-relaxed mb-4 tracking-tight">
            {siteConfig.name} requires Rust 1.80+ and a C compiler for
            Tree-sitter grammar compilation. Clone the repository and build the
            workspace:
          </p>

          <CodeBlock
            id="install"
            copied={copied}
            onCopy={copyToClipboard}
            code={`git clone ${siteConfig.links.github}.git
cd ${siteConfig.name.toLowerCase()}
cargo build --release`}
          />

          <p className="text-[14px] text-zinc-500 leading-relaxed mb-8 tracking-tight">
            The release build produces three binaries in{" "}
            <code className="bg-white/5 px-1.5 py-0.5 rounded text-zinc-300 text-[13px]">
              target/release/
            </code>
            :{" "}
            <code className="bg-white/5 px-1.5 py-0.5 rounded text-zinc-300 text-[13px]">
              omnicontext
            </code>{" "}
            (CLI),{" "}
            <code className="bg-white/5 px-1.5 py-0.5 rounded text-zinc-300 text-[13px]">
              omnicontext-mcp
            </code>{" "}
            (MCP server), and the core library.
          </p>

          <h2 className="text-[22px] text-white mt-12 mb-4 font-semibold tracking-tight">
            2. Index your codebase
          </h2>
          <p className="text-[15px] text-zinc-400 leading-relaxed mb-4 tracking-tight">
            Point the CLI at any directory. {siteConfig.name} will detect
            languages, parse ASTs, generate embeddings, and build the dependency
            graph automatically.
          </p>

          <CodeBlock
            id="index"
            copied={copied}
            onCopy={copyToClipboard}
            code={`# Index the current directory
omnicontext index .

# Index a specific project
omnicontext index /path/to/your/project

# Check indexing status
omnicontext status`}
          />

          <h2 className="text-[22px] text-white mt-12 mb-4 font-semibold tracking-tight">
            3. Start the MCP server
          </h2>
          <p className="text-[15px] text-zinc-400 leading-relaxed mb-4 tracking-tight">
            Launch the MCP server so any compatible AI agent can connect and
            query your codebase context.
          </p>

          <CodeBlock
            id="mcp"
            copied={copied}
            onCopy={copyToClipboard}
            code={`# Start with stdio transport (for Claude Code, Codex, etc.)
omnicontext-mcp --transport stdio

# Start with SSE transport (for web-based agents)
omnicontext-mcp --transport sse --port 8080`}
          />

          <h2 className="text-[22px] text-white mt-12 mb-4 font-semibold tracking-tight">
            4. Connect your agent
          </h2>
          <p className="text-[15px] text-zinc-400 leading-relaxed mb-4 tracking-tight">
            Configure your AI agent to use {siteConfig.name} as an MCP server.
            For Claude Code, add the following to your MCP configuration:
          </p>

          <CodeBlock
            id="config"
            lang="json"
            copied={copied}
            onCopy={copyToClipboard}
            code={`{
  "mcpServers": {
    "${siteConfig.name.toLowerCase()}": {
      "command": "${siteConfig.name.toLowerCase()}-mcp",
      "args": ["--transport", "stdio", "--repo", "."],
      "env": {}
    }
  }
}`}
          />

          <div className="bg-emerald-500/5 border border-emerald-500/20 rounded-2xl p-6 mb-16 mt-8">
            <div className="text-[13px] text-emerald-400 font-semibold mb-2">
              You&apos;re ready.
            </div>
            <p className="text-[14px] text-zinc-400 leading-relaxed tracking-tight m-0">
              Your agent can now call{" "}
              <code className="text-emerald-400/80 bg-emerald-500/10 px-1.5 py-0.5 rounded text-[12px]">
                search_code
              </code>
              ,{" "}
              <code className="text-emerald-400/80 bg-emerald-500/10 px-1.5 py-0.5 rounded text-[12px]">
                get_dependencies
              </code>
              ,{" "}
              <code className="text-emerald-400/80 bg-emerald-500/10 px-1.5 py-0.5 rounded text-[12px]">
                get_architecture
              </code>
              , and 5 other tools to understand your full codebase. See the{" "}
              <Link
                href="/docs/mcp-tools"
                className="text-emerald-400 underline underline-offset-2"
              >
                MCP Tools reference
              </Link>{" "}
              for the complete API.
            </p>
          </div>

          <div className="flex justify-between mt-12 pb-16">
            <Link
              href="/docs"
              className="inline-flex items-center gap-2 text-[14px] text-zinc-500 hover:text-zinc-200 transition-colors"
            >
              <ChevronRight size={14} className="rotate-180" /> Introduction
            </Link>
            <Link
              href="/docs/installation"
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded-full bg-zinc-100 text-black text-[14px] font-semibold hover:scale-105 active:scale-95 transition-all duration-300"
            >
              Installation Guide <ChevronRight size={16} strokeWidth={2.5} />
            </Link>
          </div>
        </article>
      </div>

      <aside className="w-[240px] shrink-0 p-10 overflow-y-auto custom-scrollbar border-l border-white/5 hidden xl:block bg-[#09090B]">
        <div className="text-[12px] font-semibold uppercase tracking-wider text-zinc-600 mb-6">
          On this page
        </div>
        <nav className="flex flex-col gap-4 text-[13px] tracking-tight">
          <a
            href="#"
            className="text-zinc-200 font-medium hover:text-white transition-colors"
          >
            Install from source
          </a>
          <a
            href="#"
            className="text-zinc-500 hover:text-zinc-300 transition-colors"
          >
            Index your codebase
          </a>
          <a
            href="#"
            className="text-zinc-500 hover:text-zinc-300 transition-colors"
          >
            Start the MCP server
          </a>
          <a
            href="#"
            className="text-zinc-500 hover:text-zinc-300 transition-colors"
          >
            Connect your agent
          </a>
        </nav>
      </aside>
    </div>
  );
}
