"use client";

import React from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  Search,
  Moon,
  ArrowRight,
  Settings,
  Box,
  Database,
  Terminal,
  ShieldCheck,
  Zap,
  Layers,
  Server,
  Code,
  LayoutTemplate,
} from "lucide-react";
import styles from "./docs.module.css";

export default function DocsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const pathname = usePathname();

  const isLinkActive = (path: string) => {
    return pathname === path ? styles.sidebarLinkActive : "";
  };

  return (
    <div className={styles.layout}>
      {/* Left Sidebar */}
      <aside className={styles.sidebar}>
        <div className={styles.sidebarHeader}>
          <div className={styles.sidebarIcon}>
            <Layers size={22} strokeWidth={2.5} />
          </div>
          <span>OmniContext</span>
        </div>

        <div className={styles.sidebarContent}>
          <div className={styles.sidebarGroup}>
            <div className={styles.sidebarGroupTitle}>Getting Started</div>
            <Link
              href="/docs"
              className={`${styles.sidebarLink} ${isLinkActive("/docs")}`}
            >
              <Box size={16} className={styles.sidebarLinkIcon} />
              Introduction
            </Link>
            <Link
              href="/docs/quickstart"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/quickstart")}`}
            >
              <ArrowRight size={16} className={styles.sidebarLinkIcon} />
              Quickstart
            </Link>
          </div>

          <div className={styles.sidebarGroup}>
            <div className={styles.sidebarGroupTitle}>Models & Pricing</div>
            <Link
              href="/docs/models"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/models")}`}
            >
              Available Models
            </Link>
            <Link
              href="/docs/pricing"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/pricing")}`}
            >
              Credit-Based Pricing
            </Link>
          </div>

          <div className={styles.sidebarGroup}>
            <div className={styles.sidebarGroupTitle}>Configuration</div>
            <Link
              href="/docs/rules"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/rules")}`}
            >
              Rules & Guidelines
            </Link>
            <Link
              href="/docs/install-app"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/install-app")}`}
            >
              Install App
            </Link>
            <Link
              href="/docs/network"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/network")}`}
            >
              Network configuration
            </Link>
          </div>

          <div className={styles.sidebarGroup}>
            <div className={styles.sidebarGroupTitle}>Visual Studio Code</div>
            <Link
              href="/docs/vscode/setup"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/vscode/setup")}`}
            >
              Setup OmniContext
            </Link>
            <Link
              href="/docs/vscode/agent"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/vscode/agent")}`}
            >
              Agent
            </Link>
            <Link
              href="/docs/vscode/chat"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/vscode/chat")}`}
            >
              Chat
            </Link>
            <Link
              href="/docs/vscode/completions"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/vscode/completions")}`}
            >
              Completions
            </Link>
          </div>

          <div className={styles.sidebarGroup}>
            <div className={styles.sidebarGroupTitle}>JetBrains IDEs</div>
            <Link
              href="/docs/jetbrains/setup"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/jetbrains/setup")}`}
            >
              Setup OmniContext
            </Link>
            <Link
              href="/docs/jetbrains/agent"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/jetbrains/agent")}`}
            >
              Agent
            </Link>
            <Link
              href="/docs/jetbrains/chat"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/jetbrains/chat")}`}
            >
              Chat
            </Link>
            <Link
              href="/docs/jetbrains/completions"
              className={`${styles.sidebarLink} ${isLinkActive("/docs/jetbrains/completions")}`}
            >
              Completions
            </Link>
          </div>
        </div>
      </aside>

      {/* Main Wrapper */}
      <div className={styles.mainWrapper}>
        {/* Topbar */}
        <header className={styles.topbar}>
          {/* Search Bar */}
          <div className={styles.searchBar}>
            <Search size={16} />
            <span className={styles.searchPlaceholder}>Search...</span>
            <span style={{ flex: 1 }}></span>
            <span className={styles.searchShortcut}>Ctrl K</span>
            <button className={styles.askAiBtn}>
              <Zap size={14} /> Ask AI
            </button>
          </div>

          {/* Right Links */}
          <div className={styles.topLinks}>
            <Link href="/status" className={styles.topLink}>
              Status
            </Link>
            <Link href="/blog" className={styles.topLink}>
              Blog
            </Link>
            <Link href="/support" className={styles.topLink}>
              Support
            </Link>
            <button className={styles.iconBtn}>
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
