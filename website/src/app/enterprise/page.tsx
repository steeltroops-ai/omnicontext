"use client";

import Link from "next/link";
import { siteConfig } from "@/config/site";
import {
  Shield,
  Users,
  Server,
  BarChart3,
  Lock,
  ChevronRight,
} from "lucide-react";
import { SiteNav } from "@/components/site-nav";
import { SiteFooterFull } from "@/components/site-footer-full";

const features = [
  {
    icon: Server,
    title: "Hosted REST API",
    description:
      "Deploy OmniContext as a centralized API endpoint. Agents across your entire organization connect to a single, always-indexed knowledge base via REST or gRPC.",
  },
  {
    icon: Users,
    title: "Team Knowledge Sharing",
    description:
      "Share indexed context across teams. Multi-repo workspaces, org-wide patterns, and shared knowledge graphs accessible to every developer and agent.",
  },
  {
    icon: Lock,
    title: "SSO & Access Control",
    description:
      "Enterprise SSO integration (SAML, OIDC), role-based access control, and fine-grained permissions for who can query which repositories.",
  },
  {
    icon: Shield,
    title: "Security & Compliance",
    description:
      "SOC 2 compliance readiness, API key management with rate limiting, and full audit trails. Your source code never leaves your infrastructure.",
  },
  {
    icon: BarChart3,
    title: "Usage Metering",
    description:
      "Track API usage per team, per user, per repository. Built-in metering for capacity planning and cost allocation across departments.",
  },
  {
    icon: BarChart3,
    title: "SLA Guarantees",
    description:
      "99.9% uptime SLA, dedicated support channel, and guaranteed response times for critical issues. Priority access to new features and releases.",
  },
];

export default function EnterprisePage() {
  return (
    <div className="flex flex-col min-h-screen bg-[#09090B] selection:bg-primary/30">
      <SiteNav />

      <main className="flex-1 flex flex-col">
        {/* Hero */}
        <section className="pt-32 sm:pt-40 pb-20 sm:pb-28 px-6 sm:px-8 md:px-16 max-w-[1400px] mx-auto w-full text-center relative">
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-[60%] w-[500px] h-[400px] bg-primary/5 blur-[120px] rounded-full pointer-events-none" />
          <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-6">
            Enterprise
          </div>
          <h1 className="text-4xl sm:text-5xl md:text-[64px] font-semibold text-white tracking-tighter mb-6 leading-[1.05] relative z-10">
            Code context for
            <br className="hidden sm:block" />
            the entire organization.
          </h1>
          <p className="text-[16px] sm:text-[18px] text-zinc-400 max-w-[580px] mx-auto tracking-tight leading-snug mb-10 relative z-10">
            Deploy OmniContext as a hosted API for your engineering org.
            Team-wide knowledge sharing, audit logging, SSO, and SLA guarantees.
          </p>
          <div className="flex flex-col sm:flex-row gap-4 justify-center items-center relative z-10">
            <Link
              href={`mailto:${siteConfig.links.email}`}
              className="w-full sm:w-auto px-7 py-3.5 text-[15px] font-medium rounded-full bg-zinc-100 text-black hover:scale-105 active:scale-95 transition-all duration-300"
            >
              Contact Sales
            </Link>
            <Link
              href="/docs"
              className="w-full sm:w-auto px-7 py-3.5 text-[15px] font-medium rounded-full bg-zinc-900 text-white border border-white/10 hover:bg-zinc-800 transition-colors"
            >
              Read Documentation
            </Link>
          </div>
        </section>

        {/* Features Grid */}
        <section className="pb-24 sm:pb-32 px-6 sm:px-8 md:px-16 max-w-[1400px] mx-auto w-full">
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
            {features.map((feat, i) => (
              <div
                key={i}
                className="bg-[#0E0E11] border border-white/5 p-7 rounded-[20px] flex flex-col group hover:border-white/10 transition-all duration-300"
              >
                <div className="w-12 h-12 rounded-full bg-white/5 flex items-center justify-center mb-6 group-hover:bg-emerald-500/10 group-hover:text-emerald-400 text-zinc-500 transition-colors">
                  <feat.icon size={20} strokeWidth={1.5} />
                </div>
                <h3 className="text-[18px] font-semibold text-zinc-100 mb-3 tracking-tight group-hover:text-white transition-colors">
                  {feat.title}
                </h3>
                <p className="text-[14px] text-zinc-500 leading-relaxed tracking-tight">
                  {feat.description}
                </p>
              </div>
            ))}
          </div>
        </section>

        {/* CTA */}
        <section className="py-20 sm:py-28 bg-[#0E0E11] border-y border-white/5 text-center px-6 sm:px-8 md:px-16 relative">
          <div className="absolute inset-0 bg-[radial-gradient(circle_at_center,rgba(34,197,94,0.03)_0%,transparent_60%)] pointer-events-none" />
          <div className="max-w-[560px] mx-auto relative z-10">
            <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
              Get Started
            </div>
            <h2 className="text-3xl sm:text-[44px] font-semibold text-white tracking-tighter mb-5 leading-tight">
              Give your codebase the context engine it deserves
            </h2>
            <p className="text-[16px] text-zinc-400 mb-10 tracking-tight leading-snug">
              Install OmniContext to get started. Works with codebases of any
              size, from side projects to enterprise monorepos.
            </p>
            <div className="flex flex-col sm:flex-row gap-4 justify-center">
              <Link
                href="/docs/quickstart"
                className="px-7 py-3.5 text-[15px] font-medium rounded-full bg-emerald-500 text-black hover:bg-emerald-400 transition-colors flex items-center justify-center gap-2"
              >
                Install {siteConfig.name} <ChevronRight size={16} />
              </Link>
              <Link
                href={`mailto:${siteConfig.links.email}`}
                className="px-7 py-3.5 text-[15px] font-medium rounded-full bg-zinc-900 text-white border border-white/10 hover:bg-zinc-800 transition-colors"
              >
                Contact Sales
              </Link>
            </div>
          </div>
        </section>

        <SiteFooterFull />
      </main>
    </div>
  );
}
