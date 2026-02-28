"use client";

import Link from "next/link";
import { siteConfig } from "@/config/site";
import { Logo } from "@/components/icons";
import {
  Shield,
  Users,
  Server,
  BarChart3,
  Lock,
  ChevronRight,
  Github,
  ArrowUpRight,
} from "lucide-react";

export default function EnterprisePage() {
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
              className="text-[13px] font-medium text-zinc-100 transition-colors"
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

      <main className="flex-1 overflow-y-scroll custom-scrollbar">
        {/* Hero */}
        <section className="pt-[160px] pb-[120px] px-8 md:px-16 max-w-[1200px] mx-auto w-full text-center relative">
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-[60%] w-[500px] h-[400px] bg-primary/5 blur-[120px] rounded-full pointer-events-none" />
          <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-6">
            Enterprise
          </div>
          <h1 className="text-4xl md:text-[64px] font-semibold text-white tracking-tighter mb-8 leading-[1.05] relative z-10">
            Code context for
            <br />
            the entire organization.
          </h1>
          <p className="text-[18px] text-zinc-400 max-w-[600px] mx-auto tracking-tight leading-snug mb-12 relative z-10">
            Deploy OmniContext as a hosted API for your engineering org.
            Team-wide knowledge sharing, audit logging, SSO, and SLA guarantees.
          </p>
          <div className="flex flex-col sm:flex-row gap-4 justify-center items-center relative z-10">
            <Link
              href={`mailto:${siteConfig.links.email}`}
              className="px-6 py-3 text-[15px] font-medium rounded-full bg-zinc-100 text-black hover:scale-105 active:scale-95 transition-all duration-300"
            >
              Contact Sales
            </Link>
            <Link
              href="/docs"
              className="px-6 py-3 text-[15px] font-medium rounded-full bg-zinc-900 text-white border border-white/10 hover:bg-zinc-800 transition-colors"
            >
              Read Documentation
            </Link>
          </div>
        </section>

        {/* Features Grid */}
        <section className="pb-[160px] px-8 md:px-16 max-w-[1200px] mx-auto w-full">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
            <div className="bg-[#0E0E11] border border-white/5 p-8 rounded-[20px] flex flex-col group">
              <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <Server size={20} strokeWidth={1.5} />
              </div>
              <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight">
                Hosted REST API
              </h3>
              <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
                Deploy OmniContext as a centralized API endpoint. Agents across
                your entire organization connect to a single, always-indexed
                knowledge base via REST or gRPC.
              </p>
            </div>
            <div className="bg-[#0E0E11] border border-white/5 p-8 rounded-[20px] flex flex-col group">
              <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <Users size={20} strokeWidth={1.5} />
              </div>
              <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight">
                Team Knowledge Sharing
              </h3>
              <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
                Share indexed context across teams. Multi-repo workspaces,
                org-wide patterns, and shared knowledge graphs accessible to
                every developer and agent.
              </p>
            </div>
            <div className="bg-[#0E0E11] border border-white/5 p-8 rounded-[20px] flex flex-col group">
              <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <Lock size={20} strokeWidth={1.5} />
              </div>
              <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight">
                SSO &amp; Access Control
              </h3>
              <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
                Enterprise SSO integration (SAML, OIDC), role-based access
                control, and fine-grained permissions for who can query which
                repositories.
              </p>
            </div>
            <div className="bg-[#0E0E11] border border-white/5 p-8 rounded-[20px] flex flex-col group">
              <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <Shield size={20} strokeWidth={1.5} />
              </div>
              <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight">
                Security &amp; Compliance
              </h3>
              <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
                SOC 2 compliance readiness, API key management with rate
                limiting, and full audit trails. Your source code never leaves
                your infrastructure.
              </p>
            </div>
            <div className="bg-[#0E0E11] border border-white/5 p-8 rounded-[20px] flex flex-col group">
              <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <BarChart3 size={20} strokeWidth={1.5} />
              </div>
              <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight">
                Usage Metering
              </h3>
              <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
                Track API usage per team, per user, per repository. Built-in
                metering for capacity planning and cost allocation across
                departments.
              </p>
            </div>
            <div className="bg-[#0E0E11] border border-white/5 p-8 rounded-[20px] flex flex-col group">
              <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                <BarChart3 size={20} strokeWidth={1.5} />
              </div>
              <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight">
                SLA Guarantees
              </h3>
              <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
                99.9% uptime SLA, dedicated support channel, and guaranteed
                response times for critical issues. Priority access to new
                features and releases.
              </p>
            </div>
          </div>
        </section>

        {/* CTA */}
        <section className="py-[120px] bg-[#0E0E11] border-y border-white/5 text-center px-8 relative">
          <div className="absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(34,197,94,0.03)_0%,transparent_60%)] pointer-events-none" />
          <div className="max-w-[600px] mx-auto relative z-10">
            <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
              Get Started
            </div>
            <h2 className="text-3xl md:text-[44px] font-semibold text-white tracking-tighter mb-6 leading-tight">
              Give your codebase the context engine it deserves
            </h2>
            <p className="text-[16px] text-zinc-400 mb-10 tracking-tight leading-snug">
              Install OmniContext to get started. Works with codebases of any
              size, from side projects to enterprise monorepos.
            </p>
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <Link
                href="/docs/quickstart"
                className="px-6 py-3 text-[15px] font-medium rounded-full bg-emerald-500 text-black hover:bg-emerald-400 transition-colors flex items-center justify-center gap-2"
              >
                Install OmniContext <ChevronRight size={16} />
              </Link>
              <Link
                href={`mailto:${siteConfig.links.email}`}
                className="px-6 py-3 text-[15px] font-medium rounded-full bg-zinc-900 text-white border border-white/10 hover:bg-zinc-800 transition-colors"
              >
                Contact Sales
              </Link>
            </div>
          </div>
        </section>

        {/* Footer */}
        <footer className="py-16 px-8 md:px-16 bg-[#09090B] border-t border-white/5">
          <div className="max-w-[1200px] mx-auto">
            <div className="grid grid-cols-2 md:grid-cols-5 gap-12 mb-16">
              <div className="col-span-2 md:col-span-1">
                <Link
                  href="/"
                  className="flex items-center gap-2 font-semibold text-sm text-zinc-100 mb-4"
                >
                  <Logo
                    className="text-primary"
                    size={siteConfig.branding.sizes.footer}
                  />
                  <span>{siteConfig.name}</span>
                </Link>
                <p className="text-[12px] text-zinc-600 leading-relaxed">
                  High-performance code context engine. Open-source core. Built
                  in Rust.
                </p>
              </div>
              <div>
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Product
                </div>
                <div className="flex flex-col gap-2 text-[13px] text-zinc-500">
                  <Link
                    href="/docs"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Documentation
                  </Link>
                  <Link
                    href="/docs/quickstart"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Quickstart
                  </Link>
                  <Link
                    href="/docs/pricing"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Pricing
                  </Link>
                  <Link
                    href="/enterprise"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Enterprise
                  </Link>
                </div>
              </div>
              <div>
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Resources
                </div>
                <div className="flex flex-col gap-2 text-[13px] text-zinc-500">
                  <Link
                    href="/blog"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Blog
                  </Link>
                  <Link
                    href="/docs/architecture"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Architecture
                  </Link>
                  <Link
                    href="/docs/supported-languages"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Supported Languages
                  </Link>
                </div>
              </div>
              <div>
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Company
                </div>
                <div className="flex flex-col gap-2 text-[13px] text-zinc-500">
                  <Link
                    href="/support"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Support
                  </Link>
                  <Link
                    href={`mailto:${siteConfig.links.email}`}
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Contact
                  </Link>
                  <a
                    href={siteConfig.links.github}
                    className="hover:text-zinc-200 transition-colors"
                  >
                    GitHub
                  </a>
                </div>
              </div>
              <div>
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Legal
                </div>
                <div className="flex flex-col gap-2 text-[13px] text-zinc-500">
                  <Link
                    href="/privacy"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Privacy Policy
                  </Link>
                  <Link
                    href="/terms"
                    className="hover:text-zinc-200 transition-colors"
                  >
                    Terms of Service
                  </Link>
                </div>
              </div>
            </div>
            <div className="border-t border-white/5 pt-8 flex flex-col md:flex-row items-center justify-between gap-4">
              <p className="text-[12px] text-zinc-600">
                (c) 2026 {siteConfig.name}. All rights reserved. Apache-2.0
                License.
              </p>
              <div className="flex items-center gap-4">
                <a
                  href={siteConfig.links.github}
                  className="text-zinc-600 hover:text-zinc-300 transition-colors"
                >
                  <Github size={18} />
                </a>
              </div>
            </div>
          </div>
        </footer>
      </main>
    </div>
  );
}
