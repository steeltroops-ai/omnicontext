"use client";

import React from "react";
import { Code, Cpu, ChevronRight, Zap, FileCode2, Command } from "lucide-react";
import Link from "next/link";

export default function DocsPage() {
  return (
    <div className="flex-1 flex overflow-hidden">
      {/* Article Content */}
      <div className="flex-1 overflow-y-auto px-16 py-12 flex justify-center">
        <article className="max-w-[800px] w-full">
          <div className="text-sm text-muted-foreground mb-4">
            Getting Started
          </div>

          <h1 className="text-4xl font-bold text-white mb-6 tracking-tight">
            Introduction
          </h1>
          <p className="text-[1.1rem] text-muted-foreground leading-relaxed mb-10">
            OmniContext is the developer AI platform that helps you understand
            code, debug issues, and ship faster because it understands your
            codebase. Use Agent, Next Edit, and Code Completions to get more
            done.
          </p>

          {/* IDE Mockup */}
          <div className="w-full bg-[#111] border border-border rounded-xl aspect-video mb-12 flex items-center justify-center text-[#555] font-mono shadow-[0_10px_30px_rgba(0,0,0,0.5)] overflow-hidden">
            {/* Realistically styled mockup matching the reference image's Augment code view */}
            <div className="w-full h-full flex flex-col">
              {/* Header */}
              <div className="bg-[#18181b] px-4 py-2 flex items-center gap-2 border-b border-[#27272a] text-[13px] text-zinc-400">
                <FileCode2 size={14} /> <span>ringbuffer.test.ts</span>
                <span className="flex-1"></span>
                <span className="bg-[#27272a] px-2 py-0.5 rounded text-white">
                  OMNICONTEXT CHAT
                </span>
              </div>

              {/* Body */}
              <div className="flex-1 flex bg-[#0a0a0c]">
                {/* Editor Left */}
                <div className="flex-[2] p-4 border-r border-[#27272a] relative">
                  <div className="absolute top-8 left-4 bg-[rgba(0,208,107,0.15)] px-1.5 py-0.5 rounded text-primary text-xs font-bold">
                    Tab to accept suggestion
                  </div>
                  <pre className="text-[#8b949e] text-xs mt-12 leading-relaxed">
                    <span className="text-zinc-400">193</span>{" "}
                    expect(buffer.removeItem(2)).toBe(
                    <span className="text-[#79c0ff]">true</span>);{"\n"}
                    <span className="text-zinc-400">194</span>{" "}
                    expect(buffer.length).toBe(
                    <span className="text-[#a5d6ff]">2</span>);{"\n"}
                    <span className="text-zinc-400">195</span>{" "}
                    expect(buffer.slice()).toEqual([1, 3]);{"\n"}
                    <span className="text-zinc-400">196</span>
                    {"\n"}
                    <span className="text-zinc-400">198</span> test(&quot;remove
                    non-existent item&quot;, () =&gt; {"{"}
                    {"\n"}
                    <span className="text-zinc-400">199</span>{" "}
                    <span className="text-[#ff7b72]">const</span> buffer ={" "}
                    <span className="text-[#ff7b72]">new</span> RingBuffer(
                    <span className="text-[#a5d6ff]">5</span>);{"\n"}
                    <span className="text-zinc-400">200</span> buffer.addItem(
                    <span className="text-[#a5d6ff]">1</span>);{"\n"}
                    <span className="text-zinc-400">201</span> buffer.addItem(
                    <span className="text-[#a5d6ff]">2</span>);{"\n"}
                    <span className="text-zinc-400">202</span> {"\n"}
                    <span className="text-zinc-400">203</span>{" "}
                    expect(buffer.removeItem(5)).toBe(
                    <span className="text-[#79c0ff]">false</span>);{"\n"}
                    <span className="text-zinc-400">204</span>{" "}
                    expect(buffer.length).toBe(
                    <span className="text-[#a5d6ff]">2</span>);{"\n"}
                  </pre>
                </div>

                {/* Chat Right */}
                <div className="flex-1 p-4 flex flex-col gap-4">
                  <div className="self-end bg-[#27272a] p-3 rounded-lg text-white text-[13px]">
                    You
                    <br />
                    <span className="text-zinc-400 mt-1 block">
                      Add a function to remove a specific item from the
                      RingBuffer.
                    </span>
                  </div>
                  <div className="self-start bg-[#09090b] p-3 rounded-lg border border-[#27272a] text-white text-[13px] w-full">
                    <div className="bg-primary text-black inline-block px-2 py-0.5 rounded text-[11px] font-bold mb-2">
                      OmniContext
                    </div>
                    <p className="text-zinc-300 mb-2">
                      Certainly! I&apos;ll add a function to remove a specific
                      item from the RingBuffer...
                    </p>
                    <div className="bg-[#18181b] p-2 rounded text-[#ff7b72]">
                      <span className="text-zinc-400">{`// implementation`}</span>
                    </div>
                    <div className="mt-4 bg-[#1c1c1c] p-2 rounded text-[11px]">
                      <span className="text-zinc-400">ACTIVE CONTEXT</span>
                      <br />
                      <span className="text-primary">@ Repository</span>{" "}
                      source/omnicontext/ringbuffer
                      <br />
                      <span className="text-primary">@ Current File</span>{" "}
                      ringbuffer.test.ts
                      <br />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <h2 className="text-2xl text-white mt-10 mb-4 font-semibold">
            Get started in minutes
          </h2>
          <p className="text-zinc-300 leading-relaxed mb-6">
            OmniContext works with your favorite IDE and your favorite
            programming language. Download the extension, sign in, and get
            coding.
          </p>

          {/* Ask Input */}
          <div className="flex items-center bg-[#111] border border-[#333] px-4 py-3 rounded-lg mb-8 transition-colors duration-200 focus-within:border-primary">
            <input
              type="text"
              placeholder="Ask a question..."
              className="flex-1 bg-transparent border-none text-white text-[0.95rem] outline-none placeholder:text-[#666]"
            />
            <span className="text-[#555] text-xs flex items-center gap-1 font-mono">
              Ctrl+I <Command size={14} />
            </span>
          </div>

          {/* 3 Grid Cards */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-12">
            <div className="bg-white/5 border border-border p-6 rounded-lg transition-all duration-200 hover:bg-white/10 hover:border-white/15 cursor-pointer flex flex-col h-full">
              <div className="text-base font-semibold text-white mb-2">
                Visual Studio Code
              </div>
              <p className="text-sm text-muted-foreground leading-relaxed m-0 flex-1">
                Get completions, chat, and instructions in your favorite open
                source editor.
              </p>
            </div>
            <div className="bg-white/5 border border-border p-6 rounded-lg transition-all duration-200 hover:bg-white/10 hover:border-white/15 cursor-pointer flex flex-col h-full">
              <div className="text-base font-semibold text-white mb-2">
                JetBrains IDEs
              </div>
              <p className="text-sm text-muted-foreground leading-relaxed m-0 flex-1">
                Completions are available for all JetBrains IDEs, like WebStorm,
                PyCharm, and IntelliJ.
              </p>
            </div>
            <div className="bg-white/5 border border-border p-6 rounded-lg transition-all duration-200 hover:bg-white/10 hover:border-white/15 cursor-pointer flex flex-col h-full">
              <div className="text-base font-semibold text-white mb-2">
                Omni CLI
              </div>
              <p className="text-sm text-muted-foreground leading-relaxed m-0 flex-1">
                All the power of OmniContext&apos;s agent, context engine, and
                tools in your terminal.
              </p>
            </div>
          </div>

          <h2 className="text-2xl text-white mt-10 mb-4 font-semibold">
            Learn more
          </h2>
          <p className="text-zinc-300 leading-relaxed mb-6">
            Get up to speed, stay in the flow, and get more done. Chat, Next
            Edit, and Code Completions will change the way you build software.
          </p>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="bg-white/5 border border-border p-6 rounded-lg transition-all duration-200 hover:bg-white/10 hover:border-white/15 cursor-pointer flex flex-col h-full">
              <Cpu className="mb-4 text-white opacity-80" size={24} />
              <div className="text-base font-semibold text-white mb-2">
                Agent
              </div>
              <p className="text-sm text-muted-foreground leading-relaxed m-0 flex-1">
                Autonomous coding with OmniContext&apos;s context engine and
                tools can tackle tasks big and small.
              </p>
            </div>
            <div className="bg-white/5 border border-border p-6 rounded-lg transition-all duration-200 hover:bg-white/10 hover:border-white/15 cursor-pointer flex flex-col h-full">
              <Zap className="mb-4 text-white opacity-80" size={24} />
              <div className="text-base font-semibold text-white mb-2">
                Next Edit
              </div>
              <p className="text-sm text-muted-foreground leading-relaxed m-0 flex-1">
                Keep moving through your tasks by guiding you step-by-step
                through complex or repetitive changes.
              </p>
            </div>
            <div className="bg-white/5 border border-border p-6 rounded-lg transition-all duration-200 hover:bg-white/10 hover:border-white/15 cursor-pointer flex flex-col h-full">
              <Code className="mb-4 text-white opacity-80" size={24} />
              <div className="text-base font-semibold text-white mb-2">
                Code Completions
              </div>
              <p className="text-sm text-muted-foreground leading-relaxed m-0 flex-1">
                Intelligent code suggestions that knows your codebase right at
                your fingertips.
              </p>
            </div>
          </div>

          <div className="flex justify-end mt-8">
            <Link href="/docs/quickstart">
              <span className="text-sm transition-colors duration-100 flex items-center gap-2 font-semibold text-white hover:text-primary">
                Quickstart <ChevronRight size={16} />
              </span>
            </Link>
          </div>
        </article>
      </div>

      {/* Right TOC Sidebar */}
      <aside className="w-[250px] shrink-0 p-12 overflow-y-auto border-l border-border hidden xl:block">
        <div className="text-sm font-semibold text-white mb-4 flex items-center gap-2">
          On this page
        </div>
        <ul className="list-none m-0 p-0 text-sm">
          <li className="mb-2.5">
            <a href="#" className="text-primary transition-colors duration-100">
              Get started in minutes
            </a>
          </li>
          <li className="mb-2.5">
            <a
              href="#"
              className="text-muted-foreground hover:text-white transition-colors duration-100"
            >
              Visual Studio Code
            </a>
          </li>
          <li className="mb-2.5">
            <a
              href="#"
              className="text-muted-foreground hover:text-white transition-colors duration-100"
            >
              JetBrains IDEs
            </a>
          </li>
          <li className="mb-2.5">
            <a
              href="#"
              className="text-muted-foreground hover:text-white transition-colors duration-100"
            >
              Omni CLI
            </a>
          </li>
          <li className="mb-2.5">
            <a
              href="#"
              className="text-muted-foreground hover:text-white transition-colors duration-100"
            >
              Learn more
            </a>
          </li>
        </ul>
      </aside>
    </div>
  );
}
