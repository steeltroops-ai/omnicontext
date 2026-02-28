"use client";

import React from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Search, Moon, ArrowRight, Box, Zap, Layers } from "lucide-react";

export default function DocsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const pathname = usePathname();

  const isLinkActive = (path: string) => {
    return pathname === path
      ? "bg-white/10 text-white font-medium"
      : "text-zinc-400 hover:text-white hover:bg-white/5";
  };

  return (
    <div className="flex h-screen bg-background text-foreground font-sans overflow-hidden">
      {/* Left Sidebar */}
      <aside className="w-[280px] shrink-0 border-r border-border flex flex-col bg-[#070707] overflow-y-auto">
        <div className="h-16 flex items-center px-6 font-bold text-[1.1rem] text-white border-b border-border shrink-0">
          <div className="text-primary mr-2">
            <Layers size={22} strokeWidth={2.5} />
          </div>
          <span>OmniContext</span>
        </div>

        <div className="p-6">
          <div className="mb-8">
            <div className="text-xs uppercase tracking-wider text-muted-foreground font-semibold mb-3 px-2">
              Getting Started
            </div>
            <Link
              href="/docs"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs")}`}
            >
              <Box size={16} className="mr-2 opacity-70" />
              Introduction
            </Link>
            <Link
              href="/docs/quickstart"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/quickstart")}`}
            >
              <ArrowRight size={16} className="mr-2 opacity-70" />
              Quickstart
            </Link>
          </div>

          <div className="mb-8">
            <div className="text-xs uppercase tracking-wider text-muted-foreground font-semibold mb-3 px-2">
              Models & Pricing
            </div>
            <Link
              href="/docs/models"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/models")}`}
            >
              Available Models
            </Link>
            <Link
              href="/docs/pricing"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/pricing")}`}
            >
              Credit-Based Pricing
            </Link>
          </div>

          <div className="mb-8">
            <div className="text-xs uppercase tracking-wider text-muted-foreground font-semibold mb-3 px-2">
              Configuration
            </div>
            <Link
              href="/docs/rules"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/rules")}`}
            >
              Rules & Guidelines
            </Link>
            <Link
              href="/docs/install-app"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/install-app")}`}
            >
              Install App
            </Link>
            <Link
              href="/docs/network"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/network")}`}
            >
              Network configuration
            </Link>
          </div>

          <div className="mb-8">
            <div className="text-xs uppercase tracking-wider text-muted-foreground font-semibold mb-3 px-2">
              Visual Studio Code
            </div>
            <Link
              href="/docs/vscode/setup"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/vscode/setup")}`}
            >
              Setup OmniContext
            </Link>
            <Link
              href="/docs/vscode/agent"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/vscode/agent")}`}
            >
              Agent
            </Link>
            <Link
              href="/docs/vscode/chat"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/vscode/chat")}`}
            >
              Chat
            </Link>
            <Link
              href="/docs/vscode/completions"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/vscode/completions")}`}
            >
              Completions
            </Link>
          </div>

          <div className="mb-8">
            <div className="text-xs uppercase tracking-wider text-muted-foreground font-semibold mb-3 px-2">
              JetBrains IDEs
            </div>
            <Link
              href="/docs/jetbrains/setup"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/jetbrains/setup")}`}
            >
              Setup OmniContext
            </Link>
            <Link
              href="/docs/jetbrains/agent"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/jetbrains/agent")}`}
            >
              Agent
            </Link>
            <Link
              href="/docs/jetbrains/chat"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/jetbrains/chat")}`}
            >
              Chat
            </Link>
            <Link
              href="/docs/jetbrains/completions"
              className={`flex items-center px-2 py-1.5 mb-1 rounded-md text-sm transition-all duration-150 ${isLinkActive("/docs/jetbrains/completions")}`}
            >
              Completions
            </Link>
          </div>
        </div>
      </aside>

      {/* Main Wrapper */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* Topbar */}
        <header className="h-16 shrink-0 flex items-center justify-between px-6 border-b border-border bg-[#0a0a0a]/50 backdrop-blur-md">
          {/* Search Bar */}
          <div className="flex items-center bg-white/5 border border-white/10 px-3 py-1.5 rounded-md w-[400px] gap-2 text-muted-foreground cursor-pointer transition-all duration-200 hover:bg-white/10 hover:border-white/20">
            <Search size={16} />
            <span className="flex-1 text-sm">Search...</span>
            <span className="font-mono text-xs bg-white/10 px-1.5 py-0.5 rounded text-white">
              Ctrl K
            </span>
            <button className="flex items-center gap-1.5 text-xs text-primary bg-[rgba(0,208,107,0.1)] px-2 py-1 rounded border border-[rgba(0,208,107,0.2)]">
              <Zap size={14} /> Ask AI
            </button>
          </div>

          {/* Right Links */}
          <div className="flex items-center gap-6">
            <Link
              href="/status"
              className="text-sm font-medium text-muted-foreground hover:text-white transition-colors duration-150"
            >
              Status
            </Link>
            <Link
              href="/blog"
              className="text-sm font-medium text-muted-foreground hover:text-white transition-colors duration-150"
            >
              Blog
            </Link>
            <Link
              href="/support"
              className="text-sm font-medium text-muted-foreground hover:text-white transition-colors duration-150"
            >
              Support
            </Link>
            <button className="text-muted-foreground hover:text-white bg-transparent border-none flex items-center justify-center cursor-pointer transition-colors duration-150">
              <Moon size={18} />
            </button>
          </div>
        </header>

        {/* Content Area + Right TOC Layout inside children render component container generally */}
        {children}
      </div>
    </div>
  );
}
