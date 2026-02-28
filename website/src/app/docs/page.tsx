"use client";

import React from "react";
import {
  Terminal,
  Code,
  Cpu,
  ChevronRight,
  Zap,
  FileCode2,
  Command,
} from "lucide-react";
import styles from "./docs.module.css";
import Link from "next/link";

export default function DocsPage() {
  return (
    <div className={styles.contentLayout}>
      {/* Article Content */}
      <div className={styles.contentScroll}>
        <article className={styles.article}>
          <div className={styles.breadcrumbs}>Getting Started</div>

          <h1 className={styles.pageTitle}>Introduction</h1>
          <p className={styles.pageDescription}>
            OmniContext is the developer AI platform that helps you understand
            code, debug issues, and ship faster because it understands your
            codebase. Use Agent, Next Edit, and Code Completions to get more
            done.
          </p>

          {/* IDE Mockup */}
          <div className={styles.mockUiImage}>
            {/* Realistically styled mockup matching the reference image's Augment code view */}
            <div
              style={{
                width: "100%",
                height: "100%",
                display: "flex",
                flexDirection: "column",
              }}
            >
              {/* Header */}
              <div
                style={{
                  background: "#18181b",
                  padding: "0.5rem 1rem",
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  borderBottom: "1px solid #27272a",
                  fontSize: "13px",
                  color: "#a1a1aa",
                }}
              >
                <FileCode2 size={14} /> <span>ringbuffer.test.ts</span>
                <span style={{ flex: 1 }}></span>
                <span
                  style={{
                    background: "#27272a",
                    padding: "2px 8px",
                    borderRadius: "4px",
                    color: "white",
                  }}
                >
                  OMNICONTEXT CHAT
                </span>
              </div>

              {/* Body */}
              <div style={{ flex: 1, display: "flex", background: "#0a0a0c" }}>
                {/* Editor Left */}
                <div
                  style={{
                    flex: 2,
                    padding: "1rem",
                    borderRight: "1px solid #27272a",
                    position: "relative",
                  }}
                >
                  <div
                    style={{
                      position: "absolute",
                      top: "2rem",
                      left: "1rem",
                      background: "rgba(0, 208, 107, 0.15)",
                      padding: "2px 6px",
                      borderRadius: "4px",
                      color: "var(--color-accent)",
                      fontSize: "12px",
                      fontWeight: "bold",
                    }}
                  >
                    Tab to accept suggestion
                  </div>
                  <pre
                    style={{
                      color: "#8b949e",
                      fontSize: "12px",
                      marginTop: "3rem",
                      lineHeight: "1.6",
                    }}
                  >
                    <span style={{ color: "#a1a1aa" }}>193</span>{" "}
                    expect(buffer.removeItem(2)).toBe(
                    <span style={{ color: "#79c0ff" }}>true</span>);
                    <br />
                    <span style={{ color: "#a1a1aa" }}>194</span>{" "}
                    expect(buffer.length).toBe(
                    <span style={{ color: "#a5d6ff" }}>2</span>);
                    <br />
                    <span style={{ color: "#a1a1aa" }}>195</span>{" "}
                    expect(buffer.slice()).toEqual([1, 3]);
                    <br />
                    <span style={{ color: "#a1a1aa" }}>196</span>
                    <br />
                    <span style={{ color: "#a1a1aa" }}>198</span> test("remove
                    non-existent item", () =&gt; {"{"}
                    <br />
                    <span style={{ color: "#a1a1aa" }}>199</span>{" "}
                    <span style={{ color: "#ff7b72" }}>const</span> buffer ={" "}
                    <span style={{ color: "#ff7b72" }}>new</span> RingBuffer(
                    <span style={{ color: "#a5d6ff" }}>5</span>);
                    <br />
                    <span style={{ color: "#a1a1aa" }}>200</span>{" "}
                    buffer.addItem(<span style={{ color: "#a5d6ff" }}>1</span>);
                    <br />
                    <span style={{ color: "#a1a1aa" }}>201</span>{" "}
                    buffer.addItem(<span style={{ color: "#a5d6ff" }}>2</span>);
                    <br />
                    <span style={{ color: "#a1a1aa" }}>202</span> <br />
                    <span style={{ color: "#a1a1aa" }}>203</span>{" "}
                    expect(buffer.removeItem(5)).toBe(
                    <span style={{ color: "#79c0ff" }}>false</span>);
                    <br />
                    <span style={{ color: "#a1a1aa" }}>204</span>{" "}
                    expect(buffer.length).toBe(
                    <span style={{ color: "#a5d6ff" }}>2</span>);
                    <br />
                  </pre>
                </div>

                {/* Chat Right */}
                <div
                  style={{
                    flex: 1,
                    padding: "1rem",
                    display: "flex",
                    flexDirection: "column",
                    gap: "1rem",
                  }}
                >
                  <div
                    style={{
                      alignSelf: "flex-end",
                      background: "#27272a",
                      padding: "0.8rem",
                      borderRadius: "8px",
                      color: "white",
                      fontSize: "13px",
                    }}
                  >
                    You
                    <br />
                    <span
                      style={{
                        color: "#a1a1aa",
                        marginTop: "4px",
                        display: "block",
                      }}
                    >
                      Add a function to remove a specific item from the
                      RingBuffer.
                    </span>
                  </div>
                  <div
                    style={{
                      alignSelf: "flex-start",
                      background: "#09090b",
                      padding: "0.8rem",
                      borderRadius: "8px",
                      border: "1px solid #27272a",
                      color: "white",
                      fontSize: "13px",
                      width: "100%",
                    }}
                  >
                    <div
                      style={{
                        background: "var(--color-accent)",
                        color: "black",
                        display: "inline-block",
                        padding: "2px 8px",
                        borderRadius: "4px",
                        fontSize: "11px",
                        fontWeight: "bold",
                        marginBottom: "8px",
                      }}
                    >
                      OmniContext
                    </div>
                    <p style={{ color: "#d4d4d8", marginBottom: "8px" }}>
                      Certainly! I'll add a function to remove a specific item
                      from the RingBuffer...
                    </p>
                    <div
                      style={{
                        background: "#18181b",
                        padding: "0.5rem",
                        borderRadius: "4px",
                        color: "#ff7b72",
                      }}
                    >
                      <span style={{ color: "#a1a1aa" }}>
                        // implementation
                      </span>
                    </div>
                    <div
                      style={{
                        marginTop: "1rem",
                        background: "#1c1c1c",
                        padding: "0.5rem",
                        borderRadius: "4px",
                        fontSize: "11px",
                      }}
                    >
                      <span style={{ color: "#a1a1aa" }}>ACTIVE CONTEXT</span>
                      <br />
                      <span style={{ color: "#00D06B" }}>
                        @ Repository
                      </span>{" "}
                      source/omnicontext/ringbuffer
                      <br />
                      <span style={{ color: "#00D06B" }}>
                        @ Current File
                      </span>{" "}
                      ringbuffer.test.ts
                      <br />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <h2>Get started in minutes</h2>
          <p>
            OmniContext works with your favorite IDE and your favorite
            programming language. Download the extension, sign in, and get
            coding.
          </p>

          {/* Ask Input */}
          <div className={styles.askInputBox}>
            <input
              type="text"
              placeholder="Ask a question..."
              className={styles.askInput}
            />
            <span
              style={{
                color: "#555",
                fontSize: "0.8rem",
                display: "flex",
                alignItems: "center",
                gap: "4px",
                fontFamily: "var(--font-mono)",
              }}
            >
              Ctrl+I <Command size={14} />
            </span>
          </div>

          {/* 3 Grid Cards */}
          <div className={styles.cardGrid}>
            <div className={styles.card}>
              <div className={styles.cardTitle}>Visual Studio Code</div>
              <p className={styles.cardDesc}>
                Get completions, chat, and instructions in your favorite open
                source editor.
              </p>
            </div>
            <div className={styles.card}>
              <div className={styles.cardTitle}>JetBrains IDEs</div>
              <p className={styles.cardDesc}>
                Completions are available for all JetBrains IDEs, like WebStorm,
                PyCharm, and IntelliJ.
              </p>
            </div>
            <div className={styles.card}>
              <div className={styles.cardTitle}>Omni CLI</div>
              <p className={styles.cardDesc}>
                All the power of OmniContext's agent, context engine, and tools
                in your terminal.
              </p>
            </div>
          </div>

          <h2>Learn more</h2>
          <p>
            Get up to speed, stay in the flow, and get more done. Chat, Next
            Edit, and Code Completions will change the way you build software.
          </p>

          <div className={styles.cardGrid}>
            <div className={styles.card}>
              <Cpu className={styles.cardIcon} size={24} />
              <div className={styles.cardTitle}>Agent</div>
              <p className={styles.cardDesc}>
                Autonomous coding with OmniContext's context engine and tools
                can tackle tasks big and small.
              </p>
            </div>
            <div className={styles.card}>
              <Zap className={styles.cardIcon} size={24} />
              <div className={styles.cardTitle}>Next Edit</div>
              <p className={styles.cardDesc}>
                Keep moving through your tasks by guiding you step-by-step
                through complex or repetitive changes.
              </p>
            </div>
            <div className={styles.card}>
              <Code className={styles.cardIcon} size={24} />
              <div className={styles.cardTitle}>Code Completions</div>
              <p className={styles.cardDesc}>
                Intelligent code suggestions that knows your codebase right at
                your fingertips.
              </p>
            </div>
          </div>

          <div
            style={{
              display: "flex",
              justifyContent: "flex-end",
              marginTop: "2rem",
            }}
          >
            <Link href="/docs/quickstart">
              <span
                className={styles.tocLink}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "0.5rem",
                  fontWeight: 600,
                  color: "white",
                }}
              >
                Quickstart <ChevronRight size={16} />
              </span>
            </Link>
          </div>
        </article>
      </div>

      {/* Right TOC Sidebar */}
      <aside className={styles.tocSidebar}>
        <div className={styles.tocTitle}>On this page</div>
        <ul className={styles.tocList}>
          <li className={styles.tocItem}>
            <a href="#" className={`${styles.tocLink} ${styles.tocLinkActive}`}>
              Get started in minutes
            </a>
          </li>
          <li className={styles.tocItem}>
            <a href="#" className={styles.tocLink}>
              Visual Studio Code
            </a>
          </li>
          <li className={styles.tocItem}>
            <a href="#" className={styles.tocLink}>
              JetBrains IDEs
            </a>
          </li>
          <li className={styles.tocItem}>
            <a href="#" className={styles.tocLink}>
              Omni CLI
            </a>
          </li>
          <li className={styles.tocItem}>
            <a href="#" className={styles.tocLink}>
              Learn more
            </a>
          </li>
        </ul>
      </aside>
    </div>
  );
}
