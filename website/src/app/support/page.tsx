"use client";

import Link from "next/link";
import { siteConfig } from "@/config/site";
import { Logo } from "@/components/icons";
import {
  Github,
  MessageCircle,
  BookOpen,
  Bug,
  ArrowUpRight,
} from "lucide-react";

export default function SupportPage() {
  return (
    <div className="flex flex-col h-screen overflow-hidden bg-[#09090B] selection:bg-primary/30">
      {/* Navigation */}
      <nav className="shrink-0 w-full h-14 pr-[10px] flex items-center justify-center z-50 border-b border-white/5 bg-[#09090B]/50 backdrop-blur-xl">
        <div className="flex items-center justify-between w-full max-w-[1200px] px-8 md:px-16">
          <Link
            href="/"
            className="flex items-center gap-2 font-semibold text-sm text-zinc-100 transition-opacity hover:opacity-80"
          >
            <Logo
              className="text-primary"
              size={siteConfig.branding.sizes.header}
            />
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

      <main className="flex-1 overflow-y-scroll custom-scrollbar flex flex-col items-center pt-[100px] pb-[80px]">
        <h1 className="text-4xl md:text-[56px] font-semibold text-white tracking-tighter mb-6 leading-tight">
          Support
        </h1>
        <p className="text-[18px] text-zinc-400 max-w-[600px] tracking-tight leading-snug mb-20">
          Get help with OmniContext. Whether you are setting up for the first
          time or debugging a complex integration, we are here to help.
        </p>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-8 mb-20">
          <a
            href={`${siteConfig.links.github}/issues`}
            target="_blank"
            rel="noopener noreferrer"
            className="bg-[#0E0E11] border border-white/5 p-8 rounded-[20px] flex flex-col group hover:border-white/10 transition-all duration-300"
          >
            <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
              <Bug size={20} strokeWidth={1.5} />
            </div>
            <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-white transition-colors flex items-center gap-2">
              Report a Bug <ArrowUpRight size={14} className="text-zinc-600" />
            </h3>
            <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
              Found a bug? Open an issue on GitHub. Include your OS, Rust
              version, and the error output. We triage daily.
            </p>
          </a>

          <Link
            href="/docs"
            className="bg-[#0E0E11] border border-white/5 p-8 rounded-[20px] flex flex-col group hover:border-white/10 transition-all duration-300"
          >
            <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
              <BookOpen size={20} strokeWidth={1.5} />
            </div>
            <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-white transition-colors">
              Documentation
            </h3>
            <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
              Comprehensive guides covering installation, MCP integration,
              search tuning, dependency graph queries, and enterprise
              deployment.
            </p>
          </Link>

          <a
            href={`${siteConfig.links.github}/discussions`}
            target="_blank"
            rel="noopener noreferrer"
            className="bg-[#0E0E11] border border-white/5 p-8 rounded-[20px] flex flex-col group hover:border-white/10 transition-all duration-300"
          >
            <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
              <MessageCircle size={20} strokeWidth={1.5} />
            </div>
            <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-white transition-colors flex items-center gap-2">
              Community Discussions{" "}
              <ArrowUpRight size={14} className="text-zinc-600" />
            </h3>
            <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
              Ask questions, share your setup, or request features. The GitHub
              Discussions board is the best place for open-ended conversations.
            </p>
          </a>

          <a
            href={`mailto:${siteConfig.links.email}`}
            className="bg-[#0E0E11] border border-white/5 p-8 rounded-[20px] flex flex-col group hover:border-white/10 transition-all duration-300"
          >
            <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
              <Logo size={20} />
            </div>
            <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-white transition-colors">
              Enterprise Support
            </h3>
            <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
              Enterprise customers get dedicated support with guaranteed
              response times. Contact us to discuss your organization&apos;s
              needs.
            </p>
          </a>
        </div>

        {/* Footer */}
        <footer className="py-12 px-8 md:px-16 border-t border-white/5 bg-[#09090B] mt-auto">
          <div className="max-w-[1200px] mx-auto flex flex-col md:flex-row items-center justify-between gap-6">
            <Link
              href="/"
              className="flex items-center gap-2 font-semibold text-sm text-zinc-100"
            >
              <Logo
                className="text-primary"
                size={siteConfig.branding.sizes.footer}
              />
              <span>{siteConfig.name}</span>
            </Link>
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
    </div>
  );
}
