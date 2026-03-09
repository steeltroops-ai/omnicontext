import React from "react";
import Link from "next/link";
import { Search, Github } from "lucide-react";
import { siteConfig } from "@/config/site";
import { Logo } from "@/components/icons";
import { getDocsByCategory } from "@/lib/markdown";
import { DocsSidebar } from "@/components/docs-sidebar";

export default function DocsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const docsByCategory = getDocsByCategory();

  return (
    <div className="flex flex-col md:flex-row min-h-screen bg-[#09090B] text-zinc-100 font-sans selection:bg-primary/30">
      {/* Left Sidebar */}
      <aside className="w-[280px] shrink-0 border-r border-white/5 flex flex-col bg-[#0E0E11] md:fixed md:h-screen md:overflow-y-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden" data-lenis-prevent>
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
          <DocsSidebar docsByCategory={docsByCategory} />
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
      <div className="flex-1 flex flex-col min-w-0 md:ml-[280px]">
        {/* Topbar */}
        <header className="h-14 shrink-0 flex items-center justify-between px-8 border-b border-white/5 bg-[#09090B]/70 backdrop-blur-xl z-20 md:fixed md:top-0 md:right-0 md:left-[280px]">
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
        <div className="flex-1 md:mt-14">{children}</div>
      </div>
    </div>
  );
}
