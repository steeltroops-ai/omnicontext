"use client";

import React from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Search, Github } from "lucide-react";
import { siteConfig } from "@/config/site";
import { Logo } from "@/components/icons";

export default function DocsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const pathname = usePathname();

  const isLinkActive = (path: string) => {
    return pathname === path
      ? "bg-zinc-900 text-zinc-100 font-medium"
      : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/30 font-normal";
  };

  return (
    <div className="flex flex-col md:flex-row h-screen bg-[#09090B] text-zinc-100 font-sans overflow-hidden selection:bg-primary/30">
      {/* Left Sidebar */}
      <aside className="w-[280px] shrink-0 border-r border-white/5 flex flex-col bg-[#0E0E11] overflow-y-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
        <div className="h-14 flex items-center px-8 font-semibold text-[14px] text-zinc-100 border-b border-white/5 shrink-0 bg-[#09090B]/50 backdrop-blur-xl sticky top-0 z-10">
          <Link
            href="/"
            className="flex items-center gap-2.5 hover:opacity-80 transition-opacity"
          >
            <Logo
              className="text-primary"
              size={siteConfig.branding.sizes.header}
            />
            <span>{siteConfig.name}</span>
          </Link>
        </div>

        <div className="p-6 flex-1">
          <div className="mb-10">
            <div className="text-[11px] uppercase tracking-widest text-zinc-600 font-semibold mb-4 px-2">
              Getting Started
            </div>
            <Link
              href="/docs"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs")}`}
            >
              Introduction
            </Link>
            <Link
              href="/docs/quickstart"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/quickstart")}`}
            >
              Quickstart
            </Link>
            <Link
              href="/docs/installation"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/installation")}`}
            >
              Installation
            </Link>
          </div>

          <div className="mb-10">
            <div className="text-[11px] uppercase tracking-widest text-zinc-600 font-semibold mb-4 px-2">
              Core Concepts
            </div>
            <Link
              href="/docs/context-engine"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/context-engine")}`}
            >
              Context Engine
            </Link>
            <Link
              href="/docs/indexing"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/indexing")}`}
            >
              Indexing Pipeline
            </Link>
            <Link
              href="/docs/search"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/search")}`}
            >
              Hybrid Search
            </Link>
            <Link
              href="/docs/dependency-graph"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/dependency-graph")}`}
            >
              Dependency Graph
            </Link>
          </div>

          <div className="mb-10">
            <div className="text-[11px] uppercase tracking-widest text-zinc-600 font-semibold mb-4 px-2">
              MCP Integration
            </div>
            <Link
              href="/docs/mcp-server"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/mcp-server")}`}
            >
              MCP Server
            </Link>
            <Link
              href="/docs/mcp-tools"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/mcp-tools")}`}
            >
              Available Tools
            </Link>
            <Link
              href="/docs/mcp-transports"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/mcp-transports")}`}
            >
              Transports (stdio / SSE)
            </Link>
          </div>

          <div className="mb-10">
            <div className="text-[11px] uppercase tracking-widest text-zinc-600 font-semibold mb-4 px-2">
              Architecture
            </div>
            <Link
              href="/docs/architecture"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/architecture")}`}
            >
              System Overview
            </Link>
            <Link
              href="/docs/supported-languages"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/supported-languages")}`}
            >
              Supported Languages
            </Link>
            <Link
              href="/docs/configuration"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/configuration")}`}
            >
              Configuration
            </Link>
          </div>

          <div className="mb-10">
            <div className="text-[11px] uppercase tracking-widest text-zinc-600 font-semibold mb-4 px-2">
              Enterprise
            </div>
            <Link
              href="/docs/pricing"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/pricing")}`}
            >
              Pricing Tiers
            </Link>
            <Link
              href="/docs/rest-api"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/rest-api")}`}
            >
              REST API
            </Link>
            <Link
              href="/docs/security"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/security")}`}
            >
              Security &amp; Auth
            </Link>
          </div>
        </div>

        {/* Sidebar Footer */}
        <div className="p-6 border-t border-white/5 mt-auto">
          <a
            href={siteConfig.links.github}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 text-[12px] text-zinc-500 hover:text-zinc-300 transition-colors"
          >
            <Github size={14} /> View on GitHub
          </a>
        </div>
      </aside>

      {/* Main Wrapper */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden bg-[#09090B]">
        {/* Topbar */}
        <header className="h-14 shrink-0 flex items-center justify-between px-8 border-b border-white/5 bg-[#09090B]/70 backdrop-blur-xl z-20">
          <div className="flex items-center bg-[#141418] border border-white/5 px-3 h-8 rounded-full w-[360px] gap-2 text-zinc-500 cursor-pointer transition-all duration-300 hover:bg-[#111] hover:border-white/10 group">
            <Search
              size={14}
              className="text-zinc-600 group-hover:text-zinc-400 transition-colors"
            />
            <span className="flex-1 text-[13px] font-medium tracking-tight">
              Search documentation...
            </span>
            <span className="font-mono text-[10px] bg-white/5 px-2 py-0.5 rounded text-zinc-500/80">
              Ctrl K
            </span>
          </div>

          <div className="flex items-center gap-8">
            <Link
              href="/"
              className="text-[13px] font-medium text-zinc-500 hover:text-zinc-200 transition-colors duration-200 tracking-tight"
            >
              Home
            </Link>
            <Link
              href="/blog"
              className="text-[13px] font-medium text-zinc-500 hover:text-zinc-200 transition-colors duration-200 tracking-tight"
            >
              Blog
            </Link>
            <a
              href={siteConfig.links.github}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[13px] font-medium text-zinc-500 hover:text-zinc-200 transition-colors duration-200 tracking-tight"
            >
              GitHub
            </a>
          </div>
        </header>

        {/* Content Area */}
        <div className="flex-1 overflow-hidden">{children}</div>
      </div>
    </div>
  );
}
