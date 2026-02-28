"use client";

import React from "react";
import { Code, Cpu, ChevronRight, Zap, FileCode2, Command } from "lucide-react";
import Link from "next/link";

export default function DocsPage() {
  return (
    <div className="flex-1 flex overflow-hidden h-full">
      {/* Article Content */}
      <div className="flex-1 overflow-y-auto px-10 md:px-20 py-16 flex justify-center bg-[#000]">
        <article className="max-w-[760px] w-full">
          <div className="text-[12px] font-semibold tracking-wider uppercase text-zinc-600 mb-6">
            Getting Started
          </div>

          <h1 className="text-4xl md:text-5xl font-semibold text-white tracking-tighter mb-6 leading-tight">
            Introduction
          </h1>
          <p className="text-[18px] text-zinc-400 leading-[1.6] mb-14 tracking-tight">
            OmniContext is the developer AI platform that helps you understand
            code, debug issues, and ship faster because it understands your
            entire architecture. Start exploring the graph with our built-in
            agents and precise retrieval.
          </p>

          {/* IDE Mockup - Ultra Minimal */}
          <div className="w-full bg-[#050505] border border-white/5 rounded-2xl aspect-video mb-16 flex items-center justify-center text-[#555] font-mono shadow-[0_20px_80px_rgba(0,0,0,0.8)] overflow-hidden">
            <div className="w-full h-full flex flex-col">
              {/* Header */}
              <div className="bg-[#0a0a0a] px-5 py-2.5 flex items-center gap-2 border-b border-white/5 text-[12px] text-zinc-500 tracking-wide font-sans">
                <FileCode2 size={13} className="text-zinc-600" />{" "}
                <span>ringbuffer.test.ts</span>
                <span className="flex-1"></span>
                <span className="bg-zinc-900 border border-white/5 px-2.5 py-1 rounded text-zinc-300 font-medium">
                  OMNICONTEXT MCP CLIENT
                </span>
              </div>

              {/* Body */}
              <div className="flex-1 flex bg-[#000]">
                {/* Editor Left */}
                <div className="flex-[2] p-6 border-r border-white/5 relative">
                  <div className="absolute top-8 left-6 bg-emerald-500/10 border border-emerald-500/20 px-2 py-1 rounded text-emerald-400 text-[11px] font-semibold tracking-wide">
                    Tab to accept suggestion
                  </div>
                  <pre className="text-zinc-500 text-[12px] mt-16 leading-[1.8] tracking-tight">
                    <span className="text-zinc-700">193</span>{" "}
                    expect(buffer.removeItem(2)).toBe(
                    <span className="text-emerald-400">true</span>);{"\n"}
                    <span className="text-zinc-700">194</span>{" "}
                    expect(buffer.length).toBe(
                    <span className="text-zinc-300">2</span>);{"\n"}
                    <span className="text-zinc-700">195</span>{" "}
                    expect(buffer.slice()).toEqual([1, 3]);{"\n"}
                    <span className="text-zinc-700">196</span>
                    {"\n"}
                    <span className="text-zinc-700">198</span> test(&quot;remove
                    non-existent item&quot;, () =&gt; {"{"}
                    {"\n"}
                    <span className="text-zinc-700">199</span>{" "}
                    <span className="text-zinc-100">const</span> buffer ={" "}
                    <span className="text-zinc-100">new</span> RingBuffer(
                    <span className="text-zinc-300">5</span>);{"\n"}
                    <span className="text-zinc-700">200</span> buffer.addItem(
                    <span className="text-zinc-300">1</span>);{"\n"}
                    <span className="text-zinc-700">201</span> buffer.addItem(
                    <span className="text-zinc-300">2</span>);{"\n"}
                    <span className="text-zinc-700">202</span> {"\n"}
                    <span className="text-zinc-700">203</span>{" "}
                    expect(buffer.removeItem(5)).toBe(
                    <span className="text-emerald-400">false</span>);{"\n"}
                    <span className="text-zinc-700">204</span>{" "}
                    expect(buffer.length).toBe(
                    <span className="text-zinc-300">2</span>);{"\n"}
                  </pre>
                </div>

                {/* Chat Right */}
                <div className="flex-[1.5] p-6 flex flex-col gap-6 font-sans">
                  <div className="self-end bg-zinc-900 border border-white/5 px-4 py-3 rounded-[14px] text-zinc-100 text-[13px] tracking-tight">
                    <span className="text-zinc-400 mb-1 block text-[11px] font-medium tracking-wide">
                      YOU
                    </span>
                    Add a function to remove a specific item from the
                    RingBuffer.
                  </div>
                  <div className="self-start bg-[#050505] px-4 py-3 rounded-[14px] border border-white/5 text-zinc-300 text-[13px] tracking-tight w-full">
                    <div className="bg-zinc-100 text-black px-2 py-0.5 rounded text-[10px] font-bold mb-3 inline-block">
                      OMNICONTEXT
                    </div>
                    <p className="text-zinc-400 mb-4 leading-relaxed tracking-tight">
                      Certainly! I&apos;ll add a function to remove a specific
                      item from the RingBuffer.
                    </p>
                    <div className="bg-[#0a0a0a] border border-white/5 p-3 rounded-[10px] text-zinc-500 font-mono text-[11px]">
                      {`// implementation`}
                    </div>
                    <div className="mt-4 bg-[#0a0a0a] border border-white/5 p-3 rounded-[10px] text-[11px] tracking-wide font-medium">
                      <span className="text-zinc-600 mb-2 block text-[10px] uppercase">
                        Active Graph Context
                      </span>
                      <span className="text-emerald-500 opacity-80">
                        @ Repository
                      </span>{" "}
                      source/omnicontext
                      <br />
                      <span className="text-emerald-500 opacity-80">
                        @ Current File
                      </span>{" "}
                      ringbuffer.test.ts
                      <br />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <h2 className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight">
            Get started in minutes
          </h2>
          <p className="text-[16px] text-zinc-400 leading-relaxed mb-8 tracking-tight">
            OmniContext works with your favorite IDE and your favorite
            programming language. Download the extension, authenticate your API
            key, and get coding.
          </p>

          {/* Ask Input */}
          <div className="flex items-center bg-[#050505] border border-white/10 px-5 py-3.5 rounded-2xl mb-10 transition-colors duration-300 focus-within:border-emerald-500/50 shadow-sm focus-within:shadow-[0_0_20px_rgba(16,185,129,0.1)]">
            <input
              type="text"
              placeholder="Ask the documentation..."
              className="flex-1 bg-transparent border-none text-white text-[15px] outline-none placeholder:text-zinc-600/80 font-medium tracking-tight"
            />
            <span className="text-zinc-500 text-[11px] flex items-center gap-1.5 font-medium tracking-wide bg-white/5 px-2 py-1 rounded">
              Ctrl+I <Command size={12} />
            </span>
          </div>

          {/* 3 Grid Cards */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5 mb-16">
            <div className="bg-[#050505] border border-white/5 p-6 rounded-[20px] transition-all duration-300 hover:bg-[#0a0a0a] hover:border-white/10 group cursor-pointer flex flex-col">
              <div className="text-[16px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-white transition-colors">
                Visual Studio Code
              </div>
              <p className="text-[14px] text-zinc-500 leading-[1.6] m-0 flex-1 tracking-tight">
                Get completions, chat, and graph analysis natively inside VS
                Code.
              </p>
            </div>
            <div className="bg-[#050505] border border-white/5 p-6 rounded-[20px] transition-all duration-300 hover:bg-[#0a0a0a] hover:border-white/10 group cursor-pointer flex flex-col">
              <div className="text-[16px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-white transition-colors">
                JetBrains IDEs
              </div>
              <p className="text-[14px] text-zinc-500 leading-[1.6] m-0 flex-1 tracking-tight">
                Full integration for all JetBrains IDEs, like WebStorm, PyCharm,
                and IntelliJ.
              </p>
            </div>
            <div className="bg-[#050505] border border-white/5 p-6 rounded-[20px] transition-all duration-300 hover:bg-[#0a0a0a] hover:border-white/10 group cursor-pointer flex flex-col">
              <div className="text-[16px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-white transition-colors">
                Omni CLI
              </div>
              <p className="text-[14px] text-zinc-500 leading-[1.6] m-0 flex-1 tracking-tight">
                Access the power of the blazing-fast context engine directly in
                your terminal.
              </p>
            </div>
          </div>

          <h2 className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight">
            Learn more
          </h2>
          <p className="text-[16px] text-zinc-400 leading-relaxed mb-8 tracking-tight">
            Discover the tools powering OmniContext. Leverage advanced
            Reciprocal Rank Fusion, Next Edit predictions, and enterprise-grade
            rate-limiting.
          </p>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
            <div className="bg-[#050505] border border-white/5 p-6 rounded-[20px] transition-all duration-300 hover:bg-[#0a0a0a] hover:border-emerald-500/30 group cursor-pointer flex flex-col">
              <div className="w-10 h-10 rounded-full bg-white/5 border border-white/5 flex items-center justify-center mb-5 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-400 transition-colors">
                <Cpu size={18} strokeWidth={1.5} />
              </div>
              <div className="text-[16px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-emerald-400 transition-colors">
                Graph Engine
              </div>
              <p className="text-[14px] text-zinc-500 leading-[1.6] m-0 flex-1 tracking-tight">
                Highly parallel indexer using Tree-sitter and SQLite to map
                every single import.
              </p>
            </div>
            <div className="bg-[#050505] border border-white/5 p-6 rounded-[20px] transition-all duration-300 hover:bg-[#0a0a0a] hover:border-emerald-500/30 group cursor-pointer flex flex-col">
              <div className="w-10 h-10 rounded-full bg-white/5 border border-white/5 flex items-center justify-center mb-5 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-400 transition-colors">
                <Zap size={18} strokeWidth={1.5} />
              </div>
              <div className="text-[16px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-emerald-400 transition-colors">
                Intelligent APIs
              </div>
              <p className="text-[14px] text-zinc-500 leading-[1.6] m-0 flex-1 tracking-tight">
                Integrations with the Model Context Protocol (MCP) and REST to
                feed context reliably.
              </p>
            </div>
            <div className="bg-[#050505] border border-white/5 p-6 rounded-[20px] transition-all duration-300 hover:bg-[#0a0a0a] hover:border-emerald-500/30 group cursor-pointer flex flex-col">
              <div className="w-10 h-10 rounded-full bg-white/5 border border-white/5 flex items-center justify-center mb-5 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-400 transition-colors">
                <Code size={18} strokeWidth={1.5} />
              </div>
              <div className="text-[16px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-emerald-400 transition-colors">
                Enterprise Ready
              </div>
              <p className="text-[14px] text-zinc-500 leading-[1.6] m-0 flex-1 tracking-tight">
                Built-in authorization guards, usage metering, and API key
                limits right from `omni-core`.
              </p>
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

      {/* Right TOC Sidebar - Ultra Minimal */}
      <aside className="w-[240px] shrink-0 p-10 overflow-y-auto border-l border-white/5 hidden xl:block bg-[#000]">
        <div className="text-[12px] font-semibold uppercase tracking-wider text-zinc-600 mb-6">
          On this page
        </div>
        <nav className="flex flex-col gap-4 text-[13px] tracking-tight">
          <a
            href="#"
            className="text-zinc-200 font-medium hover:text-white transition-colors duration-200"
          >
            Get started
          </a>
          <a
            href="#"
            className="text-zinc-500 hover:text-zinc-300 transition-colors duration-200"
          >
            Visual Studio Code
          </a>
          <a
            href="#"
            className="text-zinc-500 hover:text-zinc-300 transition-colors duration-200"
          >
            JetBrains IDEs
          </a>
          <a
            href="#"
            className="text-zinc-500 hover:text-zinc-300 transition-colors duration-200"
          >
            Omni CLI
          </a>
          <a
            href="#"
            className="text-zinc-500 hover:text-zinc-300 transition-colors duration-200"
          >
            Learn more
          </a>
        </nav>
      </aside>
    </div>
  );
}
