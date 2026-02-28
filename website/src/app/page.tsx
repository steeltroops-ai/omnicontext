"use client";

import { motion } from "framer-motion";
import {
  Terminal,
  Network,
  ShieldCheck,
  Github,
  Zap,
  SearchCode,
} from "lucide-react";
import styles from "./page.module.css";
import Link from "next/link";

export default function Home() {
  return (
    <main className={styles.main}>
      {/* Navigation */}
      <nav className={styles.nav}>
        <div className={styles.logo}>
          <SearchCode className={styles.logoIcon} size={28} strokeWidth={2.5} />
          <span>OmniContext</span>
        </div>

        <div className={styles.navLinks}>
          <Link href="/docs" className={styles.navLink}>
            Documentation
          </Link>
          <Link href="/blog" className={styles.navLink}>
            Blog
          </Link>
          <Link href="/enterprise" className={styles.navLink}>
            Enterprise
          </Link>
        </div>

        <div className={styles.navRight}>
          <button className={styles.buttonOutline}>Sign In</button>
          <button className={`${styles.buttonPrimary}`}>Get Started</button>
        </div>
      </nav>

      {/* Hero Section */}
      <section className={styles.hero}>
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className={styles.badge}
        >
          v0.1.0 Available Now — Blazing Fast Local Search
        </motion.div>

        <motion.h1
          className={styles.title}
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.1 }}
        >
          Give your codebase the <br /> context engine it deserves
        </motion.h1>

        <motion.p
          className={styles.subtitle}
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.2 }}
        >
          A universal, dependency-aware search engine built for AI coding
          agents. Written in Rust. Local first. MCP Native.
        </motion.p>

        <motion.div
          className={styles.heroActions}
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.3 }}
        >
          <button className={`${styles.buttonPrimary} ${styles.buttonLarge}`}>
            Install CLI
          </button>
          <button className={`${styles.buttonOutline} ${styles.buttonLarge}`}>
            Read the Docs
          </button>
        </motion.div>

        {/* Hero Visual (Terminal Mockup) */}
        <motion.div
          className={`${styles.heroVisual} ${styles.glassPanel}`}
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.7, delay: 0.4 }}
        >
          <div className={styles.heroCode}>
            <div className={styles.codeHeader}>
              <div className={`${styles.dot} ${styles.dotRed}`}></div>
              <div className={`${styles.dot} ${styles.dotYellow}`}></div>
              <div className={`${styles.dot} ${styles.dotGreen}`}></div>
            </div>
            <pre className={styles.pre}>
              $ <span className={styles.keyword}>omnicontext</span> index
              ./workspace
              <br />
              <span className={styles.comment}>[14:02:12]</span> Building
              semantic chunks...
              <br />
              <span className={styles.comment}>[14:02:13]</span> Generating
              embeddings (ONNX local)...
              <br />
              <span className={styles.comment}>[14:02:14]</span> Computing
              dependency graph...
              <br />
              <span className={styles.success}>
                ✓ Successfully indexed 42,104 files in 2.1s
              </span>
              <br />
              <br />$ <span className={styles.keyword}>
                omnicontext-mcp
              </span>{" "}
              --repo ./workspace
              <br />
              <span className={styles.success}>
                ► OmniContext MCP Server listening on stdio...
              </span>
              <br />
              <span className={styles.comment}>
                {" "}
                Ready to serve advanced code intelligence to your agent.
              </span>
            </pre>
          </div>
        </motion.div>
      </section>

      {/* Features Grid */}
      <section className={styles.featuresSection}>
        <h2 className={styles.sectionTitle}>
          Built differently. Built better.
        </h2>
        <div className={styles.featuresGrid}>
          <motion.div
            className={`${styles.featureCard} ${styles.glassPanel}`}
            whileHover={{ y: -5 }}
          >
            <div className={styles.featureIcon}>
              <Terminal size={32} />
            </div>
            <h3 className={styles.featureTitle}>Blazing Fast Rust Core</h3>
            <p className={styles.featureDesc}>
              Forget slow TypeScript parsers. OmniContext uses Tree-sitter and
              an optimized SQLite+Vector pipeline to index millions of lines of
              code in seconds, locally.
            </p>
          </motion.div>

          <motion.div
            className={`${styles.featureCard} ${styles.glassPanel}`}
            whileHover={{ y: -5 }}
          >
            <div className={styles.featureIcon}>
              <Network size={32} />
            </div>
            <h3 className={styles.featureTitle}>Dependency Graph Fusion</h3>
            <p className={styles.featureDesc}>
              We don't just do semantic search. We build a full dependency graph
              (imports, extends, calls) and fuse signals via Reciprocal Rank
              Fusion (RRF) for precise results.
            </p>
          </motion.div>

          <motion.div
            className={`${styles.featureCard} ${styles.glassPanel}`}
            whileHover={{ y: -5 }}
          >
            <div className={styles.featureIcon}>
              <ShieldCheck size={32} />
            </div>
            <h3 className={styles.featureTitle}>Enterprise Grade Context</h3>
            <p className={styles.featureDesc}>
              API keys, rate-limiting, usage metering, commit lineage, and
              pattern recognition engines. Perfect for deploying internal custom
              agent fleets securely.
            </p>
          </motion.div>
        </div>
      </section>

      {/* Footer */}
      <footer className={styles.footer}>
        <p>© 2026 OmniContext by Mayank. Built with Next.js and Bun.</p>
        <div className={styles.socials}>
          <a
            href="https://github.com/steeltroops-ai/omnicontext"
            className={styles.socialIcon}
          >
            <Github size={20} />
          </a>
        </div>
      </footer>
    </main>
  );
}
