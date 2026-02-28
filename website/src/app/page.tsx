"use client";

import { motion } from "framer-motion";
import {
  Terminal,
  Network,
  ShieldCheck,
  Github,
  SearchCode,
} from "lucide-react";
import Link from "next/link";

export default function Home() {
  return (
    <main className="flex flex-col min-h-screen">
      {/* Navigation */}
      <nav className="fixed top-0 w-full h-16 flex items-center justify-between px-8 z-50 border-b border-border bg-background/70 backdrop-blur-md">
        <div className="flex items-center gap-2 font-bold text-xl text-foreground">
          <SearchCode className="text-primary" size={28} strokeWidth={2.5} />
          <span>OmniContext</span>
        </div>

        <div className="hidden md:flex gap-8">
          <Link
            href="/docs"
            className="text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
          >
            Docs
          </Link>
          <Link
            href="/blog"
            className="text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
          >
            Blog
          </Link>
          <Link
            href="/enterprise"
            className="text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
          >
            Enterprise
          </Link>
        </div>

        <div className="flex items-center gap-4">
          <button className="px-4 py-2 text-sm font-medium border border-border rounded-md text-foreground hover:bg-border/50 hover:border-muted-foreground transition-all duration-200">
            Sign In
          </button>
          <button className="px-4 py-2 text-sm font-semibold rounded-md bg-primary text-primary-foreground shadow-[0_0_15px_rgba(34,197,94,0.15)] hover:bg-primary/90 transition-colors">
            Get Started
          </button>
        </div>
      </nav>

      {/* Hero Section */}
      <section className="flex-1 flex flex-col items-center justify-center text-center px-4 pt-32 pb-16 relative">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className="bg-primary/10 text-primary border border-primary/30 px-3 py-1 rounded-full text-sm font-semibold mb-8"
        >
          v0.1.0 Available Now — Blazing Fast Local Search
        </motion.div>

        <motion.h1
          className="text-5xl md:text-7xl font-bold max-w-4xl mx-auto tracking-tight bg-gradient-to-b from-foreground to-muted-foreground bg-clip-text text-transparent mb-6 lg:leading-tight"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
        >
          Give your codebase the <br className="hidden md:block" /> context
          engine it deserves
        </motion.h1>

        <motion.p
          className="text-lg md:text-xl text-muted-foreground max-w-2xl mx-auto mb-12 leading-relaxed"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
        >
          A universal, dependency-aware search engine built for AI coding
          agents. Written in Rust. Local first. MCP Native.
        </motion.p>

        <motion.div
          className="flex gap-4 justify-center"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.3 }}
        >
          <button className="px-6 py-3 text-base font-semibold rounded-md bg-primary text-primary-foreground shadow-[0_0_15px_rgba(34,197,94,0.15)] hover:bg-primary/90 transition-colors">
            Install CLI
          </button>
          <button className="px-6 py-3 text-base font-medium border border-border rounded-md text-foreground hover:bg-border/50 hover:border-muted-foreground transition-all duration-200">
            Read the Docs
          </button>
        </motion.div>

        {/* Hero Visual (Terminal Mockup) */}
        <motion.div
          className="mt-16 w-full max-w-4xl h-[400px] md:h-[500px] relative flex items-center justify-center overflow-hidden border border-border/50 rounded-xl bg-black/60 backdrop-blur-md shadow-2xl p-6"
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.7, delay: 0.4 }}
        >
          <div className="w-full h-full bg-[#0a0a0c] rounded-lg p-6 shadow-inner flex flex-col relative overflow-hidden text-left border border-border/30">
            <div className="flex gap-2 mb-6">
              <div className="w-3 h-3 rounded-full bg-red-500"></div>
              <div className="w-3 h-3 rounded-full bg-yellow-500"></div>
              <div className="w-3 h-3 rounded-full bg-green-500"></div>
            </div>
            <pre className="font-mono text-sm text-zinc-400 m-0 whitespace-pre-wrap leading-relaxed">
              $ <span className="text-red-400">omnicontext</span> index
              ./workspace{"\n"}
              <span className="text-zinc-500">[14:02:12]</span> Building
              semantic chunks...{"\n"}
              <span className="text-zinc-500">[14:02:13]</span> Generating
              embeddings (ONNX local)...{"\n"}
              <span className="text-zinc-500">[14:02:14]</span> Computing
              dependency graph...{"\n"}
              <span className="text-primary font-bold">
                ✓ Successfully indexed 42,104 files in 2.1s
              </span>
              {"\n"}
              {"\n"}$ <span className="text-red-400">omnicontext-mcp</span>{" "}
              --repo ./workspace{"\n"}
              <span className="text-primary font-bold">
                ► OmniContext MCP Server listening on stdio...
              </span>
              {"\n"}
              <span className="text-zinc-500">
                {" "}
                Ready to serve advanced code intelligence to your agent.
              </span>
            </pre>
          </div>
        </motion.div>
      </section>

      {/* Features Grid */}
      <section className="py-24 px-4 w-full max-w-7xl mx-auto">
        <h2 className="text-4xl font-bold text-center mb-16 text-foreground">
          Built differently. Built better.
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          <motion.div className="flex flex-col items-start gap-6 p-10 bg-black/40 backdrop-blur-md border border-border rounded-xl transition-all duration-300 hover:-translate-y-2 hover:border-border/80">
            <div className="p-4 rounded-xl bg-primary/10 text-primary">
              <Terminal size={32} />
            </div>
            <h3 className="text-xl font-bold text-foreground">
              Blazing Fast Rust Core
            </h3>
            <p className="text-muted-foreground leading-relaxed">
              Forget slow TypeScript parsers. OmniContext uses Tree-sitter and
              an optimized SQLite+Vector pipeline to index millions of lines of
              code in seconds, locally.
            </p>
          </motion.div>

          <motion.div className="flex flex-col items-start gap-6 p-10 bg-black/40 backdrop-blur-md border border-border rounded-xl transition-all duration-300 hover:-translate-y-2 hover:border-border/80">
            <div className="p-4 rounded-xl bg-primary/10 text-primary">
              <Network size={32} />
            </div>
            <h3 className="text-xl font-bold text-foreground">
              Dependency Graph Fusion
            </h3>
            <p className="text-muted-foreground leading-relaxed">
              We don&apos;t just do semantic search. We build a full dependency
              graph (imports, extends, calls) and fuse signals via Reciprocal
              Rank Fusion (RRF) for precise results.
            </p>
          </motion.div>

          <motion.div className="flex flex-col items-start gap-6 p-10 bg-black/40 backdrop-blur-md border border-border rounded-xl transition-all duration-300 hover:-translate-y-2 hover:border-border/80">
            <div className="p-4 rounded-xl bg-primary/10 text-primary">
              <ShieldCheck size={32} />
            </div>
            <h3 className="text-xl font-bold text-foreground">
              Enterprise Grade Context
            </h3>
            <p className="text-muted-foreground leading-relaxed">
              API keys, rate-limiting, usage metering, commit lineage, and
              pattern recognition engines. Perfect for deploying internal custom
              agent fleets securely.
            </p>
          </motion.div>
        </div>
      </section>

      {/* Footer */}
      <footer className="border-t border-border py-12 px-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-4">
        <p>© 2026 OmniContext by Mayank. Built with Next.js and Bun.</p>
        <div className="flex justify-center gap-4">
          <a
            href="https://github.com/steeltroops-ai/omnicontext"
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            <Github size={20} />
          </a>
        </div>
      </footer>
    </main>
  );
}
