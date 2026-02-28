"use client";

import { motion } from "framer-motion";
import {
  Terminal,
  Network,
  ShieldCheck,
  Github,
  SearchCode,
  ChevronRight,
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
              Documentation
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

      {/* Features Grid - Minimalist Apple Style */}
      <section className="py-[160px] px-6 w-full max-w-[1000px] mx-auto border-t border-white/5 relative">
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-1 h-32 bg-gradient-to-b from-white/10 to-transparent"></div>

        <div className="text-center mb-[120px]">
          <h2 className="text-4xl md:text-[56px] font-semibold tracking-tighter text-white mb-6 leading-tight">
            Inside the Omni Engine.
          </h2>
          <p className="text-[20px] text-zinc-400 max-w-[650px] mx-auto tracking-tight leading-snug">
            Engineered from first principles to be the fastest, most intelligent
            context layer for the next generation of autonomous development.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-12 md:gap-16 relative z-10">
          <div className="flex flex-col items-center text-center group">
            <div className="h-16 w-16 rounded-[18px] bg-gradient-to-b from-zinc-800 to-zinc-950 border border-white/10 flex items-center justify-center mb-8 shadow-lg group-hover:border-primary/40 group-hover:shadow-[0_0_30px_rgba(34,197,94,0.15)] transition-all duration-500">
              <Terminal
                size={24}
                className="text-zinc-300 group-hover:text-primary transition-colors duration-500"
                strokeWidth={1.5}
              />
            </div>
            <h3 className="text-[20px] font-semibold text-zinc-100 mb-3 tracking-tight">
              Blazing Fast Core
            </h3>
            <p className="text-[15px] text-zinc-500 leading-relaxed max-w-[280px]">
              Powered by Tree-sitter and a hyper-optimized SQLite vector
              pipeline. Index millions of lines locally in under three seconds.
            </p>
          </div>

          <div className="flex flex-col items-center text-center group">
            <div className="h-16 w-16 rounded-[18px] bg-gradient-to-b from-zinc-800 to-zinc-950 border border-white/10 flex items-center justify-center mb-8 shadow-lg group-hover:border-primary/40 group-hover:shadow-[0_0_30px_rgba(34,197,94,0.15)] transition-all duration-500">
              <Network
                size={24}
                className="text-zinc-300 group-hover:text-primary transition-colors duration-500"
                strokeWidth={1.5}
              />
            </div>
            <h3 className="text-[20px] font-semibold text-zinc-100 mb-3 tracking-tight">
              Dependency Fusion
            </h3>
            <p className="text-[15px] text-zinc-500 leading-relaxed max-w-[280px]">
              Maps your entire project graph—imports, extends, calls. Fuses
              semantic and relational signals via Reciprocal Rank Fusion.
            </p>
          </div>

          <div className="flex flex-col items-center text-center group">
            <div className="h-16 w-16 rounded-[18px] bg-gradient-to-b from-zinc-800 to-zinc-950 border border-white/10 flex items-center justify-center mb-8 shadow-lg group-hover:border-primary/40 group-hover:shadow-[0_0_30px_rgba(34,197,94,0.15)] transition-all duration-500">
              <ShieldCheck
                size={24}
                className="text-zinc-300 group-hover:text-primary transition-colors duration-500"
                strokeWidth={1.5}
              />
            </div>
            <h3 className="text-[20px] font-semibold text-zinc-100 mb-3 tracking-tight">
              Enterprise Standard
            </h3>
            <p className="text-[15px] text-zinc-500 leading-relaxed max-w-[280px]">
              Meters, API keys, commit lineage, and pattern recognition. Built
              to scale across your organization&apos;s entire custom agent
              fleet.
            </p>
          </div>
        </div>
      </section>

      {/* Deep Dive / Full Width Callout */}
      <section className="py-[160px] bg-[#050505] border-y border-white/5 relative overflow-hidden flex flex-col items-center justify-center text-center">
        {/* Subtle radial gradient */}
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(34,197,94,0.03)_0%,transparent_60%)] pointer-events-none"></div>

        <div className="max-w-[700px] mx-auto px-6 relative z-10">
          <Terminal
            size={40}
            className="mx-auto mb-8 text-primary/80"
            strokeWidth={1}
          />
          <h2 className="text-4xl md:text-[56px] font-semibold tracking-tighter text-white mb-8 leading-[1.1]">
            Intelligence,
            <br />
            decentralized.
          </h2>
          <p className="text-[20px] text-zinc-400 mb-12 leading-snug tracking-tight">
            No massive cloud uploads. No privacy leaks. All parsing, chunking,
            and vector embedding runs natively on your machine without ever
            leaving the host. Keep your codebase yours.
          </p>

          <Link
            href="/docs/architecture"
            className="inline-flex items-center gap-2 text-[15px] text-zinc-300 hover:text-white transition-colors pb-1 border-b border-zinc-700 hover:border-white"
          >
            Read the Architecture Whitepaper <ChevronRight size={14} />
          </Link>
        </div>
      </section>

      {/* Footer */}
      <footer className="py-16 px-6 flex flex-col items-center text-center bg-black w-full border-t border-white/5">
        <div className="mb-8">
          <SearchCode className="text-zinc-700" size={32} strokeWidth={1.5} />
        </div>
        <div className="flex gap-8 mb-8 text-[13px] font-medium text-zinc-500">
          <Link href="/docs" className="hover:text-zinc-200 transition-colors">
            Documentation
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
