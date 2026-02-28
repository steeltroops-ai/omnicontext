import Link from "next/link";
import { Github } from "lucide-react";
import { siteConfig } from "@/config/site";
import { Logo } from "@/components/icons";

export function SiteFooterFull() {
  return (
    <footer className="py-16 px-6 sm:px-8 md:px-16 bg-[#09090B] border-t border-white/5">
      <div className="max-w-[1400px] mx-auto">
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-[1.5fr_1fr_1fr_1fr_1fr] gap-x-8 gap-y-10 mb-16">
          <div className="col-span-1 sm:col-span-2 lg:col-span-1">
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
              High-performance code context engine. Open-source core. Built in
              Rust.
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

        <div className="border-t border-white/5 pt-8 flex flex-col sm:flex-row items-center justify-between gap-4 text-center sm:text-left">
          <p className="text-[12px] text-zinc-600">
            (c) 2026 {siteConfig.name}. All rights reserved. Apache-2.0 License.
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
  );
}
