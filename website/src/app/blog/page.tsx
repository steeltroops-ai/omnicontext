"use client";

import Link from "next/link";
import { ChevronRight } from "lucide-react";
import { siteConfig } from "@/config/site";
import { SiteNav } from "@/components/site-nav";
import { SiteFooterMini } from "@/components/site-footer-mini";

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
    <div className="flex flex-col min-h-screen bg-[#09090B] selection:bg-primary/30">
      <SiteNav />

      <main className="flex-1 flex flex-col pt-24 sm:pt-28">
        <div className="w-full max-w-[1400px] mx-auto px-6 sm:px-8 md:px-16 pb-20">
          <h1 className="text-4xl sm:text-5xl md:text-[56px] font-semibold text-white tracking-tighter mb-4 leading-tight">
            Blog
          </h1>
          <p className="text-[16px] sm:text-[18px] text-zinc-400 max-w-[560px] tracking-tight leading-snug mb-16">
            Engineering insights, architecture decisions, and release notes from
            the {siteConfig.name} team.
          </p>

          <div className="flex flex-col gap-0 border-t border-white/5">
            {posts.map((post, i) => (
              <Link
                key={i}
                href={post.href}
                className="flex flex-col sm:flex-row sm:items-center gap-4 sm:gap-12 py-8 sm:py-10 border-b border-white/5 group transition-colors hover:bg-white/[0.01] px-2 -mx-2 rounded-lg"
              >
                <div className="shrink-0 sm:w-[140px]">
                  <div className="text-[12px] text-zinc-600 tracking-tight">
                    {post.date}
                  </div>
                  <div className="text-[10px] uppercase tracking-widest text-emerald-500 font-semibold mt-1">
                    {post.tag}
                  </div>
                </div>
                <div className="flex-1 min-w-0">
                  <h2 className="text-[17px] sm:text-[20px] font-semibold text-zinc-100 tracking-tight mb-2 group-hover:text-white transition-colors flex items-center gap-2">
                    <span>{post.title}</span>
                    <ChevronRight
                      size={16}
                      className="text-zinc-600 group-hover:text-emerald-500 group-hover:translate-x-1 transition-all duration-300 shrink-0"
                    />
                  </h2>
                  <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight line-clamp-2">
                    {post.description}
                  </p>
                </div>
              </Link>
            ))}
          </div>
        </div>

        <SiteFooterMini />
      </main>
    </div>
  );
}
