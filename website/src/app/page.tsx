"use client";

import { motion } from "framer-motion";
import { Github, ChevronRight, ArrowUpRight } from "lucide-react";
import Link from "next/link";
import { siteConfig } from "@/config/site";
import { Logo } from "@/components/icons";

export default function Home() {
  return (
    <div className="flex flex-col h-screen overflow-hidden bg-[#09090B] selection:bg-primary/30">
      {/* Navigation - Apple Style Ultra-Minimal */}
      <nav className="shrink-0 w-full h-14 flex items-center justify-center z-50 border-b border-white/5 bg-[#09090B]/50 backdrop-blur-xl">
        <div className="flex items-center justify-between w-full max-w-[1200px] px-8 md:px-16">
          <Link
            href="/"
            className="flex items-center gap-2.5 font-semibold text-[15px] text-zinc-100 transition-opacity hover:opacity-80"
          >
            <Logo className="text-primary" size={22} />
            <span>{siteConfig.name}</span>
          </Link>

          <div className="hidden md:flex items-center gap-8">
            <Link
              href="/docs"
              className="text-[13px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors"
            >
              Docs
            </Link>
            <Link
              href="/blog"
              className="text-[13px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors"
            >
              Blog
            </Link>
            <Link
              href="/enterprise"
              className="text-[13px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors"
            >
              Enterprise
            </Link>
            <Link
              href="/support"
              className="text-[13px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors"
            >
              Support
            </Link>
          </div>

          <div className="flex items-center gap-4">
            <a
              href={siteConfig.links.github}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[13px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors hidden sm:flex items-center gap-1"
            >
              GitHub <ArrowUpRight size={12} />
            </a>
            <Link
              href="/docs/quickstart"
              className="px-3 py-1.5 text-[13px] font-medium rounded-full bg-zinc-100 text-black hover:bg-white transition-colors"
            >
              Get Started
            </Link>
          </div>
        </div>
      </nav>

      <main className="flex-1 overflow-y-auto custom-scrollbar flex flex-col pt-0">
        <section className="flex-1 flex flex-col items-center justify-center text-center px-6 pt-[180px] pb-32 relative overflow-hidden">
          {/* Subtle background glow */}
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-[60%] w-[600px] h-[500px] bg-primary/10 blur-[130px] rounded-full pointer-events-none" />

          <motion.div
            initial={{ opacity: 0, y: 15 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, ease: [0.16, 1, 0.3, 1] }}
            className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-900/50 border border-white/5 text-[13px] font-medium text-zinc-300 mb-8 backdrop-blur-md cursor-pointer hover:bg-zinc-800/50 transition-colors"
          >
            <div className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse" />
            <span>v0.1.0 is now available</span>
            <ChevronRight size={14} className="text-zinc-500" />
          </motion.div>

          <motion.h1
            className="text-5xl md:text-7xl lg:text-[84px] font-semibold max-w-[900px] mx-auto tracking-tighter text-transparent bg-clip-text bg-gradient-to-b from-white to-white/70 mb-8 leading-[1.05]"
            initial={{ opacity: 0, scale: 0.96, filter: "blur(10px)" }}
            animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
            transition={{ duration: 1.2, ease: [0.16, 1, 0.3, 1], delay: 0.1 }}
          >
            The context engine <br className="hidden md:block" /> your codebase
            deserves.
          </motion.h1>

          <motion.p
            className="text-[20px] md:text-[22px] text-zinc-400 max-w-2xl mx-auto mb-12 leading-snug tracking-tight"
            initial={{ opacity: 0, y: 15 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, ease: [0.16, 1, 0.3, 1], delay: 0.2 }}
          >
            OmniContext represents a fundamental shift in AI coding. Universal
            dependency awareness, written in Rust, and executed flawlessly on
            your local machine.
          </motion.p>

          <motion.div
            className="flex flex-col sm:flex-row gap-4 justify-center items-center w-full sm:w-auto"
            initial={{ opacity: 0, y: 15 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8, ease: [0.16, 1, 0.3, 1], delay: 0.3 }}
          >
            <button className="w-full sm:w-auto px-6 py-3 text-[15px] font-medium rounded-full bg-zinc-100 text-black hover:scale-105 active:scale-95 transition-all duration-300 shadow-[0_0_40px_rgba(255,255,255,0.1)]">
              Install OmniContext CLI
            </button>
            <Link
              href="/docs"
              className="w-full sm:w-auto px-6 py-3 text-[15px] font-medium rounded-full bg-zinc-900 text-white border border-white/10 hover:bg-zinc-800 transition-colors duration-300"
            >
              Read Documentation
            </Link>
          </motion.div>

          {/* Hero Visual (The Context Web) */}
          <motion.div
            className="mt-20 w-full max-w-[1000px] relative z-10 mx-auto h-[400px] flex items-center justify-center pointer-events-none"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 1.5, ease: [0.16, 1, 0.3, 1], delay: 0.4 }}
          >
            {/* Animated Web Elements */}
            <div className="relative w-[300px] h-[300px] md:w-[400px] md:h-[400px]">
              {/* Center Node: The Context Engine */}
              <motion.div
                className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-20 h-20 rounded-full bg-[#0E0E11] border border-primary shadow-[0_0_50px_rgba(34,197,94,0.3)] flex items-center justify-center z-30"
                animate={{
                  boxShadow: [
                    "0 0 30px rgba(34,197,94,0.2)",
                    "0 0 60px rgba(34,197,94,0.5)",
                    "0 0 30px rgba(34,197,94,0.2)",
                  ],
                }}
                transition={{
                  duration: 4,
                  repeat: Infinity,
                  ease: "easeInOut",
                }}
              >
                <Logo className="text-primary" size={36} />
              </motion.div>

              {/* Orbital Rings representing the Web */}
              <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[240px] h-[240px] rounded-full border border-white/5" />
              <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[340px] h-[340px] rounded-full border border-dashed border-white/10 animate-[spin_40s_linear_infinite]" />
              <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[460px] h-[460px] rounded-full border border-white/5 opacity-50" />

              {/* Connected Nodes forming the Web */}
              <motion.div
                className="absolute left-[-20%] top-[20%] w-[200px] bg-[#0E0E11] border border-white/10 rounded-lg p-3 flex flex-col gap-2 z-20 shadow-xl"
                initial={{ x: -20, opacity: 0 }}
                animate={{ x: 0, opacity: 1 }}
                transition={{ delay: 0.6 }}
              >
                <div className="text-[10px] uppercase text-emerald-500 font-semibold tracking-wider">
                  Semantic Vector Index
                </div>
                <div className="text-[12px] font-mono text-zinc-400">
                  usearch HNSW (Dense)
                </div>
                <div className="text-[10px] text-zinc-500">
                  all-MiniLM-L6-v2
                </div>
              </motion.div>

              <motion.div
                className="absolute right-[-20%] top-[10%] w-[180px] bg-[#0E0E11] border border-white/10 rounded-lg p-3 flex flex-col gap-2 z-20 shadow-xl"
                initial={{ x: 20, opacity: 0 }}
                animate={{ x: 0, opacity: 1 }}
                transition={{ delay: 0.8 }}
              >
                <div className="text-[10px] uppercase text-indigo-400 font-semibold tracking-wider">
                  Keyword Search
                </div>
                <div className="text-[12px] font-mono text-zinc-400">
                  SQLite FTS5 (Sparse)
                </div>
                <div className="text-[10px] text-zinc-500">BM25 + TF-IDF</div>
              </motion.div>

              <motion.div
                className="absolute left-[5%] bottom-[-10%] w-[220px] bg-[#0E0E11] border border-white/10 rounded-lg p-3 flex flex-col gap-2 z-20 shadow-xl"
                initial={{ y: 20, opacity: 0 }}
                animate={{ y: 0, opacity: 1 }}
                transition={{ delay: 1.0 }}
              >
                <div className="text-[10px] uppercase text-yellow-500 font-semibold tracking-wider">
                  Dependency Graph
                </div>
                <div className="text-[12px] font-mono text-zinc-400">
                  petgraph RAM resident
                </div>
                <div className="text-[10px] text-zinc-500">
                  Tree-sitter AST extraction
                </div>
              </motion.div>

              <motion.div
                className="absolute right-[0%] bottom-[-5%] w-[190px] bg-[#0E0E11] border border-white/10 rounded-lg p-3 flex flex-col gap-2 z-20 shadow-xl"
                initial={{ y: 20, opacity: 0 }}
                animate={{ y: 0, opacity: 1 }}
                transition={{ delay: 1.2 }}
              >
                <div className="text-[10px] uppercase text-rose-400 font-semibold tracking-wider">
                  Fusion Layer
                </div>
                <div className="text-[12px] font-mono text-zinc-400">
                  Reciprocal Rank Fusion
                </div>
                <div className="text-[10px] text-zinc-500">
                  RRF Scoring + Boosting
                </div>
              </motion.div>

              {/* Visual connecting lines drawn using CSS gradients for the web effect */}
              <svg
                className="absolute inset-0 w-full h-full -z-10 overflow-visible"
                xmlns="http://www.w3.org/2000/svg"
              >
                <motion.path
                  d="M 200 200 L 0 80"
                  stroke="rgba(255,255,255,0.15)"
                  strokeWidth="1"
                  strokeDasharray="4 4"
                  initial={{ pathLength: 0 }}
                  animate={{ pathLength: 1 }}
                  transition={{ duration: 1, delay: 0.7 }}
                />
                <motion.path
                  d="M 200 200 L 400 40"
                  stroke="rgba(255,255,255,0.15)"
                  strokeWidth="1"
                  strokeDasharray="4 4"
                  initial={{ pathLength: 0 }}
                  animate={{ pathLength: 1 }}
                  transition={{ duration: 1, delay: 0.9 }}
                />
                <motion.path
                  d="M 200 200 L 80 400"
                  stroke="rgba(255,255,255,0.15)"
                  strokeWidth="1"
                  strokeDasharray="4 4"
                  initial={{ pathLength: 0 }}
                  animate={{ pathLength: 1 }}
                  transition={{ duration: 1, delay: 1.1 }}
                />
                <motion.path
                  d="M 200 200 L 380 380"
                  stroke="rgba(255,255,255,0.15)"
                  strokeWidth="1"
                  strokeDasharray="4 4"
                  initial={{ pathLength: 0 }}
                  animate={{ pathLength: 1 }}
                  transition={{ duration: 1, delay: 1.3 }}
                />
              </svg>
            </div>
          </motion.div>
        </section>

        {/* Massive Graph Architecture Banner */}
        <section className="py-[160px] px-8 md:px-16 w-full max-w-[1400px] mx-auto border-t border-white/5 relative flex flex-col items-center">
          <h2 className="text-4xl md:text-[52px] font-semibold tracking-tighter text-white mb-6 text-center">
            The Context Engine Platform
          </h2>
          <p className="text-[18px] text-zinc-400 max-w-[600px] text-center tracking-tight leading-snug mb-16">
            Build software with AI agents that understand your entire codebase.
            From IDE to CLI to autonomous code review, OmniContext works
            locally.
          </p>

          {/* Graph Mockup Container */}
          <div className="w-full relative bg-[#0E0E11] border border-white/10 rounded-2xl p-8 md:p-12 overflow-hidden flex flex-col md:flex-row justify-between items-center text-zinc-500 font-mono text-[11px] uppercase tracking-widest min-h-[500px]">
            {/* Subtle connecting lines overlay */}
            <div className="absolute inset-0 opacity-20 bg-[radial-gradient(circle_at_center,rgba(34,197,94,0.3)_1px,transparent_1px)] bg-[size:30px_30px]"></div>

            {/* Left Column: Raw Context */}
            <div className="flex flex-col gap-12 z-10 w-full md:w-1/4">
              <div className="text-zinc-600 mb-8 font-semibold">
                Realtime Raw Context
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-300">Code</span>
                <div className="flex gap-1">
                  <div className="w-1.5 h-1.5 rounded-full bg-zinc-700" />
                  <div className="w-1.5 h-1.5 rounded-full bg-zinc-700" />
                  <div className="w-1.5 h-1.5 rounded-full bg-primary shadow-[0_0_10px_rgba(34,197,94,0.5)]" />
                </div>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-300">Dependencies</span>
                <div className="flex gap-1">
                  <div className="w-1.5 h-1.5 rounded-full bg-zinc-700" />
                  <div className="w-1.5 h-1.5 rounded-full bg-primary shadow-[0_0_10px_rgba(34,197,94,0.5)]" />
                  <div className="w-1.5 h-1.5 rounded-full bg-zinc-700" />
                </div>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-300">Documentation</span>
                <div className="flex gap-1">
                  <div className="w-1.5 h-1.5 rounded-full bg-primary shadow-[0_0_10px_rgba(34,197,94,0.5)]" />
                  <div className="w-1.5 h-1.5 rounded-full bg-zinc-700" />
                  <div className="w-1.5 h-1.5 rounded-full bg-zinc-700" />
                </div>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-zinc-300">Recent Changes</span>
                <div className="flex gap-1">
                  <div className="w-1.5 h-1.5 rounded-full bg-zinc-700" />
                  <div className="w-1.5 h-1.5 rounded-full bg-primary shadow-[0_0_10px_rgba(34,197,94,0.5)]" />
                  <div className="w-1.5 h-1.5 rounded-full bg-zinc-700" />
                </div>
              </div>
            </div>

            {/* Center Column: Semantic Understanding (The Node Engine) */}
            <div className="flex-1 flex flex-col items-center justify-center relative z-10 min-h-[300px] w-full my-12 md:my-0">
              <div className="absolute top-0 text-zinc-600 font-semibold">
                Semantic Understanding
              </div>

              {/* Circular node graph abstraction */}
              <div className="relative w-[280px] h-[280px] rounded-full border border-white/5 flex items-center justify-center mt-12">
                <div className="absolute inset-0 rounded-full border border-dashed border-white/10 animate-[spin_60s_linear_infinite]" />
                <div className="w-[180px] h-[180px] rounded-full border border-white/5 flex items-center justify-center relative">
                  <div className="absolute inset-0 bg-primary/5 rounded-full blur-xl animate-pulse" />
                  {/* Center Node */}
                  <div className="w-8 h-8 rounded-full bg-[#09090B] border border-primary/50 shadow-[0_0_20px_rgba(34,197,94,0.4)] flex items-center justify-center z-10">
                    <div className="w-2 h-2 rounded-full bg-primary" />
                  </div>

                  {/* Orbiting nodes */}
                  <div className="absolute w-2 h-2 rounded-full bg-zinc-500 top-0 left-1/2 -ml-1" />
                  <div className="absolute w-1.5 h-1.5 rounded-full bg-zinc-700 bottom-4 right-8" />
                  <div className="absolute w-2.5 h-2.5 rounded-full bg-primary shadow-[0_0_10px_rgba(34,197,94,0.5)] left-2 top-1/3" />
                  <div className="absolute w-1 h-1 rounded-full bg-zinc-400 right-0 top-1/2" />
                </div>

                {/* Label popups matching reference image */}
                <div className="absolute bottom-4 left-4 bg-primary/20 border border-primary/40 text-primary px-2 py-0.5 rounded text-[9px] backdrop-blur-sm whitespace-nowrap">
                  router.ts
                </div>
                <div className="absolute top-8 right-2 bg-primary/20 border border-primary/40 text-primary px-2 py-0.5 rounded text-[9px] backdrop-blur-sm whitespace-nowrap">
                  middleware/auth.rs
                </div>
              </div>
            </div>

            {/* Right Column: Curated Context */}
            <div className="flex flex-col gap-12 z-10 w-full md:w-1/4 text-right">
              <div className="text-zinc-600 mb-8 font-semibold">
                Curated Context
              </div>
              <div className="flex items-center justify-end gap-3">
                <div className="w-2 h-2 rounded-full bg-primary shadow-[0_0_10px_rgba(34,197,94,0.5)]" />
                <span className="text-zinc-300">Completions</span>
              </div>
              <div className="flex items-center justify-end gap-3">
                <div className="w-2 h-2 rounded-full bg-primary shadow-[0_0_10px_rgba(34,197,94,0.5)]" />
                <span className="text-zinc-300">Code Review</span>
              </div>
              <div className="flex items-center justify-end gap-3">
                <div className="bg-primary text-black px-2 py-0.5 rounded text-[9px] font-bold">
                  omnicontext.mcp
                </div>
                <span className="text-zinc-300">Remote Agents</span>
              </div>
              <div className="flex items-center justify-end gap-3">
                <div className="w-2 h-2 rounded-full bg-zinc-700" />
                <span className="text-zinc-300">Chat</span>
              </div>

              <div className="mt-8 pt-6 border-t border-white/5 text-zinc-500 text-[10px]">
                42,104 sources → 12 relevant
              </div>
            </div>
          </div>
        </section>

        {/* Feature Sections - Alternating Interactive Blocks */}
        <section className="py-[160px] w-full max-w-[1280px] mx-auto flex flex-col gap-[200px] px-8 md:px-16 mb-20">
          {/* Block 1: Hybrid Retrieval */}
          <div className="flex flex-col md:flex-row items-stretch gap-16">
            <div className="flex-1 flex flex-col justify-center py-2">
              <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                Hybrid Search Engine
              </div>
              <h3 className="text-3xl md:text-4xl font-semibold text-white tracking-tight mb-5 leading-tight">
                Best-in-class retrieval precision.
              </h3>
              <p className="text-[16px] text-zinc-400 leading-relaxed mb-8">
                OmniContext does not rely on simple vector lookups. We fuse
                dense semantic vectors (usearch HNSW) with sparse exact-match
                keywords (SQLite FTS5) via Reciprocal Rank Fusion (RRF),
                ensuring your agents get exactly the context they need without
                hallucinating non-existent APIs.
              </p>
              <ul className="flex flex-col gap-3">
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                  usearch HNSW vector index
                </li>
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" /> SQLite
                  FTS5 for exact keyword matches
                </li>
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" /> RRF
                  fusion + heuristic boosting
                </li>
              </ul>
              <Link
                href="/docs"
                className="inline-flex items-center gap-2 mt-8 text-[14px] text-emerald-500 hover:text-emerald-400 font-medium transition-colors"
              >
                Learn about Hybrid Search <ChevronRight size={14} />
              </Link>
            </div>
            <div className="flex-[1.2] w-full relative flex flex-col py-4">
              <div className="w-full h-full min-h-[300px] bg-gradient-to-b from-white/[0.03] to-transparent border border-white/10 rounded-2xl shadow-2xl overflow-hidden flex flex-col backdrop-blur-3xl group">
                {/* Refined MacOS Header */}
                <div className="flex items-center px-5 h-11 border-b border-white/5 bg-white/[0.02] relative">
                  <div className="flex gap-2">
                    <div className="w-2.5 h-2.5 rounded-full bg-[#FF5F56] opacity-80" />
                    <div className="w-2.5 h-2.5 rounded-full bg-[#FFBD2E] opacity-80" />
                    <div className="w-2.5 h-2.5 rounded-full bg-[#27C93F] opacity-80" />
                  </div>
                  <div className="absolute left-1/2 -translate-x-1/2 text-[11px] font-medium text-zinc-500 font-mono tracking-tight opacity-70">
                    hybrid-search — active
                  </div>
                </div>

                {/* Terminal Content */}
                <div className="flex-1 p-6 pt-5 font-mono text-[13px] text-zinc-400 flex flex-col gap-4 relative overflow-hidden">
                  <div className="absolute top-0 right-0 w-64 h-64 bg-emerald-500/10 blur-[100px] pointer-events-none rounded-full" />
                  <div className="flex flex-col gap-2 relative z-10">
                    <div className="flex items-center gap-2 mb-2 text-zinc-300">
                      <span className="text-emerald-500 font-bold shrink-0">
                        ➜
                      </span>
                      <span className="truncate">
                        omni search --query &quot;auth middleware&quot;
                      </span>
                    </div>
                    <div className="relative">
                      <span className="text-zinc-500 font-bold">[1]</span> dense
                      search ... <span className="text-emerald-400">12ms</span>
                    </div>
                    <div className="relative">
                      <span className="text-zinc-500 font-bold">[2]</span>{" "}
                      sparse search ...{" "}
                      <span className="text-emerald-400">4ms</span>
                    </div>
                    <div className="relative">
                      <span className="text-zinc-500 font-bold">[3]</span> rank
                      fusion ...
                    </div>
                  </div>
                  <div className="pl-4 border-l-2 border-white/10 ml-1 mt-1 text-zinc-300 relative z-10 flex flex-col gap-3">
                    <div className="text-zinc-600 text-[10px] mb-1 font-sans uppercase tracking-[0.2em] font-bold">
                      Top Context Matches
                    </div>
                    <div className="flex items-center justify-between gap-4">
                      <div className="truncate flex-1">
                        <span className="text-emerald-500 mr-2">1.</span>{" "}
                        src/middleware/auth.rs
                      </div>
                      <span className="text-zinc-500 text-[10px] border border-white/10 px-1.5 py-0.5 rounded tracking-tighter">
                        0.0331
                      </span>
                    </div>
                    <div className="flex items-center justify-between gap-4">
                      <div className="truncate flex-1">
                        <span className="text-emerald-500 mr-2">2.</span>{" "}
                        tests/auth_integration.rs
                      </div>
                      <span className="text-zinc-500 text-[10px] border border-white/10 px-1.5 py-0.5 rounded tracking-tighter">
                        0.0325
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
          {/* Block 2: Dependency Graph */}
          <div className="flex flex-col md:flex-row-reverse items-stretch gap-16">
            <div className="flex-1 flex flex-col justify-center py-2">
              <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                Structural Understanding
              </div>
              <h3 className="text-3xl md:text-4xl font-semibold text-white tracking-tight mb-5 leading-tight">
                A deeply connected dependency web.
              </h3>
              <p className="text-[16px] text-zinc-400 leading-relaxed mb-8">
                Code is not text; it is deeply structural. OmniContext uses
                Tree-sitter to parse your entire workspace, extracting
                functions, classes, and cross-file imports into an in-memory
                `petgraph` network.
              </p>
              <ul className="flex flex-col gap-3">
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                  Tree-sitter AST extraction
                </li>
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                  In-memory Petgraph traversal
                </li>
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                  Instantly resolve caller/callee graphs
                </li>
              </ul>
              <Link
                href="/docs"
                className="inline-flex items-center gap-2 mt-8 text-[14px] text-emerald-500 hover:text-emerald-400 font-medium transition-colors"
              >
                Explore the Graph API <ChevronRight size={14} />
              </Link>
            </div>
            <div className="flex-[1.2] w-full relative flex flex-col py-4">
              <div className="w-full h-full min-h-[300px] bg-gradient-to-b from-[#0e0c15] to-[#07050a] border border-indigo-500/20 rounded-2xl shadow-2xl overflow-hidden flex flex-col backdrop-blur-3xl group">
                {/* Refined MacOS Header */}
                <div className="flex flex-row items-center px-5 h-11 border-b border-indigo-500/10 bg-white/[0.01] relative">
                  <div className="flex gap-2">
                    <div className="w-2.5 h-2.5 rounded-full bg-[#FF5F56] opacity-80" />
                    <div className="w-2.5 h-2.5 rounded-full bg-[#FFBD2E] opacity-80" />
                    <div className="w-2.5 h-2.5 rounded-full bg-[#27C93F] opacity-80" />
                  </div>
                  <div className="absolute left-1/2 -translate-x-1/2 text-[11px] font-medium text-indigo-400/40 font-mono tracking-tight opacity-70">
                    petgraph-trace
                  </div>
                </div>

                {/* Terminal Content */}
                <div className="flex-1 p-6 pt-5 font-mono text-[13px] leading-[1.8] flex flex-col relative overflow-hidden">
                  <div className="absolute top-0 left-0 w-full h-32 bg-indigo-500/10 blur-[100px] pointer-events-none" />
                  <div className="text-indigo-400 font-bold text-[14px] mb-5 tracking-tight flex items-center gap-2 relative z-10">
                    <span className="text-indigo-500 opacity-60">❯</span> omni
                    trace{" "}
                    <span className="text-zinc-500 font-normal">
                      UserService::validate()
                    </span>
                  </div>
                  <div className="relative z-10 mb-6 flex flex-col gap-1.5">
                    <div className="text-[10px] text-zinc-500 uppercase tracking-widest font-sans font-bold">
                      Semantic Dependencies
                    </div>
                    <div className="h-[1px] w-6 bg-indigo-500/40" />
                  </div>
                  <div className="pl-4 border-l-2 border-indigo-500/20 text-zinc-300 flex flex-col gap-4 text-[12px] relative z-10">
                    <div className="flex items-center gap-3">
                      <span className="text-indigo-400/80 mr-1 opacity-50">
                        ↳
                      </span>
                      <span className="text-zinc-600 text-[9px] font-bold border border-zinc-800 px-1 py-0.5 rounded text-center tracking-tighter uppercase">
                        Call
                      </span>
                      <span className="text-zinc-200">auth::verify_token</span>
                    </div>
                    <div className="flex items-center gap-3 pl-6">
                      <span className="text-indigo-400/80 mr-1 opacity-50">
                        ↳
                      </span>
                      <span className="text-zinc-600 text-[9px] font-bold border border-zinc-800 px-1 py-0.5 rounded text-center tracking-tighter uppercase">
                        Import
                      </span>
                      <span className="text-orange-400/70">
                        jsonwebtoken::decode
                      </span>
                    </div>
                    <div className="flex items-center gap-3">
                      <span className="text-indigo-400/80 mr-1 opacity-50">
                        ↳
                      </span>
                      <span className="text-zinc-600 text-[9px] font-bold border border-zinc-800 px-1 py-0.5 rounded text-center tracking-tighter uppercase">
                        Call
                      </span>
                      <span className="text-zinc-200">db::fetch_user</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          {/* Block 3: Native MCP */}
          <div className="flex flex-col md:flex-row items-stretch gap-16">
            <div className="flex-1 flex flex-col justify-center py-2">
              <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                Agent Protocol
              </div>
              <h3 className="text-3xl md:text-4xl font-semibold text-white tracking-tight mb-5 leading-tight">
                Native MCP Server Integration.
              </h3>
              <p className="text-[16px] text-zinc-400 leading-relaxed mb-8">
                OmniContext does not try to be an AI agent; it empowers the ones
                you already use. It runs fully locally as a standard Model
                Context Protocol (MCP) server over `stdio` or `sse`.
              </p>
              <ul className="flex flex-col gap-3">
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                  Provides 8 powerful MCP tools
                </li>
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                  Connects to Claude Code &amp; Cursor
                </li>
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" /> Stdio
                  and HTTP SSE transports
                </li>
              </ul>
              <Link
                href="/docs"
                className="inline-flex items-center gap-2 mt-8 text-[14px] text-emerald-500 hover:text-emerald-400 font-medium transition-colors"
              >
                View MCP Configuration <ChevronRight size={14} />
              </Link>
            </div>
            <div className="flex-[1.2] w-full relative flex flex-col py-4">
              {/* VS Code / IDE Editor Mockup */}
              <div className="w-full h-full min-h-[300px] bg-[#121214] border border-white/10 rounded-2xl shadow-2xl overflow-hidden font-sans flex flex-col backdrop-blur-3xl group">
                {/* Editor Tabs & Controls */}
                <div className="flex items-center h-11 bg-[#1A1A1C] border-b border-black/50 select-none">
                  <div className="flex items-center gap-2 px-5 h-full border-r border-black/50">
                    <div className="w-2.5 h-2.5 rounded-full bg-[#FF5F56] opacity-80" />
                    <div className="w-2.5 h-2.5 rounded-full bg-[#FFBD2E] opacity-80" />
                    <div className="w-2.5 h-2.5 rounded-full bg-[#27C93F] opacity-80" />
                  </div>
                  {/* Active Tab */}
                  <div className="px-5 h-full flex items-center gap-2.5 bg-[#121214] border-r border-black/50 relative">
                    <div className="absolute top-0 left-0 w-full h-[2px] bg-blue-500" />
                    <span className="text-[#FFC107] text-[13px] font-mono">
                      {"{ }"}
                    </span>
                    <span className="text-[12px] text-zinc-300 font-medium tracking-wide">
                      claude_desktop_config.json
                    </span>
                    <div className="w-4 h-4 ml-4 rounded flex items-center justify-center text-[10px] cursor-pointer text-zinc-500 hover:bg-white/10 transition-colors">
                      ✕
                    </div>
                  </div>
                </div>

                {/* Editor Content Area */}
                <div className="flex-1 p-6 flex flex-row relative">
                  {/* Line Numbers */}
                  <div className="flex flex-col text-[13px] font-mono text-zinc-700 pr-5 select-none border-r border-white/5 text-right font-medium leading-[1.7]">
                    <span>1</span>
                    <span>2</span>
                    <span>3</span>
                    <span>4</span>
                    <span>5</span>
                    <span>6</span>
                    <span>7</span>
                    <span>8</span>
                    <span>9</span>
                  </div>

                  {/* Code Body */}
                  <div className="font-mono text-[13px] leading-[1.7] pl-5 flex flex-col w-full overflow-x-auto">
                    <div>
                      <span className="text-zinc-500">{"{"}</span>
                    </div>
                    <div>
                      {"  "}
                      <span className="text-[#9CDCFE]">
                        &quot;mcpServers&quot;
                      </span>
                      <span className="text-zinc-400">:</span>{" "}
                      <span className="text-zinc-500">{"{"}</span>
                    </div>
                    <div>
                      {"    "}
                      <span className="text-[#9CDCFE]">
                        &quot;omnicontext&quot;
                      </span>
                      <span className="text-zinc-400">:</span>{" "}
                      <span className="text-zinc-500">{"{"}</span>
                    </div>
                    <div>
                      {"      "}
                      <span className="text-[#9CDCFE]">
                        &quot;command&quot;
                      </span>
                      <span className="text-zinc-400">:</span>{" "}
                      <span className="text-[#CE9178]">
                        &quot;omnicontext-mcp&quot;
                      </span>
                      <span className="text-zinc-400">,</span>
                    </div>
                    <div>
                      {"      "}
                      <span className="text-[#9CDCFE]">&quot;args&quot;</span>
                      <span className="text-zinc-400">:</span>{" "}
                      <span className="text-zinc-500">[</span>
                      <span className="text-[#CE9178]">
                        &quot;--transport&quot;
                      </span>
                      <span className="text-zinc-400">,</span>{" "}
                      <span className="text-[#CE9178]">&quot;stdio&quot;</span>
                      <span className="text-zinc-400">,</span>{" "}
                      <span className="text-[#CE9178]">&quot;--repo&quot;</span>
                      <span className="text-zinc-400">,</span>{" "}
                      <span className="text-[#CE9178]">&quot;.&quot;</span>
                      <span className="text-zinc-500">]</span>
                      <span className="text-zinc-400">,</span>
                    </div>
                    <div>
                      {"      "}
                      <span className="text-[#9CDCFE]">&quot;env&quot;</span>
                      <span className="text-zinc-400">:</span>{" "}
                      <span className="text-zinc-500">{"{}"}</span>
                    </div>
                    <div>
                      {"    "}
                      <span className="text-zinc-500">{"}"}</span>
                    </div>
                    <div>
                      {"  "}
                      <span className="text-zinc-500">{"}"}</span>
                    </div>
                    <div>
                      <span className="text-zinc-500">{"}"}</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Footer - Enterprise Grade */}
        <footer className="py-16 px-8 md:px-16 bg-[#09090B] border-t border-white/5">
          <div className="max-w-[1200px] mx-auto">
            <div className="grid grid-cols-2 md:grid-cols-5 gap-12 mb-16">
              <div className="col-span-2 md:col-span-1">
                <Link
                  href="/"
                  className="flex items-center gap-2 font-semibold text-sm text-zinc-100 mb-4"
                >
                  <Logo className="text-primary" size={16} />
                  <span>{siteConfig.name}</span>
                </Link>
                <p className="text-[12px] text-zinc-600 leading-relaxed">
                  High-performance code context engine. Open-source core. Built
                  in Rust.
                </p>
              </div>
              <div>
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Product
                </div>
                <div className="flex flex-col gap-2 text-[13px] text-zinc-500">
                  <Link
                    href="/docs"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Documentation
                  </Link>
                  <Link
                    href="/docs/quickstart"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Quickstart
                  </Link>
                  <Link
                    href="/docs/pricing"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Pricing
                  </Link>
                  <Link
                    href="/enterprise"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Enterprise
                  </Link>
                </div>
              </div>
              <div>
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Resources
                </div>
                <div className="flex flex-col gap-2 text-[13px] text-zinc-500">
                  <Link
                    href="/blog"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Blog
                  </Link>
                  <Link
                    href="/docs/architecture"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Architecture
                  </Link>
                  <Link
                    href="/docs/supported-languages"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Supported Languages
                  </Link>
                </div>
              </div>
              <div>
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Company
                </div>
                <div className="flex flex-col gap-2 text-[13px] text-zinc-500">
                  <Link
                    href="/support"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Support
                  </Link>
                  <Link
                    href={`mailto:${siteConfig.links.email}`}
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Contact
                  </Link>
                  <a
                    href={siteConfig.links.github}
                    className="hover:text-zinc-200 transition-colors"
                  >
                    GitHub
                  </a>
                </div>
              </div>
              <div>
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Legal
                </div>
                <div className="flex flex-col gap-2 text-[13px] text-zinc-500">
                  <Link
                    href="/privacy"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Privacy Policy
                  </Link>
                  <Link
                    href="/terms"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Terms of Service
                  </Link>
                </div>
              </div>
            </div>
            <div className="border-t border-white/5 pt-8 flex flex-col md:flex-row items-center justify-between gap-4">
              <p className="text-[12px] text-zinc-600">
                (c) 2026 {siteConfig.name}. All rights reserved. Apache-2.0
                License.
              </p>
              <div className="flex items-center gap-4">
                <a
                  href={siteConfig.links.github}
                  className="text-zinc-600 hover:text-zinc-300 transition-colors"
                >
                  <Github size={18} />
                </a>
              </div>
            </div>
          </div>
        </footer>
      </main>
    </div>
  );
}
