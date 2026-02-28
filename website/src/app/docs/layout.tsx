"use client";

import React from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Search, Moon, Zap, Layers } from "lucide-react";

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
    <div className="flex flex-col md:flex-row h-screen bg-[#000] text-zinc-100 font-sans overflow-hidden selection:bg-primary/30">
      {/* Left Sidebar - Ultra Minimal */}
      <aside className="w-[280px] shrink-0 border-r border-white/5 flex flex-col bg-[#050505] overflow-y-auto">
        <div className="h-14 flex items-center px-8 font-semibold text-[14px] text-zinc-100 border-b border-white/5 shrink-0 bg-black/50 backdrop-blur-xl sticky top-0 z-10 transition-colors">
          <Link
            href="/"
            className="flex items-center gap-2 hover:opacity-80 transition-opacity"
          >
            <div className="text-primary">
              <Layers size={18} strokeWidth={2} />
            </div>
            <span>OmniContext Docs</span>
          </Link>
        </div>

        <div className="p-6">
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
          </div>

          <div className="mb-10">
            <div className="text-[11px] uppercase tracking-widest text-zinc-600 font-semibold mb-4 px-2">
              Models &amp; Pricing
            </div>
            <Link
              href="/docs/models"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/models")}`}
            >
              Available Models
            </Link>
            <Link
              href="/docs/pricing"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/pricing")}`}
            >
              Credit-Based Pricing
            </Link>
          </div>

          <div className="mb-10">
            <div className="text-[11px] uppercase tracking-widest text-zinc-600 font-semibold mb-4 px-2">
              Integrations
            </div>
            <Link
              href="/docs/vscode/setup"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/vscode/setup")}`}
            >
              Visual Studio Code
            </Link>
            <Link
              href="/docs/jetbrains/setup"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/jetbrains/setup")}`}
            >
              JetBrains IDEs
            </Link>
            <Link
              href="/docs/mcp/setup"
              className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive("/docs/mcp/setup")}`}
            >
              MCP Client Support
            </Link>
          </div>
        </div>
      </aside>

      {/* Main Wrapper */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden bg-[#000]">
        {/* Topbar - Glassmorphic minimal */}
        <header className="h-14 shrink-0 flex items-center justify-between px-8 border-b border-white/5 bg-[#000]/70 backdrop-blur-xl z-20">
          {/* Search Bar */}
          <div className="flex items-center bg-[#0a0a0a] border border-white/5 px-3 h-8 rounded-full w-[360px] gap-2 text-zinc-500 cursor-pointer transition-all duration-300 hover:bg-[#111] hover:border-white/10 group">
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
            <button className="flex items-center gap-1.5 text-[11px] font-medium text-emerald-500 bg-emerald-500/10 px-2 py-0.5 rounded-full border border-emerald-500/20 opacity-0 md:opacity-100 transition-all group-hover:bg-emerald-500/20">
              <Zap size={12} /> Ask AI
            </button>
          </div>

          {/* Right Links */}
          <div className="flex items-center gap-8">
            <Link
              href="/"
              className="text-[13px] font-medium text-zinc-500 hover:text-zinc-200 transition-colors duration-200 tracking-tight"
            >
              Back to Site
            </Link>
            <button className="text-zinc-600 hover:text-zinc-300 bg-transparent border-none flex items-center justify-center cursor-pointer transition-colors duration-200">
              <Moon size={16} />
            </button>
          </div>
        </header>

        {/* Content Area */}
        <div className="flex-1 overflow-hidden">{children}</div>
      </div>
    </div>
  );
}
