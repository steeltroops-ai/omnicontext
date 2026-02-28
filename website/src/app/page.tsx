"use client";

import { motion } from "framer-motion";
import { ChevronRight, Terminal, Zap } from "lucide-react";
import Link from "next/link";
import { siteConfig } from "@/config/site";
import { DottedSurface } from "@/components/ui/dotted-surface";
import { SiteNav } from "@/components/site-nav";
import { SiteFooterFull } from "@/components/site-footer-full";

export default function Home() {
  return (
    <div className="flex flex-col h-screen overflow-hidden bg-[#09090B] selection:bg-primary/30">
      <SiteNav transparent scrollTarget="main-scroll" />

      <main
        id="main-scroll"
        className="flex-1 overflow-y-scroll custom-scrollbar flex flex-col pt-0"
      >
        <section className="relative w-full flex items-center min-h-screen pt-14 pb-16 overflow-hidden">
          <DottedSurface className="absolute inset-0 z-0 opacity-50" />

          {/* Subtle background glow */}
          <div className="absolute top-[30%] left-[-10%] w-[600px] h-[500px] bg-primary/10 blur-[130px] rounded-full pointer-events-none" />
          <div className="absolute bottom-[-10%] right-[-10%] w-[800px] h-[600px] bg-emerald-500/5 blur-[150px] rounded-full pointer-events-none" />

          <div className="relative z-10 flex flex-col lg:flex-row items-center lg:items-center justify-between px-6 sm:px-8 md:px-16 w-full max-w-[1400px] mx-auto gap-10 lg:gap-16 xl:gap-24 py-12 lg:py-0">
            {/* Left Content */}
            <div className="flex-1 flex flex-col items-center lg:items-start text-center lg:text-left w-full max-w-[640px] mx-auto lg:mx-0">
              <motion.div
                initial={{ opacity: 0, y: 15 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.8, ease: [0.16, 1, 0.1, 1] }}
                className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-900/50 border border-white/5 text-[13px] font-medium text-zinc-300 mb-8 backdrop-blur-md cursor-pointer hover:bg-zinc-800/50 transition-colors"
              >
                <div className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse" />
                <span>v0.1.0 is now available</span>
                <ChevronRight size={14} className="text-zinc-500" />
              </motion.div>

              <motion.h1
                className="text-4xl sm:text-5xl md:text-6xl lg:text-7xl xl:text-[84px] font-semibold tracking-tighter text-transparent bg-clip-text bg-gradient-to-b from-white to-white/70 mb-6 leading-[1.05]"
                initial={{ opacity: 0, scale: 0.96, filter: "blur(10px)" }}
                animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
                transition={{
                  duration: 1.2,
                  ease: [0.16, 1, 0.1, 1],
                  delay: 0.1,
                }}
              >
                The context engine <br className="hidden md:block" /> your
                codebase deserves.
              </motion.h1>

              <motion.p
                className="text-[17px] sm:text-[19px] md:text-[21px] text-zinc-400 max-w-lg mb-10 leading-snug tracking-tight"
                initial={{ opacity: 0, y: 15 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{
                  duration: 0.8,
                  ease: [0.16, 1, 0.1, 1],
                  delay: 0.2,
                }}
              >
                {siteConfig.name} represents a fundamental shift in AI coding.
                Universal dependency awareness, written in Rust, and executed
                flawlessly on your local machine.
              </motion.p>

              <motion.div
                className="flex flex-col sm:flex-row gap-3 w-full sm:w-auto justify-center lg:justify-start"
                initial={{ opacity: 0, y: 15 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{
                  duration: 0.8,
                  ease: [0.16, 1, 0.1, 1],
                  delay: 0.3,
                }}
              >
                <button className="w-full sm:w-auto px-7 py-3.5 text-[15px] font-medium rounded-full bg-zinc-100 text-black hover:scale-105 active:scale-95 transition-all duration-300 shadow-[0_0_40px_rgba(255,255,255,0.1)]">
                  Install {siteConfig.name} CLI
                </button>
                <Link
                  href="/docs"
                  className="w-full sm:w-auto px-7 py-3.5 text-[15px] font-medium rounded-full bg-zinc-900 text-white border border-white/10 hover:bg-zinc-800 transition-colors duration-300 flex items-center justify-center"
                >
                  Read Docs
                </Link>
              </motion.div>
            </div>

            {/* Right Hero Visual (Elegant Dark-Glass Terminal) */}
            <motion.div
              className="flex-[1.3] w-full relative z-10 hidden lg:flex flex-col"
              initial={{ opacity: 0, x: 40 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{
                duration: 1.2,
                ease: [0.16, 1, 0.3, 1],
                delay: 0.4,
              }}
            >
              {/* True liquid glass — near-zero opacity, minimal blur */}
              <div
                className="w-full relative rounded-2xl overflow-hidden flex flex-col group transform-gpu"
                style={{
                  background: "rgba(255, 255, 255, 0.02)",
                  border: "1px solid rgba(255,255,255,0.08)",
                  boxShadow:
                    "0 24px 80px rgba(0,0,0,0.55), 0 0 0 0.5px rgba(255,255,255,0.04) inset, 0 1px 0 rgba(255,255,255,0.07) inset",
                  backdropFilter: "blur(8px)",
                  WebkitBackdropFilter: "blur(8px)",
                }}
              >
                {/* Interior ambient color tints */}
                <div className="absolute top-[-20%] left-[-5%] w-[55%] h-[55%] bg-emerald-500/[0.07] blur-[90px] pointer-events-none rounded-full" />
                <div className="absolute bottom-[-15%] right-[0%] w-[45%] h-[50%] bg-indigo-500/[0.07] blur-[90px] pointer-events-none rounded-full" />

                {/* Top specular shine — glass refraction line */}
                <div className="absolute top-0 left-0 w-full h-px bg-gradient-to-r from-transparent via-white/20 to-transparent pointer-events-none" />
                {/* Left edge shine */}
                <div className="absolute top-0 left-0 w-px h-full bg-gradient-to-b from-white/[0.12] to-transparent pointer-events-none" />

                {/* Window Chrome — same liquid glass treatment */}
                <div
                  className="w-full h-10 flex items-center justify-between px-5 relative z-20 border-b"
                  style={{
                    borderColor: "rgba(255,255,255,0.06)",
                    background: "rgba(255,255,255,0.03)",
                  }}
                >
                  <div className="flex gap-2">
                    <div className="w-2.5 h-2.5 rounded-full bg-[#FF5F56]/80 shadow-[0_0_6px_rgba(255,95,86,0.3)]" />
                    <div className="w-2.5 h-2.5 rounded-full bg-[#FFBD2E]/80 shadow-[0_0_6px_rgba(255,189,46,0.3)]" />
                    <div className="w-2.5 h-2.5 rounded-full bg-[#27C93F]/80 shadow-[0_0_6px_rgba(39,201,63,0.3)]" />
                  </div>
                  <div className="flex items-center gap-1.5 text-[10px] font-mono font-medium text-zinc-500 uppercase tracking-widest">
                    <Terminal size={9} />
                    <span>omni — daemon</span>
                  </div>
                  <div className="w-[60px]" />
                </div>

                {/* Window Content */}
                <div className="w-full p-6 md:p-8 text-left relative z-10 font-mono text-[13px] leading-[1.9] text-zinc-300 tracking-tight overflow-x-auto">
                  {/* Command 1 */}
                  <div className="flex items-center gap-3 mb-4 font-semibold text-zinc-200">
                    <span className="text-emerald-500 shrink-0">❯</span>
                    <span className="truncate">
                      omnicontext index ./omnicontext
                    </span>
                  </div>

                  <div className="pl-4 border-l-2 border-emerald-500/20 ml-[5px] flex flex-col gap-3 my-5 text-[12px]">
                    <div className="flex flex-row items-center justify-between max-w-[380px]">
                      <span className="text-zinc-500 font-bold uppercase tracking-widest w-[100px]">
                        Indexing
                      </span>
                      <span className="text-zinc-400">
                        Building semantic chunks...
                      </span>
                      <span className="text-emerald-500 font-bold opacity-80 text-right w-12">
                        140ms
                      </span>
                    </div>
                    <div className="flex flex-row items-center justify-between max-w-[380px]">
                      <span className="text-zinc-500 font-bold uppercase tracking-widest w-[100px]">
                        Embedding
                      </span>
                      <span className="text-zinc-400">
                        Generating ONNX local vectors...
                      </span>
                      <span className="text-emerald-500 font-bold opacity-80 text-right w-12">
                        1.2s
                      </span>
                    </div>
                    <div className="flex flex-row items-center justify-between max-w-[380px]">
                      <span className="text-zinc-500 font-bold uppercase tracking-widest w-[100px]">
                        Graphing
                      </span>
                      <span className="text-zinc-400">
                        Computing dependency graph...
                      </span>
                      <span className="text-emerald-500 font-bold opacity-80 text-right w-12">
                        450ms
                      </span>
                    </div>

                    <div className="text-primary font-semibold tracking-wide mt-3 flex items-center gap-2">
                      <Zap size={14} className="fill-primary/20 text-primary" />
                      Successfully indexed 42,104 files in 2.1s
                    </div>
                  </div>

                  {/* Command 2 */}
                  <div className="mt-8 flex items-center gap-3 mb-4 font-semibold text-zinc-200">
                    <span className="text-emerald-500 shrink-0">❯</span>
                    <span className="truncate">
                      omnicontext mcp --repo ./omnicontext
                    </span>
                  </div>

                  <div className="pl-4 border-l-2 border-indigo-500/20 ml-[5px] flex flex-col gap-2 my-5 text-[12px]">
                    <div className="text-indigo-400 font-semibold tracking-wide flex items-center gap-3">
                      <div className="w-2.5 h-2.5 rounded-full bg-indigo-500 animate-[pulse_2s_ease-in-out_infinite] shadow-[0_0_12px_rgba(99,102,241,0.6)]" />
                      OmniContext MCP Server active on stdio
                    </div>
                    <div className="text-zinc-500 mt-1 flex items-center gap-2">
                      Ready to serve advanced code intelligence to your agent.
                    </div>
                  </div>
                </div>
              </div>
            </motion.div>
          </div>
        </section>

        {/* Architecture Banner */}
        <section className="py-[140px] px-8 md:px-16 w-full max-w-[1400px] mx-auto border-t border-white/5 relative flex flex-col items-center">
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
        <section className="py-[140px] w-full max-w-[1400px] mx-auto flex flex-col gap-[180px] px-8 md:px-16 mb-20">
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
              <div className="w-full h-full min-h-[340px] bg-gradient-to-b from-white/[0.03] to-transparent border border-white/10 rounded-2xl shadow-2xl overflow-hidden flex flex-col backdrop-blur-3xl group">
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
                Local-First Performance
              </div>
              <h3 className="text-3xl md:text-4xl font-semibold text-white tracking-tight mb-5 leading-tight">
                Your code never leaves your hardware.
              </h3>
              <p className="text-[16px] text-zinc-400 leading-relaxed mb-8">
                OmniContext is a zero-latency, local-first engine. By executing
                entirely on your machine with a highly parallel Rust backend and
                local ONNX embeddings, we ensure your code stays private and
                your agents stay fast—no cloud dependencies required.
              </p>
              <ul className="flex flex-col gap-3">
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" /> 100%
                  local ONNX model inference
                </li>
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                  Parallelized indexing (10k files &lt; 60s)
                </li>
                <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                  <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                  Encrypted SQLite index with WAL concurrency
                </li>
              </ul>
              <Link
                href="/docs"
                className="inline-flex items-center gap-2 mt-8 text-[14px] text-emerald-500 hover:text-emerald-400 font-medium transition-colors"
              >
                Performance Benchmarks <ChevronRight size={14} />
              </Link>
            </div>
            <div className="flex-[1.2] w-full relative flex flex-col py-4">
              <div className="w-full h-full min-h-[340px] bg-gradient-to-b from-[#0e0c15] to-[#07050a] border border-emerald-500/10 rounded-2xl shadow-2xl overflow-hidden flex flex-col backdrop-blur-3xl group">
                {/* Refined MacOS Header */}
                <div className="flex flex-row items-center px-5 h-11 border-b border-indigo-500/10 bg-white/[0.01] relative">
                  <div className="flex gap-2">
                    <div className="w-2.5 h-2.5 rounded-full bg-[#FF5F56] opacity-80" />
                    <div className="w-2.5 h-2.5 rounded-full bg-[#FFBD2E] opacity-80" />
                    <div className="w-2.5 h-2.5 rounded-full bg-[#27C93F] opacity-80" />
                  </div>
                  <div className="absolute left-1/2 -translate-x-1/2 text-[11px] font-medium text-emerald-400/40 font-mono tracking-tight opacity-70">
                    omnicontext-engine — status
                  </div>
                </div>

                {/* Terminal Content */}
                <div className="flex-1 p-6 pt-5 font-mono text-[13px] leading-[1.8] flex flex-col relative overflow-hidden">
                  <div className="absolute top-0 left-0 w-full h-32 bg-emerald-500/10 blur-[100px] pointer-events-none" />
                  <div className="text-emerald-400 font-bold text-[14px] mb-5 tracking-tight flex items-center gap-2 relative z-10">
                    <span className="text-emerald-500 opacity-60">❯</span> omni
                    status --verbose
                  </div>

                  <div className="relative z-10 mb-6 flex flex-col gap-2.5">
                    <div className="text-[10px] text-zinc-500 uppercase tracking-widest font-sans font-bold">
                      System Telemetry
                    </div>
                    <div className="flex flex-col gap-1.5 border-l-2 border-emerald-500/20 pl-4">
                      <div className="flex justify-between items-center text-zinc-400">
                        <span>Binary Runtime</span>
                        <span className="text-zinc-200">Rust / Static</span>
                      </div>
                      <div className="flex justify-between items-center text-zinc-400">
                        <span>Memory RSS</span>
                        <span className="text-zinc-200">84 MB</span>
                      </div>
                      <div className="flex justify-between items-center text-zinc-400">
                        <span>Model Latency</span>
                        <span className="text-zinc-200">14ms (CPU)</span>
                      </div>
                    </div>
                  </div>

                  <div className="relative z-10 flex flex-col gap-2.5">
                    <div className="text-[10px] text-zinc-500 uppercase tracking-widest font-sans font-bold">
                      Index Integrity
                    </div>
                    <div className="flex flex-col gap-1.5 border-l-2 border-primary/20 pl-4">
                      <div className="flex items-center gap-2">
                        <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(34,197,94,0.5)]" />
                        <span className="text-zinc-300">
                          SQLite FTS5 (Fossilized)
                        </span>
                      </div>
                      <div className="flex items-center gap-2">
                        <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(34,197,94,0.5)]" />
                        <span className="text-zinc-300">
                          HNSW Vector Index (MMAP)
                        </span>
                      </div>
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
              <div className="w-full h-full min-h-[340px] bg-[#121214] border border-white/10 rounded-2xl shadow-2xl overflow-hidden font-sans flex flex-col backdrop-blur-3xl group">
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

        <SiteFooterFull />
      </main>
    </div>
  );
}
