import Link from "next/link";
import { Github } from "lucide-react";
import { siteConfig } from "@/config/site";
import { Logo } from "@/components/icons";

export function SiteFooterMini() {
  return (
    <footer className="py-10 px-6 sm:px-8 md:px-16 border-t border-white/5 bg-[#09090B] mt-auto">
      <div className="max-w-[1400px] mx-auto flex flex-col sm:flex-row items-center justify-between gap-6 text-center sm:text-left">
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
        <div className="flex flex-wrap justify-center sm:justify-start items-center gap-x-6 gap-y-2 text-[13px] text-zinc-500">
          <Link href="/docs" className="hover:text-zinc-200 transition-colors">
            Docs
          </Link>
          <Link href="/blog" className="hover:text-zinc-200 transition-colors">
            Blog
          </Link>
          <Link
            href="/enterprise"
            className="hover:text-zinc-200 transition-colors"
          >
            Enterprise
          </Link>
          <Link
            href="/support"
            className="hover:text-zinc-200 transition-colors"
          >
            Support
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
  );
}
