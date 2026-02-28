"use client";

import { motion } from "framer-motion";
import {
  Terminal,
  ShieldCheck,
  Github,
  SearchCode,
  ChevronRight,
  Zap,
} from "lucide-react";
import Link from "next/link";

export default function Home() {
  return (
    <main className="flex flex-col min-h-screen bg-black font-sans selection:bg-primary/30">
      {/* Navigation - Apple Style Ultra-Minimal */}
      <nav className="fixed top-0 w-full h-14 flex items-center justify-center z-50 border-b border-white/5 bg-black/50 backdrop-blur-xl">
        <div className="flex items-center justify-between w-full max-w-[1000px] px-6">
          <Link
            href="/"
            className="flex items-center gap-2 font-semibold text-sm text-zinc-100 transition-opacity hover:opacity-80"
          >
            <SearchCode className="text-primary" size={18} strokeWidth={2} />
            <span>OmniContext</span>
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
            <Link
              href="/login"
              className="text-[13px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors hidden sm:block"
            >
              Sign In
            </Link>
            <button className="px-3 py-1.5 text-[13px] font-medium rounded-full bg-zinc-100 text-black hover:bg-white transition-colors">
              Get Started
            </button>
          </div>
        </div>
      </nav>

      {/* Hero Section */}
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
          dependency awareness, written in Rust, and executed flawlessly on your
          local machine.
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

        {/* Hero Visual (Elegant Terminal Mockup) */}
        <motion.div
          className="mt-28 w-full max-w-[900px] relative z-10 mx-auto"
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 1.2, ease: [0.16, 1, 0.3, 1], delay: 0.4 }}
        >
          <div className="w-full rounded-[20px] bg-[#050505] border border-white/10 shadow-[0_30px_100px_rgba(0,0,0,0.9)] overflow-hidden flex flex-col items-center">
            {/* Window Chrome */}
            <div className="w-full h-12 bg-[#0d0d0d] border-b border-white/5 flex items-center px-5 justify-between relative">
              <div className="flex gap-2 relative z-10">
                <div className="w-3 h-3 rounded-full bg-[#555] border border-white/5"></div>
                <div className="w-3 h-3 rounded-full bg-[#555] border border-white/5"></div>
                <div className="w-3 h-3 rounded-full bg-[#555] border border-white/5"></div>
              </div>
              <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                <span className="text-[12px] font-medium text-zinc-500 tracking-wide flex items-center gap-2">
                  <Terminal size={12} />
                  omnicontext — local
                </span>
              </div>
            </div>
            {/* Window Content */}
            <div className="w-full p-8 text-left overflow-x-auto">
              <pre className="font-mono text-[14px] leading-[2] text-zinc-300 tracking-tight">
                <span className="text-zinc-600">$ </span>
                <span className="text-zinc-100">omnicontext</span> index
                ./workspace{"\n"}
                <span className="text-zinc-600">[14:02:12]</span> Building
                semantic chunks...{"\n"}
                <span className="text-zinc-600">[14:02:13]</span> Generating
                embeddings (ONNX local)...{"\n"}
                <span className="text-zinc-600">[14:02:14]</span> Computing
                dependency graph...{"\n"}
                <span className="text-primary font-medium">
                  ✓ Successfully indexed 42,104 files in 2.1s
                </span>
                {"\n"}
                {"\n"}
                <span className="text-zinc-600">$ </span>
                <span className="text-zinc-100">omnicontext-mcp</span> --repo
                ./workspace{"\n"}
                <span className="text-primary font-medium">
                  ► OmniContext MCP Server listening on stdio...
                </span>
                {"\n"}
                <span className="text-zinc-500">
                  {" "}
                  Ready to serve advanced code intelligence to your agent.
                </span>
              </pre>
            </div>
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
          From IDE to CLI to autonomous code review, OmniContext works locally.
        </p>

        {/* Graph Mockup Container */}
        <div className="w-full relative bg-[#050505] border border-white/10 rounded-2xl p-8 md:p-12 overflow-hidden flex flex-col md:flex-row justify-between items-center text-zinc-500 font-mono text-[11px] uppercase tracking-widest min-h-[500px]">
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
                <div className="w-8 h-8 rounded-full bg-black border border-primary/50 shadow-[0_0_20px_rgba(34,197,94,0.4)] flex items-center justify-center z-10">
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
        {/* Block 1: Left Text, Right Interactive */}
        <div className="flex flex-col md:flex-row items-center gap-16">
          <div className="flex-1">
            <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
              Intent
            </div>
            <h3 className="text-3xl md:text-4xl font-semibold text-white tracking-tight mb-5 leading-tight">
              Universal Agent Readiness.
            </h3>
            <p className="text-[16px] text-zinc-400 leading-relaxed mb-8">
              A developer workspace where agents are coordinated, specs stay
              alive, and every context scope is isolated. Designed primarily for
              agent architectures instead of sparse vector lookups.
            </p>
            <ul className="flex flex-col gap-3">
              <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                <div className="w-1 h-1 rounded-full bg-emerald-500" /> MCP
                server native by default
              </li>
              <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                <div className="w-1 h-1 rounded-full bg-emerald-500" /> Works
                with Windsurf, Cursor, Code
              </li>
              <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                Sub-second AST graph retrievals
              </li>
            </ul>
            <Link
              href="/docs"
              className="inline-flex items-center gap-2 mt-8 text-[14px] text-emerald-500 hover:text-emerald-400 font-medium transition-colors"
            >
              Build with intent <ChevronRight size={14} />
            </Link>
          </div>
          <div className="flex-[1.2] w-full relative">
            {/* Interactive Mockup */}
            <div className="w-full bg-[#0a0a0a] border border-white/10 rounded-[16px] p-6 shadow-2xl relative overflow-hidden group">
              <div className="absolute top-0 right-1/4 w-32 h-32 bg-emerald-500/10 blur-[50px] pointer-events-none transition-opacity duration-700 opacity-50 group-hover:opacity-100" />
              <div className="flex items-center gap-2 mb-6 border-b border-white/5 pb-4">
                <SearchCode size={16} className="text-zinc-500" />
                <span className="text-[12px] font-mono text-zinc-500">
                  omni-mcp-client
                </span>
              </div>
              <div className="font-mono text-[11px] text-zinc-400 flex flex-col gap-3 leading-[1.6]">
                <div>
                  <span className="text-emerald-400">Agent:</span> Fetching
                  dependencies for UserProfile...
                </div>
                <div className="pl-4 border-l border-white/10 ml-1">
                  <div>[Core] Found User object in schemas.rs</div>
                  <div>[Core] Found UserProfile in components/Profile.tsx</div>
                  <div className="text-zinc-600">
                    Building relational graph (0.12s)
                  </div>
                </div>
                <div>
                  <span className="text-emerald-400">Agent:</span> Context
                  assembled. Over 1,200 tokens loaded from 4 verified files.
                  Generating response...
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Block 2: Left Interactive, Right Text */}
        <div className="flex flex-col md:flex-row-reverse items-center gap-16">
          <div className="flex-1">
            <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
              Terminal
            </div>
            <h3 className="text-3xl md:text-4xl font-semibold text-white tracking-tight mb-5 leading-tight">
              Omni CLI
            </h3>
            <p className="text-[16px] text-zinc-400 leading-relaxed mb-8">
              AI-powered coding in your terminal. For engineers who prefer the
              command line. Same Context Engine, same powerful intelligence,
              zero GUI overhead.
            </p>
            <ul className="flex flex-col gap-3">
              <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                <div className="w-1 h-1 rounded-full bg-emerald-500" /> Full
                terminal integration
              </li>
              <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                <div className="w-1 h-1 rounded-full bg-emerald-500" /> Works
                alongside your shell
              </li>
              <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                <div className="w-1 h-1 rounded-full bg-emerald-500" /> Embedded
                SQLite vector sync
              </li>
            </ul>
            <Link
              href="/docs/cli"
              className="inline-flex items-center gap-2 mt-8 text-[14px] text-emerald-500 hover:text-emerald-400 font-medium transition-colors"
            >
              Start building today <ChevronRight size={14} />
            </Link>
          </div>
          <div className="flex-[1.2] w-full relative">
            {/* CLI Mockup */}
            <div className="w-full bg-[#0d0a15] border border-blue-500/10 rounded-[16px] p-6 shadow-2xl font-mono text-[12px] leading-[1.8] overflow-hidden">
              <div className="flex gap-2 mb-6">
                <div className="w-2.5 h-2.5 rounded-full bg-[#555]"></div>
                <div className="w-2.5 h-2.5 rounded-full bg-[#555]"></div>
                <div className="w-2.5 h-2.5 rounded-full bg-[#555]"></div>
              </div>
              <div className="text-indigo-400 font-bold text-[24px] mb-4 tracking-tighter leading-none">
                OMNI
              </div>
              <div className="text-indigo-900 mb-6 uppercase tracking-widest text-[9px]">
                Context Engine CLI
              </div>
              <div className="text-zinc-300 mb-4">
                Personalized code queries mapped for you{" "}
                <span className="text-zinc-600">(Cmd + K)</span>
              </div>
              <div className="pl-3 border-l-2 border-indigo-500/30 text-zinc-400 mb-6 flex flex-col gap-2">
                <div>
                  <span className="text-indigo-400">✦</span> Find dead code
                  across the React monorepo
                </div>
                <div>
                  <span className="text-indigo-400">✦</span> Resolve missing
                  implementations
                </div>
                <div>
                  <span className="text-indigo-400">✦</span> Scaffold a new Rust
                  crate
                </div>
              </div>
              <div className="bg-white/5 border border-white/10 p-3 rounded-md text-zinc-500 flex items-center gap-3">
                <span className="text-emerald-400">&gt;</span> Try &quot;how
                does rate limiting work?&quot;
              </div>
            </div>
          </div>
        </div>

        {/* Block 3: Left Text, Right Interactive */}
        <div className="flex flex-col md:flex-row items-center gap-16">
          <div className="flex-1">
            <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
              IDE Agents
            </div>
            <h3 className="text-3xl md:text-4xl font-semibold text-white tracking-tight mb-5 leading-tight">
              From prompt to pull request
            </h3>
            <p className="text-[16px] text-zinc-400 leading-relaxed mb-8">
              Most AI-generated code needs cleanup. OmniContext is different:
              our deep contextual understanding of your codebase means the code
              they write is superior, and production-ready.
            </p>
            <ul className="flex flex-col gap-3">
              <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                Multi-step intelligent workflows
              </li>
              <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                Automatic cross-session memory
              </li>
              <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                Guaranteed context precision
              </li>
            </ul>
            <Link
              href="/docs/ide"
              className="inline-flex items-center gap-2 mt-8 text-[14px] text-emerald-500 hover:text-emerald-400 font-medium transition-colors"
            >
              Explore IDE extensions <ChevronRight size={14} />
            </Link>
          </div>
          <div className="flex-[1.2] w-full relative">
            {/* PR Mockup */}
            <div className="w-full bg-[#111] border border-white/5 rounded-[16px] shadow-2xl overflow-hidden font-sans">
              <div className="bg-[#18181b] p-3 text-[11px] text-zinc-400 flex items-center justify-between border-b border-white/5">
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-emerald-500" />{" "}
                  OmniContext Agent
                </div>
                <div>...</div>
              </div>
              <div className="p-5 flex flex-col gap-4">
                <div className="flex gap-3 items-start">
                  <div className="w-6 h-6 rounded-full bg-zinc-800 flex items-center justify-center text-[10px] text-zinc-300 shrink-0">
                    U
                  </div>
                  <div className="bg-zinc-800/50 p-3 rounded-xl rounded-tl-none border border-white/5 text-[12px] text-zinc-200">
                    Add rate limiting to the API endpoints
                  </div>
                </div>

                <div className="flex gap-3 items-start">
                  <div className="w-6 h-6 rounded-full bg-emerald-500/20 text-emerald-400 flex items-center justify-center text-[12px] shrink-0">
                    <Zap size={10} />
                  </div>
                  <div className="flex flex-col gap-2 w-full">
                    <div className="text-[12px] text-zinc-300 pt-1">
                      I&apos;ll add rate limiting to your API. Let me check the
                      existing middleware setup.
                    </div>
                    {/* Inner status boxes */}
                    <div className="mt-2 flex flex-col gap-1 w-[80%]">
                      <div className="bg-[#181818] border border-white/5 p-2 rounded flex items-center gap-2 text-[10px] text-zinc-500">
                        <ShieldCheck size={12} className="text-emerald-500" />
                        <span>Context Engine: Codebase</span>
                        <span className="bg-white/5 px-1 py-0.5 rounded ml-auto text-zinc-400 font-mono">
                          rate limit api
                        </span>
                      </div>
                      <div className="bg-[#181818] border border-emerald-500/20 p-2 rounded flex items-center gap-2 text-[10px] text-emerald-400">
                        <Terminal size={12} />
                        <span>
                          Creating{" "}
                          <span className="font-mono text-zinc-300">
                            src/middleware/rateLimit.rs
                          </span>
                        </span>
                        <span className="ml-auto">+42 -0</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="py-16 px-6 flex flex-col items-center text-center bg-black w-full border-t border-white/5">
        <div className="mb-8">
          <SearchCode className="text-zinc-700" size={32} strokeWidth={1.5} />
        </div>
        <div className="flex gap-8 mb-8 text-[13px] font-medium text-zinc-500">
          <Link href="/docs" className="hover:text-zinc-200 transition-colors">
            Docs
          </Link>
          <Link
            href="/pricing"
            className="hover:text-zinc-200 transition-colors"
          >
            Pricing
          </Link>
          <Link
            href="/privacy"
            className="hover:text-zinc-200 transition-colors"
          >
            Privacy Policy
          </Link>
          <Link href="/terms" className="hover:text-zinc-200 transition-colors">
            Terms of Use
          </Link>
        </div>
        <div className="flex items-center gap-6 mb-8">
          <a
            href="https://github.com/steeltroops-ai/omnicontext"
            className="text-zinc-600 hover:text-zinc-300 transition-colors"
          >
            <Github size={20} strokeWidth={1.5} />
          </a>
        </div>
        <p className="text-[12px] text-zinc-600">
          Copyright © 2026 OmniContext. All rights reserved. Built with Next.js
          and Bun.
        </p>
      </footer>
    </main>
  );
}
