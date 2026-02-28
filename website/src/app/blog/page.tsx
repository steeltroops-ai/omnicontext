"use client";

import Link from "next/link";
import { ChevronRight, Github, ArrowUpRight } from "lucide-react";
import { siteConfig } from "@/config/site";
import { Logo } from "@/components/icons";

const posts = [
  {
    date: "Feb 28, 2026",
    tag: "ANNOUNCEMENT",
    title: `Introducing ${siteConfig.name} v0.1.0`,
    description: `The first public release of ${siteConfig.name}: a high-performance, local code context engine written in Rust. Index your codebase, build a dependency graph, and serve rich context to any MCP-compatible AI agent.`,
    href: "/blog/introducing-omnicontext",
  },
  {
    date: "Feb 28, 2026",
    tag: "ENGINEERING",
    title: "Why we chose Reciprocal Rank Fusion for code search",
    description: `A deep dive into how ${siteConfig.name} blends semantic embeddings with BM25 keyword search using RRF to deliver precise, context-aware retrieval that outperforms either method alone.`,
    href: "/blog/reciprocal-rank-fusion",
  },
  {
    date: "Feb 28, 2026",
    tag: "ARCHITECTURE",
    title: "AST-aware chunking: why code is not text",
    description: `Traditional chunking strategies designed for prose fail catastrophically on source code. Here is how ${siteConfig.name} uses Tree-sitter ASTs to chunk at structural boundaries.`,
    href: "/blog/ast-aware-chunking",
  },
];

export default function BlogPage() {
  return (
    <main className="flex flex-col min-h-screen bg-[#09090B] font-sans selection:bg-primary/30">
      {/* Navigation */}
      <nav className="fixed top-0 w-full h-14 flex items-center justify-center z-50 border-b border-white/5 bg-[#09090B]/50 backdrop-blur-xl">
        <div className="flex items-center justify-between w-full max-w-[1200px] px-8 md:px-16">
          <Link
            href="/"
            className="flex items-center gap-2 font-semibold text-sm text-zinc-100 transition-opacity hover:opacity-80"
          >
            <Logo className="text-primary" size={18} />
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
              className="text-[13px] font-medium text-zinc-100 transition-colors"
            >
              Blog
            </Link>
            <Link
              href="/enterprise"
              className="text-[13px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors"
            >
              Enterprise
            </Link>
            <a
              href={siteConfig.links.github}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[13px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors flex items-center gap-1"
            >
              GitHub <ArrowUpRight size={12} />
            </a>
          </div>
        </div>
      </nav>

      <section className="pt-[140px] pb-[100px] px-8 md:px-16 max-w-[1200px] mx-auto w-full">
        <h1 className="text-4xl md:text-[56px] font-semibold text-white tracking-tighter mb-6 leading-tight">
          Blog
        </h1>
        <p className="text-[18px] text-zinc-400 max-w-[600px] tracking-tight leading-snug mb-20">
          Engineering insights, architecture decisions, and release notes from
          the OmniContext team.
        </p>

        <div className="flex flex-col gap-0 border-t border-white/5">
          {posts.map((post, i) => (
            <Link
              key={i}
              href={post.href}
              className="flex flex-col md:flex-row md:items-center gap-4 md:gap-12 py-10 border-b border-white/5 group transition-colors hover:bg-white/[0.01] px-2 -mx-2 rounded-lg"
            >
              <div className="shrink-0 w-[140px]">
                <div className="text-[12px] text-zinc-600 tracking-tight">
                  {post.date}
                </div>
                <div className="text-[10px] uppercase tracking-widest text-emerald-500 font-semibold mt-1">
                  {post.tag}
                </div>
              </div>
              <div className="flex-1 min-w-0">
                <h2 className="text-[20px] font-semibold text-zinc-100 tracking-tight mb-2 group-hover:text-white transition-colors flex items-center gap-2">
                  {post.title}
                  <ChevronRight
                    size={16}
                    className="text-zinc-600 group-hover:text-emerald-500 group-hover:translate-x-1 transition-all duration-300"
                  />
                </h2>
                <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight line-clamp-2">
                  {post.description}
                </p>
              </div>
            </Link>
          ))}
        </div>
      </section>

      {/* Footer */}
      <footer className="py-12 px-8 md:px-16 border-t border-white/5 bg-[#09090B] mt-auto">
        <div className="max-w-[1200px] mx-auto flex flex-col md:flex-row items-center justify-between gap-6">
          <div className="flex items-center gap-6 text-[13px] text-zinc-500">
            <Link
              href="/docs"
              className="hover:text-zinc-200 transition-colors"
            >
              Docs
            </Link>
            <Link
              href="/blog"
              className="hover:text-zinc-200 transition-colors"
            >
              Blog
            </Link>
            <Link
              href="/enterprise"
              className="hover:text-zinc-200 transition-colors"
            >
              Enterprise
            </Link>
            <Link
              href="/support"
              className="hover:text-zinc-200 transition-colors"
            >
              Support
            </Link>
          </div>
          <div className="flex items-center gap-4">
            <a
              href={siteConfig.links.github}
              className="text-zinc-600 hover:text-zinc-300 transition-colors"
            >
              <Github size={18} />
            </a>
            <span className="text-[12px] text-zinc-600">
              (c) 2026 {siteConfig.name}. Apache-2.0.
            </span>
          </div>
        </div>
      </footer>
    </main>
  );
}
