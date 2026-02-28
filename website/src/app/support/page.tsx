"use client";

import Link from "next/link";
import { siteConfig } from "@/config/site";
import { Logo } from "@/components/icons";
import {
  MessageCircle,
  BookOpen,
  Bug,
  ArrowUpRight,
  ChevronRight,
} from "lucide-react";
import { SiteNav } from "@/components/site-nav";
import { SiteFooterMini } from "@/components/site-footer-mini";

export default function SupportPage() {
  return (
    <div className="flex flex-col min-h-screen bg-[#09090B] selection:bg-primary/30">
      <SiteNav />

      <main className="flex-1 flex flex-col pt-24 sm:pt-28">
        <div className="w-full max-w-[1400px] mx-auto px-6 sm:px-8 md:px-16 pb-20">
          <h1 className="text-4xl sm:text-5xl md:text-[56px] font-semibold text-white tracking-tighter mb-4 leading-tight">
            Support
          </h1>
          <p className="text-[16px] sm:text-[18px] text-zinc-400 max-w-[560px] tracking-tight leading-snug mb-16">
            Get help with {siteConfig.name}. Whether you are setting up for the
            first time or debugging a complex integration, we are here to help.
          </p>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-6 mb-20">
            <a
              href={`${siteConfig.links.github}/issues`}
              target="_blank"
              rel="noopener noreferrer"
              className="bg-[#0E0E11] border border-white/5 p-7 rounded-[20px] flex flex-col group hover:border-white/10 transition-all duration-300"
            >
              <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <Bug size={20} strokeWidth={1.5} />
              </div>
              <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-white transition-colors flex items-center gap-2">
                Report a Bug{" "}
                <ArrowUpRight size={14} className="text-zinc-600" />
              </h3>
              <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
                Found a bug? Open an issue on GitHub. Include your OS, Rust
                version, and the error output. We triage daily.
              </p>
            </a>

            <Link
              href="/docs"
              className="bg-[#0E0E11] border border-white/5 p-7 rounded-[20px] flex flex-col group hover:border-white/10 transition-all duration-300"
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
              className="bg-[#0E0E11] border border-white/5 p-7 rounded-[20px] flex flex-col group hover:border-white/10 transition-all duration-300"
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
                Discussions board is the best place for open-ended
                conversations.
              </p>
            </a>

            <a
              href={`mailto:${siteConfig.links.email}`}
              className="bg-[#0E0E11] border border-white/5 p-7 rounded-[20px] flex flex-col group hover:border-white/10 transition-all duration-300"
            >
              <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <Logo size={20} />
              </div>
              <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-white transition-colors flex items-center gap-2">
                Enterprise Support{" "}
                <ChevronRight size={14} className="text-zinc-600" />
              </h3>
              <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
                Enterprise customers get dedicated support with guaranteed
                response times. Contact us to discuss your organization&apos;s
                needs.
              </p>
            </a>
          </div>
        </div>

        <SiteFooterMini />
      </main>
    </div>
  );
}
