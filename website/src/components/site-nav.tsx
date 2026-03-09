"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { ArrowUpRight, Menu, X } from "lucide-react";
import { siteConfig } from "@/config/site";
import { Logo } from "@/components/icons";

interface SiteNavProps {
  /** If true, nav starts transparent and transitions on scroll (hero pages).
   *  If false, nav is always the frosted glass bar (inner pages). */
  transparent?: boolean;
  /** Element ID to attach scroll listener to (default: window scroll) */
  scrollTarget?: string;
}

export function SiteNav({ transparent = false, scrollTarget }: SiteNavProps) {
  const pathname = usePathname();
  // Non-transparent nav is always in the "scrolled" (frosted) state
  const [scrolled, setScrolled] = useState(() => !transparent);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  useEffect(() => {
    if (!transparent) return;

    const target = scrollTarget ? document.getElementById(scrollTarget) : null;

    const onScroll = () =>
      setScrolled((target?.scrollTop ?? window.scrollY) > 20);

    if (target) {
      target.addEventListener("scroll", onScroll);
    } else {
      window.addEventListener("scroll", onScroll);
    }

    return () => {
      if (target) {
        target.removeEventListener("scroll", onScroll);
      } else {
        window.removeEventListener("scroll", onScroll);
      }
    };
  }, [transparent, scrollTarget]);

  const isActive = (href: string) =>
    pathname === href ? "text-zinc-100" : "text-zinc-400 hover:text-zinc-100";

  return (
    <nav
      className={`fixed top-0 left-0 w-full h-14 flex items-center justify-center z-50 transition-all duration-500 px-6 sm:px-8 md:px-16 ${
        scrolled
          ? "border-b border-white/[0.07] bg-black/30 backdrop-blur-2xl shadow-[0_4px_30px_rgba(0,0,0,0.25)]"
          : "border-b border-transparent bg-transparent backdrop-blur-none"
      }`}
    >
      <div className="flex items-center justify-between w-full max-w-[1400px]">
        <Link
          href="/"
          className="flex items-center gap-2.5 font-semibold text-[15px] text-zinc-100 transition-opacity hover:opacity-80"
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
            className={`text-[13px] font-medium transition-colors ${isActive("/docs")}`}
          >
            Docs
          </Link>
          {/* Removed dead links - pages don't exist yet
          <Link
            href="/blog"
            className={`text-[13px] font-medium transition-colors ${isActive("/blog")}`}
          >
            Blog
          </Link>
          <Link
            href="/enterprise"
            className={`text-[13px] font-medium transition-colors ${isActive("/enterprise")}`}
          >
            Enterprise
          </Link>
          <Link
            href="/support"
            className={`text-[13px] font-medium transition-colors ${isActive("/support")}`}
          >
            Support
          </Link>
          */}
        </div>

        <div className="flex items-center gap-4">
          <a
            href={siteConfig.links.github}
            target="_blank"
            rel="noopener noreferrer"
            className="text-[13px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors hidden sm:flex items-center gap-1"
          >
            GitHub <ArrowUpRight size={12} />
          </a>

          {/* Menu button for mobile - white icon only */}
          <button
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            className="md:hidden text-white hover:text-zinc-300 transition-colors"
            aria-label="Toggle menu"
          >
            {mobileMenuOpen ? <X size={22} /> : <Menu size={22} />}
          </button>

          {/* Get Started button for desktop */}
          <Link
            href="/docs/quickstart"
            className="hidden md:block px-3 py-1.5 text-[13px] font-medium rounded-full bg-zinc-100 text-black hover:bg-white transition-colors"
          >
            Get Started
          </Link>
        </div>
      </div>

      {/* Mobile Menu - matches header transparency */}
      {mobileMenuOpen && (
        <div
          className={`md:hidden absolute top-14 left-0 w-full transition-all duration-500 ${
            scrolled
              ? "border-b border-white/[0.07] bg-black/30 backdrop-blur-2xl shadow-[0_4px_30px_rgba(0,0,0,0.25)]"
              : "border-b border-white/[0.07] bg-black/20 backdrop-blur-xl"
          }`}
        >
          <div className="flex flex-col px-6 py-4 gap-3">
            <Link
              href="/docs"
              onClick={() => setMobileMenuOpen(false)}
              className={`text-[14px] font-medium transition-colors py-2 ${isActive("/docs")}`}
            >
              Docs
            </Link>
            {/* Removed dead links - pages don't exist yet
            <Link
              href="/blog"
              onClick={() => setMobileMenuOpen(false)}
              className={`text-[14px] font-medium transition-colors py-2 ${isActive("/blog")}`}
            >
              Blog
            </Link>
            <Link
              href="/enterprise"
              onClick={() => setMobileMenuOpen(false)}
              className={`text-[14px] font-medium transition-colors py-2 ${isActive("/enterprise")}`}
            >
              Enterprise
            </Link>
            <Link
              href="/support"
              onClick={() => setMobileMenuOpen(false)}
              className={`text-[14px] font-medium transition-colors py-2 ${isActive("/support")}`}
            >
              Support
            </Link>
            */}
            <a
              href={siteConfig.links.github}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[14px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors py-2 flex items-center gap-1"
            >
              GitHub <ArrowUpRight size={12} />
            </a>
            <Link
              href="/docs/quickstart"
              onClick={() => setMobileMenuOpen(false)}
              className="mt-2 px-4 py-2.5 text-[14px] font-medium rounded-full bg-zinc-100 text-black hover:bg-white transition-colors text-center"
            >
              Get Started
            </Link>
          </div>
        </div>
      )}
    </nav>
  );
}
