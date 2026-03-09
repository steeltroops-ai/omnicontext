"use client";

import { motion } from "framer-motion";
import { ChevronRight, Terminal, Zap } from "lucide-react";
import Link from "next/link";
import { siteConfig } from "@/config/site";
import { VERSION, SECTIONS, CONTEXT_ENGINE_LEFT_COLUMN, CONTEXT_ENGINE_RIGHT_COLUMN, CONTEXT_ENGINE_SVG } from "@/config/constants";
import { DottedSurface } from "@/components/ui/dotted-surface";
import { SiteNav } from "@/components/site-nav";
import { SiteFooterFull } from "@/components/site-footer-full";
import { Logo } from "@/components/icons";
import React, { useEffect, useRef } from "react";

const LABELS = [
  "feature-flags.ts",
  "auth/oauth_callback.ts",
  "workspace.controller.ts",
  "auth/api-keys.ts",
  "analytics/events.ts",
  "pipeline/mod.rs",
  "embedder/mod.rs",
  "parser/mod.rs",
  "graph/mod.rs",
  "db/schema.prisma",
  "api/routes.ts",
  "utils/helpers.ts",
  "components/ui.tsx",
  "server/main.go",
  "cache/redis.ts",
  "queue/worker.ts",
  "config/env.ts",
  "models/user.ts",
  "services/email.ts",
  "tests/auth.test.ts",
  "scripts/deploy.sh",
];

const CanvasSphere = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animationFrameId: number;
    let time = 0;
    const numPoints = 600;
    const radius = 280; // 3D sphere radius strictly smaller
    const points: Array<{
      x: number;
      y: number;
      z: number;
      color: string;
      isKey: boolean;
      label?: string;
    }> = [];

    let keyIndex = 0;
    for (let i = 0; i < numPoints; i++) {
      const phi = Math.acos(-1 + (2 * i) / numPoints);
      const theta = Math.sqrt(numPoints * Math.PI) * phi;
      const isKey = i % 25 === 0;
      points.push({
        x: radius * Math.cos(theta) * Math.sin(phi),
        y: radius * Math.sin(theta) * Math.sin(phi),
        z: radius * Math.cos(phi),
        color: isKey ? "rgba(16, 185, 129," : "rgba(100, 100, 100,",
        isKey,
        label: isKey ? LABELS[keyIndex++ % LABELS.length] : undefined,
      });
    }

    // Labels conditionally render for all `isKey` points dynamically.

    const tilt = 0.15;
    const center = 400; // 800x800 canvas center

    let mouseX = -1000;
    let mouseY = -1000;

    const handleMouseMove = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      const scaleX = canvas.width / rect.width;
      const scaleY = canvas.height / rect.height;
      mouseX = (e.clientX - rect.left) * scaleX;
      mouseY = (e.clientY - rect.top) * scaleY;
    };

    const renderPoints = points.map((p) => ({
      ...p,
      rx: 0,
      ry: 0,
      finalZ: 0,
      scale: 0,
    }));

    canvas.addEventListener("mousemove", handleMouseMove);

    const render = () => {
      time += 0.0012;
      ctx.clearRect(0, 0, 800, 800);

      renderPoints.forEach((p, i) => {
        const orig = points[i];

        const rx = orig.x * Math.cos(time) + orig.z * Math.sin(time);
        const rz = -orig.x * Math.sin(time) + orig.z * Math.cos(time);

        const ry = orig.y * Math.cos(tilt) - rz * Math.sin(tilt);
        const finalZ = orig.y * Math.sin(tilt) + rz * Math.cos(tilt);

        const scale = 800 / (800 + finalZ);

        p.rx = rx * scale + center;
        p.ry = ry * scale + center;
        p.finalZ = finalZ;
        p.scale = scale;
      });

      const sortedPoints = [...renderPoints].sort(
        (a, b) => b.finalZ - a.finalZ,
      );
      const keyPoints = sortedPoints.filter((p) => p.isKey);

      ctx.lineWidth = 0.5;
      for (let i = 0; i < keyPoints.length; i++) {
        for (let j = i + 1; j < keyPoints.length; j++) {
          const dx = keyPoints[i].rx - keyPoints[j].rx;
          const dy = keyPoints[i].ry - keyPoints[j].ry;
          const dist = Math.sqrt(dx * dx + dy * dy);
          if (dist < 150) {
            const avgZ = (keyPoints[i].finalZ + keyPoints[j].finalZ) / 2;
            const opacityOffset = avgZ > 0 ? 0.3 : 0.05;
            ctx.strokeStyle = `rgba(16, 185, 129, ${opacityOffset * (1 - dist / 150)})`;
            ctx.beginPath();
            ctx.moveTo(keyPoints[i].rx, keyPoints[i].ry);
            ctx.lineTo(keyPoints[j].rx, keyPoints[j].ry);
            ctx.stroke();
          }
        }
      }

      let hoveredPoint = null;
      let minDistance = 20;

      sortedPoints.forEach((p) => {
        const isFront = p.finalZ > 0;
        const opacity = isFront ? (p.isKey ? 0.9 : 0.45) : p.isKey ? 0.4 : 0.2;
        const size = (p.isKey ? 3.5 : 1.8) * p.scale; // increased point visibility

        const dx = mouseX - p.rx;
        const dy = mouseY - p.ry;
        const dist = Math.sqrt(dx * dx + dy * dy);
        const isHovered = isFront && p.isKey && dist < 20;

        if (isHovered && dist < minDistance) {
          minDistance = dist;
          hoveredPoint = p;
        }

        const isActive = isHovered || isFront;

        ctx.fillStyle = p.color + (isActive && isFront ? "1)" : `${opacity})`);
        ctx.beginPath();
        ctx.arc(
          p.rx,
          p.ry,
          isActive && isFront ? size * 1.5 : size,
          0,
          Math.PI * 2,
        );
        ctx.fill();

        if (p.isKey && isFront) {
          ctx.shadowColor = "rgba(16, 185, 129, 0.9)";
          ctx.shadowBlur = isActive ? 20 : 10;
          ctx.fill();
          ctx.shadowBlur = 0;
        }
      });

      // Draw active labels purely on top in 2D space
      sortedPoints.forEach((p) => {
        // Only draw labels if it's a key point and it is on the FRONT side of the 3D sphere
        if (!p.isKey || p.finalZ < 0) return;

        const isHovered = p === hoveredPoint;

        if (p.label) {
          ctx.font = "11px monospace";
          const textWidth = ctx.measureText(p.label).width;

          const paddingX = 8;
          const boxX = p.rx + 10;
          const boxY = p.ry - 10;

          ctx.fillStyle = "rgba(8, 23, 15, 0.95)";
          ctx.strokeStyle = "rgba(16, 185, 129, 0.4)";
          ctx.lineWidth = 1;

          ctx.beginPath();
          ctx.roundRect(boxX, boxY - 14, textWidth + paddingX * 2 + 10, 20, 4);
          ctx.fill();
          ctx.stroke();

          ctx.fillStyle = "rgba(16, 185, 129, 1)";
          ctx.beginPath();
          ctx.arc(boxX + paddingX + 3, boxY - 4, 3, 0, Math.PI * 2);
          ctx.fill();

          ctx.fillStyle = "rgba(16, 185, 129, 0.9)";
          ctx.fillText(p.label, boxX + paddingX + 10, boxY - 1);
        }
      });

      animationFrameId = requestAnimationFrame(render);
    };

    render();

    return () => {
      cancelAnimationFrame(animationFrameId);
      canvas.removeEventListener("mousemove", handleMouseMove);
    };
  }, []);

  return (
    <div className="relative w-full aspect-square max-w-[600px] flex items-center justify-center mx-auto">
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[100%] h-[100%] scale-[0.8] bg-emerald-500/5 blur-[80px] rounded-full pointer-events-none z-0" />
      <canvas
        ref={canvasRef}
        width={800}
        height={800}
        className="w-full h-full object-contain cursor-crosshair z-20 relative"
      />
    </div>
  );
};

export default function Home() {
  return (
    <>
      <style jsx global>{`
        @keyframes spin-slow {
          from {
            transform: rotate(0deg);
          }
          to {
            transform: rotate(360deg);
          }
        }
        .animate-spin-slow {
          animation: spin-slow 20s linear infinite;
        }
      `}</style>
      <div className="flex flex-col min-h-screen bg-[#09090B] selection:bg-primary/30">
        <SiteNav transparent />

        <main className="flex-1 flex flex-col pt-0">
          <section className="relative w-full flex items-center min-h-screen pt-14 pb-16 overflow-hidden">
            <DottedSurface className="absolute inset-0 z-0 opacity-50" />

            {/* Subtle background glow */}
            <div className="absolute top-[30%] left-[-10%] w-[600px] h-[500px] bg-primary/10 blur-[130px] rounded-full pointer-events-none" />
            <div className="absolute bottom-[-10%] right-[-10%] w-[800px] h-[600px] bg-emerald-500/5 blur-[150px] rounded-full pointer-events-none" />

            <div className="relative z-10 flex flex-col lg:flex-row items-center lg:items-center justify-between px-4 sm:px-6 md:px-12 lg:px-16 w-full max-w-[1400px] mx-auto gap-8 lg:gap-12 xl:gap-16 py-12 lg:py-0">
              {/* Left Content */}
              <div className="flex-1 flex flex-col items-center lg:items-start text-center lg:text-left w-full max-w-[560px] mx-auto lg:mx-0">
                <motion.div
                  initial={{ opacity: 0, y: 15 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.8, ease: [0.16, 1, 0.1, 1] }}
                  className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-900/50 border border-white/5 text-[12px] font-medium text-zinc-300 mb-6 backdrop-blur-md cursor-pointer hover:bg-zinc-800/50 transition-colors"
                >
                  <div className="w-1.5 h-1.5 rounded-full bg-primary animate-pulse" />
                  <span>{VERSION.displayFormat} is now available</span>
                  <ChevronRight size={13} className="text-zinc-500" />
                </motion.div>

                <motion.h1
                  className="text-3xl sm:text-4xl md:text-5xl lg:text-6xl xl:text-[68px] font-semibold tracking-tighter text-transparent bg-clip-text bg-gradient-to-b from-white to-white/70 mb-5 leading-[1.1]"
                  initial={{ opacity: 0, scale: 0.96, filter: "blur(10px)" }}
                  animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
                  transition={{
                    duration: 1.2,
                    ease: [0.16, 1, 0.1, 1],
                    delay: 0.1,
                  }}
                >
                  The context engine <br className="hidden md:block" /> your
                  codebase deserves.
                </motion.h1>

                <motion.p
                  className="text-[15px] sm:text-[16px] md:text-[17px] lg:text-[18px] text-zinc-400 max-w-[580px] mb-8 leading-relaxed tracking-tight"
                  initial={{ opacity: 0, y: 15 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{
                    duration: 0.8,
                    ease: [0.16, 1, 0.1, 1],
                    delay: 0.2,
                  }}
                >
                  {siteConfig.name} represents a fundamental shift in AI coding.
                  Universal dependency awareness, written in Rust, and executed
                  flawlessly on your local machine.
                </motion.p>

                <motion.div
                  className="flex flex-col sm:flex-row gap-2 w-full sm:w-auto justify-center lg:justify-start px-4 sm:px-0"
                  initial={{ opacity: 0, y: 15 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{
                    duration: 0.8,
                    ease: [0.16, 1, 0.1, 1],
                    delay: 0.3,
                  }}
                >
                  <Link
                    href="https://marketplace.visualstudio.com/items?itemName=steeltroops.omnicontext"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="w-full sm:w-auto px-6 sm:px-5 py-2.5 text-[13px] sm:text-[14px] font-medium rounded-full bg-zinc-100 text-black hover:scale-105 active:scale-95 transition-all duration-300 shadow-[0_0_40px_rgba(255,255,255,0.1)] flex items-center justify-center whitespace-nowrap"
                  >
                    Install Extension
                  </Link>
                  <Link
                    href="/docs"
                    className="w-full sm:w-auto px-6 sm:px-5 py-2.5 text-[13px] sm:text-[14px] font-medium rounded-full bg-zinc-900 text-white border border-white/10 hover:bg-zinc-800 transition-colors duration-300 flex items-center justify-center whitespace-nowrap"
                  >
                    Read Docs
                  </Link>
                </motion.div>
              </div>

              {/* Right Hero Visual (Elegant Dark-Glass Terminal) */}
              <motion.div
                className="flex-[1.3] w-full relative z-10 hidden lg:flex flex-col"
                initial={{ opacity: 0, x: 40 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{
                  duration: 1.2,
                  ease: [0.16, 1, 0.3, 1],
                  delay: 0.4,
                }}
              >
                {/* True liquid glass — near-zero opacity, minimal blur */}
                <div
                  className="w-full relative rounded-2xl overflow-hidden flex flex-col group transform-gpu"
                  style={{
                    background: "rgba(255, 255, 255, 0.02)",
                    border: "1px solid rgba(255,255,255,0.08)",
                    boxShadow:
                      "0 12px 40px rgba(0,0,0,0.15), 0 0 0 0.5px rgba(255,255,255,0.04) inset, 0 1px 0 rgba(255,255,255,0.07) inset",
                    backdropFilter: "blur(8px)",
                    WebkitBackdropFilter: "blur(8px)",
                  }}
                >
                  {/* Interior ambient color tints */}
                  <div className="absolute top-[-20%] left-[-5%] w-[55%] h-[55%] bg-emerald-500/[0.07] blur-[90px] pointer-events-none rounded-full" />
                  <div className="absolute bottom-[-15%] right-[0%] w-[45%] h-[50%] bg-indigo-500/[0.07] blur-[90px] pointer-events-none rounded-full" />

                  {/* Top specular shine — glass refraction line */}
                  <div className="absolute top-0 left-0 w-full h-px bg-gradient-to-r from-transparent via-white/20 to-transparent pointer-events-none" />
                  {/* Left edge shine */}
                  <div className="absolute top-0 left-0 w-px h-full bg-gradient-to-b from-white/[0.12] to-transparent pointer-events-none" />

                  {/* Window Chrome — same liquid glass treatment */}
                  <div
                    className="w-full h-10 flex items-center justify-between px-5 relative z-20 border-b"
                    style={{
                      borderColor: "rgba(255,255,255,0.06)",
                      background: "rgba(255,255,255,0.03)",
                    }}
                  >
                    <div className="flex gap-2">
                      <div className="w-2.5 h-2.5 rounded-full bg-[#FF5F56]/80 shadow-[0_0_6px_rgba(255,95,86,0.3)]" />
                      <div className="w-2.5 h-2.5 rounded-full bg-[#FFBD2E]/80 shadow-[0_0_6px_rgba(255,189,46,0.3)]" />
                      <div className="w-2.5 h-2.5 rounded-full bg-[#27C93F]/80 shadow-[0_0_6px_rgba(39,201,63,0.3)]" />
                    </div>
                    <div className="flex items-center gap-1.5 text-[10px] font-mono font-medium text-zinc-500 uppercase tracking-widest">
                      <Terminal size={9} />
                      <span>omni — daemon</span>
                    </div>
                    <div className="w-[60px]" />
                  </div>

                  {/* Window Content - Fixed height with scrollable content */}
                  <div className="w-full h-[420px] overflow-y-auto overflow-x-hidden custom-scrollbar p-6 md:p-8 text-left relative z-10 font-mono text-[13px] leading-[1.9] text-zinc-300 tracking-tight">
                    {/* Command 1 */}
                    <div className="flex items-center gap-3 mb-4 font-semibold text-zinc-200">
                      <span className="text-emerald-500 shrink-0">❯</span>
                      <span className="truncate">omnicontext index .</span>
                    </div>

                    <div className="pl-4 border-l-2 border-emerald-500/20 ml-[5px] flex flex-col gap-2.5 my-5 text-[12px]">
                      <div className="text-zinc-500 text-[11px] mb-1">
                        OmniContext - Indexing: /workspace/omnicontext
                      </div>

                      <div className="flex items-start gap-3">
                        <span className="text-zinc-600 shrink-0 w-[90px]">
                          parser
                        </span>
                        <span className="text-zinc-400 flex-1">
                          AST extraction (16 languages)
                        </span>
                        <span className="text-emerald-500 font-medium text-right w-[50px]">
                          142ms
                        </span>
                      </div>

                      <div className="flex items-start gap-3">
                        <span className="text-zinc-600 shrink-0 w-[90px]">
                          chunker
                        </span>
                        <span className="text-zinc-400 flex-1">
                          Semantic boundaries + tokens
                        </span>
                        <span className="text-emerald-500 font-medium text-right w-[50px]">
                          89ms
                        </span>
                      </div>

                      <div className="flex items-start gap-3">
                        <span className="text-zinc-600 shrink-0 w-[90px]">
                          embedder
                        </span>
                        <span className="text-zinc-400 flex-1">
                          ONNX batch inference (jina-v2)
                        </span>
                        <span className="text-emerald-500 font-medium text-right w-[50px]">
                          1.24s
                        </span>
                      </div>

                      <div className="flex items-start gap-3">
                        <span className="text-zinc-600 shrink-0 w-[90px]">
                          vector_index
                        </span>
                        <span className="text-zinc-400 flex-1">
                          HNSW index construction
                        </span>
                        <span className="text-emerald-500 font-medium text-right w-[50px]">
                          267ms
                        </span>
                      </div>

                      <div className="flex items-start gap-3">
                        <span className="text-zinc-600 shrink-0 w-[90px]">
                          metadata
                        </span>
                        <span className="text-zinc-400 flex-1">
                          SQLite FTS5 + dependency graph
                        </span>
                        <span className="text-emerald-500 font-medium text-right w-[50px]">
                          178ms
                        </span>
                      </div>

                      <div className="border-t border-white/5 mt-3 pt-3 text-zinc-400">
                        <div className="flex justify-between mb-1.5">
                          <span>Files processed:</span>
                          <span className="text-zinc-300">1,247</span>
                        </div>
                        <div className="flex justify-between mb-1.5">
                          <span>Chunks created:</span>
                          <span className="text-zinc-300">8,932</span>
                        </div>
                        <div className="flex justify-between mb-1.5">
                          <span>Symbols extracted:</span>
                          <span className="text-zinc-300">4,156</span>
                        </div>
                        <div className="flex justify-between">
                          <span>Embeddings:</span>
                          <span className="text-zinc-300">8,932</span>
                        </div>
                      </div>

                      <div className="text-primary font-semibold tracking-wide mt-3 flex items-center gap-2">
                        <Zap
                          size={14}
                          className="fill-primary/20 text-primary"
                        />
                        Indexing complete in 2.08s
                      </div>
                    </div>

                    {/* Command 2 */}
                    <div className="mt-8 flex items-center gap-3 mb-4 font-semibold text-zinc-200">
                      <span className="text-emerald-500 shrink-0">❯</span>
                      <span className="truncate">omnicontext mcp --repo .</span>
                    </div>

                    <div className="pl-4 border-l-2 border-indigo-500/20 ml-[5px] flex flex-col gap-2 my-5 text-[12px]">
                      <div className="text-zinc-500 text-[11px] mb-1">
                        OmniContext MCP Server starting...
                      </div>
                      <div className="text-indigo-400 font-semibold tracking-wide flex items-center gap-3">
                        <div className="w-2.5 h-2.5 rounded-full bg-indigo-500 animate-[pulse_2s_ease-in-out_infinite] shadow-[0_0_12px_rgba(99,102,241,0.6)]" />
                        Server active on stdio transport
                      </div>
                      <div className="text-zinc-500 mt-1">
                        Repository: /workspace/omnicontext
                      </div>
                      <div className="text-zinc-600 text-[11px] mt-2">
                        Ready to serve 8 MCP tools to AI agents
                      </div>
                    </div>
                  </div>
                </div>
              </motion.div>
            </div>
          </section>

          {/* Feature Sections - Alternating Interactive Blocks */}
          <section className="py-20 md:py-24 w-full max-w-[1400px] mx-auto flex flex-col gap-[180px] px-8 md:px-16">
            {/* Block 1: Context Engine - Enterprise Visualization */}
            <div className="flex flex-col gap-12">
              {/* Header */}
              <div className="text-center max-w-3xl mx-auto">
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  {SECTIONS.contextEngine.badge}
                </div>
                <h3 className="text-3xl md:text-5xl font-semibold text-white tracking-tight mb-5 leading-tight">
                  {SECTIONS.contextEngine.title}
                </h3>
                <p className="text-[16px] md:text-[18px] text-zinc-400 leading-relaxed">
                  {SECTIONS.contextEngine.description}
                </p>
              </div>

              {/* Main Visualization Container */}
              <div className="relative w-full rounded-2xl border border-white/10 bg-gradient-to-b from-white/[0.03] to-transparent backdrop-blur-3xl overflow-hidden shadow-2xl h-auto lg:h-[700px] flex">
                <div className="relative z-10 flex flex-col lg:flex-row w-full p-4 sm:p-6 lg:p-12 lg:px-16 min-h-full items-center lg:items-stretch justify-between gap-6 lg:gap-0">
                  {/* Left SVG overlay for connecting lines */}
                  <div className="absolute top-0 bottom-0 left-[204px] w-[calc(50%-204px-240px)] pointer-events-none hidden lg:block z-0">
                    <div className="absolute top-[348px] right-0 w-[4px] h-[4px] bg-zinc-500 rounded-full shadow-[0_0_8px_rgba(255,255,255,0.3)] shrink-0 z-10 translate-x-1/2" />
                    <svg
                      className="w-full h-full relative z-0"
                      viewBox="0 0 100 700"
                      preserveAspectRatio="none"
                      xmlns="http://www.w3.org/2000/svg"
                    >
                      {CONTEXT_ENGINE_LEFT_COLUMN.map((item, i) => (
                        <path
                          key={`l-${i}`}
                          d={`M 0 ${item.y + CONTEXT_ENGINE_SVG.leftPathOffset} C 60 ${item.y + CONTEXT_ENGINE_SVG.leftPathOffset}, 60 ${CONTEXT_ENGINE_SVG.centerConnectionY}, 100 ${CONTEXT_ENGINE_SVG.centerConnectionY}`}
                          fill="none"
                          stroke="rgba(255,255,255,0.08)"
                          strokeWidth="1.5"
                          vectorEffect="non-scaling-stroke"
                        />
                      ))}
                    </svg>
                  </div>

                  {/* Right SVG overlay for connecting lines */}
                  <div className="absolute top-0 bottom-0 right-[204px] w-[calc(50%-204px-240px)] pointer-events-none hidden lg:block z-0">
                    <div className="absolute top-[350px] left-0 w-[4px] h-[4px] bg-emerald-500 rounded-full shadow-[0_0_8px_rgba(16,185,129,0.5)] shrink-0 z-10 -translate-x-1/2" />
                    <svg
                      className="w-full h-full relative z-0"
                      viewBox="0 0 100 700"
                      preserveAspectRatio="none"
                      xmlns="http://www.w3.org/2000/svg"
                    >
                      {CONTEXT_ENGINE_RIGHT_COLUMN.map((item, i) => (
                        <path
                          key={`r-${i}`}
                          d={`M 100 ${item.y + CONTEXT_ENGINE_SVG.rightPathOffset} C 40 ${item.y + CONTEXT_ENGINE_SVG.rightPathOffset}, 40 ${CONTEXT_ENGINE_SVG.centerConnectionY}, 0 ${CONTEXT_ENGINE_SVG.centerConnectionY}`}
                          fill="none"
                          stroke="rgba(16,185,129,0.3)"
                          strokeWidth="1.5"
                          vectorEffect="non-scaling-stroke"
                        />
                      ))}
                    </svg>
                  </div>

                  {/* Left Column */}
                  <div className="flex flex-col z-10 w-full lg:w-[140px] flex-none h-auto lg:h-full relative order-1 lg:order-1 items-center lg:items-start text-center lg:text-left mt-2 lg:mt-0">
                    <h4 className="lg:absolute lg:top-0 w-full text-[11px] uppercase tracking-[0.15em] text-zinc-500/80 font-mono mb-4 lg:mb-0">
                      REALTIME RAW CONTEXT
                    </h4>
                    <div className="lg:absolute w-full flex flex-row flex-wrap justify-center lg:flex-col h-auto gap-x-4 gap-y-2 lg:gap-0" style={{ top: `${CONTEXT_ENGINE_SVG.leftColumnTop}px` }}>
                      {CONTEXT_ENGINE_LEFT_COLUMN.map((item, i) => (
                        <div
                          key={i}
                          style={{ top: `${item.y}px` }}
                          className="relative lg:absolute w-auto lg:w-full flex items-center justify-between group h-auto lg:h-[20px]"
                        >
                          <span className="text-[11px] font-mono text-zinc-400 whitespace-nowrap">
                            {item.label}
                          </span>
                          <div className="flex items-center gap-[6px] opacity-40 bg-white/[0.02] backdrop-blur-sm pl-2 z-10 hidden lg:flex">
                            <div className="w-[3px] h-[3px] rounded-full bg-zinc-600" />
                            <div className="w-[3px] h-[3px] rounded-full bg-zinc-600" />
                            <div className="w-[3px] h-[3px] rounded-full bg-zinc-600" />
                            <div className="w-[4px] h-[4px] rounded-full bg-zinc-400" />
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>

                  {/* Mobile Top SVG Overlay */}
                  <div className="w-full h-[30px] lg:hidden order-2 relative z-0 mt-2">
                    <div className="absolute bottom-0 left-1/2 w-[4px] h-[4px] bg-zinc-500 rounded-full shadow-[0_0_8px_rgba(255,255,255,0.3)] shrink-0 z-10 -translate-x-1/2" />
                    <svg
                      className="w-full h-full relative z-0"
                      viewBox="0 0 100 30"
                      preserveAspectRatio="none"
                      xmlns="http://www.w3.org/2000/svg"
                    >
                      {[10, 26, 42, 58, 74, 90].map((x, i) => (
                        <path
                          key={i}
                          d={`M ${x} 0 C ${x} 15, 50 15, 50 30`}
                          fill="none"
                          stroke="rgba(255,255,255,0.15)"
                          strokeWidth="1.5"
                          vectorEffect="non-scaling-stroke"
                        />
                      ))}
                    </svg>
                  </div>

                  {/* Center Column: Sphere */}
                  <div className="relative flex flex-col items-center justify-center flex-1 w-full lg:max-w-[calc(100%-280px)] h-auto lg:h-full order-3 lg:order-2 mt-2 lg:mt-0">
                    <h4 className="text-[11px] uppercase tracking-[0.15em] text-zinc-500/80 font-mono mb-2 lg:mb-0 text-center lg:absolute lg:top-0 hidden lg:block">
                      SEMANTIC UNDERSTANDING
                    </h4>

                    {/* 3D Sphere Container */}
                    <div className="relative w-full aspect-square max-w-[700px] flex items-center justify-center pt-0 lg:pt-0 overflow-visible">
                      <CanvasSphere />
                    </div>
                  </div>

                  {/* Mobile Bottom SVG Overlay */}
                  <div className="w-full h-[30px] lg:hidden order-4 relative z-0 mb-2">
                    <div className="absolute top-0 left-1/2 w-[4px] h-[4px] bg-emerald-500 rounded-full shadow-[0_0_8px_rgba(16,185,129,0.5)] shrink-0 z-10 -translate-x-1/2" />
                    <svg
                      className="w-full h-full relative z-0"
                      viewBox="0 0 100 30"
                      preserveAspectRatio="none"
                      xmlns="http://www.w3.org/2000/svg"
                    >
                      {[20, 40, 60, 80].map((x, i) => (
                        <path
                          key={i}
                          d={`M 50 0 C 50 15, ${x} 15, ${x} 30`}
                          fill="none"
                          stroke="rgba(16,185,129,0.3)"
                          strokeWidth="1.5"
                          vectorEffect="non-scaling-stroke"
                        />
                      ))}
                    </svg>
                  </div>

                  {/* Right Column */}
                  <div className="flex flex-col z-10 w-full lg:w-[140px] flex-none h-auto lg:h-full relative order-5 lg:order-3 items-center lg:items-end text-center lg:text-right mb-2 lg:mt-0">
                    <h4 className="lg:absolute lg:top-0 w-full text-[11px] uppercase tracking-[0.15em] text-zinc-500/80 font-mono text-center lg:text-right mb-4 lg:mb-0">
                      CURATED CONTEXT
                    </h4>
                    <div className="lg:absolute w-full flex flex-row flex-wrap justify-center lg:flex-col h-auto gap-x-4 gap-y-2 lg:gap-0" style={{ top: `${CONTEXT_ENGINE_SVG.rightColumnTop}px` }}>
                      {CONTEXT_ENGINE_RIGHT_COLUMN.map((item, i) => (
                        <div
                          key={i}
                          style={{ top: `${item.y}px` }}
                          className="relative lg:absolute w-auto lg:w-full flex items-center justify-between group h-auto lg:h-[20px]"
                        >
                          <div className="flex items-center gap-[6px] opacity-40 bg-white/[0.02] backdrop-blur-sm pr-2 z-10 hidden lg:flex">
                            <div className="w-[4px] h-[4px] rounded-full bg-emerald-400" />
                            <div className="w-[3px] h-[3px] rounded-full bg-emerald-600" />
                            <div className="w-[3px] h-[3px] rounded-full bg-emerald-600" />
                            <div className="w-[3px] h-[3px] rounded-full bg-emerald-600" />
                          </div>
                          <span className="text-[11px] font-mono text-zinc-400 whitespace-nowrap">
                            {item.label}
                          </span>
                        </div>
                      ))}
                    </div>

                    <div className="flex flex-col relative lg:absolute bottom-0 lg:bottom-[48px] w-full mt-10 lg:mt-0 items-center lg:items-end text-center lg:text-right">
                      <div className="text-[9px] font-mono text-zinc-500 tracking-tight">
                        4,456 sources → 682 relevant
                      </div>
                      <div className="w-full max-w-[200px] h-[2px] bg-zinc-800 rounded-full overflow-hidden mt-2 mb-2">
                        <div className="w-[15%] h-full bg-emerald-500 rounded-full shadow-[0_0_8px_rgba(16,185,129,0.5)]" />
                      </div>
                      <div className="text-[9px] font-mono text-zinc-600/70 tracking-tight w-full max-w-[200px] text-right">
                        Fig. 1.1
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* Bottom Feature Cards */}
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
                {[
                  {
                    title: "Multi-Language Support",
                    desc: "Tree-sitter AST parsing for 16+ languages including Python, TypeScript, Rust, Go, Java, and C++ with unified semantic extraction",
                  },
                  {
                    title: "Dependency-Aware",
                    desc: "Cross-file relationship mapping with import resolution, symbol tracking, and architectural context preservation",
                  },
                  {
                    title: "Real-Time Updates",
                    desc: "File system watching with incremental indexing, hash-based change detection, and sub-second propagation latency",
                  },
                  {
                    title: "Enterprise Scale",
                    desc: "Connection pooling, circuit breakers, health monitoring, and graceful degradation for production deployments",
                  },
                ].map((item, idx) => (
                  <div
                    key={idx}
                    className="bg-white/[0.02] border border-white/5 rounded-xl p-6 hover:border-emerald-500/30 transition-all"
                  >
                    <h5 className="text-[13px] font-semibold text-white mb-3">
                      {item.title}
                    </h5>
                    <p className="text-[12px] text-zinc-500 leading-relaxed">
                      {item.desc}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          </section>

          {/* Feature Sections - Alternating Interactive Blocks */}
          <section className="py-20 md:py-24 w-full max-w-[1400px] mx-auto flex flex-col gap-[180px] px-8 md:px-16">
            {/* Block 1: Hybrid Retrieval */}
            <div className="flex flex-col md:flex-row items-stretch gap-16">
              <div className="flex-1 flex flex-col justify-center py-2">
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Hybrid Search Engine
                </div>
                <h3 className="text-3xl md:text-4xl font-semibold text-white tracking-tight mb-5 leading-tight">
                  Best-in-class retrieval precision.
                </h3>
                <p className="text-[16px] text-zinc-400 leading-relaxed mb-8">
                  OmniContext does not rely on simple vector lookups. We fuse
                  dense semantic vectors (usearch HNSW) with sparse exact-match
                  keywords (SQLite FTS5) via Reciprocal Rank Fusion (RRF),
                  ensuring your agents get exactly the context they need without
                  hallucinating non-existent APIs.
                </p>
                <ul className="flex flex-col gap-3">
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                    usearch HNSW vector index
                  </li>
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                    SQLite FTS5 for exact keyword matches
                  </li>
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" /> RRF
                    fusion + heuristic boosting
                  </li>
                </ul>
                <Link
                  href="/docs"
                  className="inline-flex items-center gap-2 mt-8 text-[14px] text-emerald-500 hover:text-emerald-400 font-medium transition-colors"
                >
                  Learn about Hybrid Search <ChevronRight size={14} />
                </Link>
              </div>
              <div className="flex-[1.2] w-full relative flex flex-col py-4">
                <div className="w-full h-full min-h-[340px] bg-gradient-to-b from-white/[0.03] to-transparent border border-white/10 rounded-2xl shadow-2xl overflow-hidden flex flex-col backdrop-blur-3xl group">
                  {/* Refined MacOS Header */}
                  <div className="flex flex-row items-center px-5 h-11 border-b border-white/5 bg-white/[0.02] relative">
                    <div className="flex gap-2">
                      <div className="w-2.5 h-2.5 rounded-full bg-[#FF5F56] opacity-80" />
                      <div className="w-2.5 h-2.5 rounded-full bg-[#FFBD2E] opacity-80" />
                      <div className="w-2.5 h-2.5 rounded-full bg-[#27C93F] opacity-80" />
                    </div>
                    <div className="absolute left-1/2 -translate-x-1/2 text-[11px] font-medium text-zinc-500 font-mono tracking-tight opacity-70">
                      omni-core hybrid-search
                    </div>
                  </div>

                  {/* Terminal Content */}
                  <div className="flex-1 p-6 pt-5 font-mono text-[13px] text-zinc-400 flex flex-col gap-4 relative overflow-hidden">
                    <div className="absolute top-0 right-0 w-64 h-64 bg-emerald-500/10 blur-[100px] pointer-events-none rounded-full" />

                    <div className="flex flex-col gap-2 relative z-10 text-[12px] leading-[1.8]">
                      <div className="text-emerald-400 font-bold text-[14px] mb-2 tracking-tight flex items-center gap-2 relative z-10">
                        <span className="text-emerald-500 opacity-60">❯</span>{" "}
                        omni search --query "auth middleware"
                      </div>
                      <div>
                        <span className="text-zinc-500 font-bold">[1]</span>{" "}
                        Executing dense search (ONNX embedding)...{" "}
                        <span className="text-emerald-400">12ms</span>
                      </div>
                      <div>
                        <span className="text-zinc-500 font-bold">[2]</span>{" "}
                        Executing sparse search (FTS5)...{" "}
                        <span className="text-emerald-400">4ms</span>
                      </div>
                      <div>
                        <span className="text-zinc-500 font-bold">[3]</span>{" "}
                        Applying Reciprocal Rank Fusion...
                      </div>
                    </div>

                    <div className="pl-4 border-l-2 border-white/10 ml-1 mt-1 text-zinc-300 relative z-10 flex flex-col gap-3 font-mono text-[11px]">
                      <div className="text-zinc-600 text-[10px] mb-1 font-sans uppercase tracking-[0.2em] font-bold">
                        Top Context Matches
                      </div>
                      <div className="flex items-center justify-between gap-4">
                        <div className="truncate flex-1">
                          <span className="text-emerald-500 mr-2">*</span>{" "}
                          src/middleware/auth.rs
                        </div>
                        <span className="text-zinc-500 text-[10px] border border-white/10 px-1.5 py-0.5 rounded tracking-tighter">
                          RRF: 0.0331
                        </span>
                      </div>
                      <div className="flex items-center justify-between gap-4">
                        <div className="truncate flex-1">
                          <span className="text-emerald-500 mr-2">*</span>{" "}
                          tests/auth_integration.rs
                        </div>
                        <span className="text-zinc-500 text-[10px] border border-white/10 px-1.5 py-0.5 rounded tracking-tighter">
                          RRF: 0.0325
                        </span>
                      </div>
                      <div className="flex items-center justify-between gap-4">
                        <div className="truncate flex-1">
                          <span className="text-emerald-500 mr-2">*</span>{" "}
                          docs/api/authentication.md
                        </div>
                        <span className="text-zinc-500 text-[10px] border border-white/10 px-1.5 py-0.5 rounded tracking-tighter">
                          RRF: 0.0150
                        </span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Block 2: Dependency Graph */}
            <div className="flex flex-col md:flex-row-reverse items-stretch gap-16">
              <div className="flex-1 flex flex-col justify-center py-2">
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Local-First Performance
                </div>
                <h3 className="text-3xl md:text-4xl font-semibold text-white tracking-tight mb-5 leading-tight">
                  Your code never leaves your hardware.
                </h3>
                <p className="text-[16px] text-zinc-400 leading-relaxed mb-8">
                  OmniContext is a zero-latency, local-first engine. By
                  executing entirely on your machine with a highly parallel Rust
                  backend and local ONNX embeddings, we ensure your code stays
                  private and your agents stay fast—no cloud dependencies
                  required.
                </p>
                <ul className="flex flex-col gap-3">
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" /> 100%
                    local ONNX model inference
                  </li>
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                    Parallelized indexing (10k files &lt; 60s)
                  </li>
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                    Contextual chunking with semantic boundaries
                  </li>
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                    Batch embedding with intelligent backpressure
                  </li>
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                    Connection pooling for concurrent access
                  </li>
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                    Encrypted SQLite index with WAL concurrency
                  </li>
                </ul>
                <Link
                  href="/docs"
                  className="inline-flex items-center gap-2 mt-8 text-[14px] text-emerald-500 hover:text-emerald-400 font-medium transition-colors"
                >
                  Performance Benchmarks <ChevronRight size={14} />
                </Link>
              </div>
              <div className="flex-[1.2] w-full relative flex flex-col py-4">
                <div className="w-full h-full min-h-[340px] bg-gradient-to-b from-white/[0.03] to-transparent border border-white/10 rounded-2xl shadow-2xl overflow-hidden flex flex-col backdrop-blur-3xl group">
                  {/* Refined MacOS Header */}
                  <div className="flex flex-row items-center px-5 h-11 border-b border-white/5 bg-white/[0.02] relative">
                    <div className="flex gap-2">
                      <div className="w-2.5 h-2.5 rounded-full bg-[#FF5F56] opacity-80" />
                      <div className="w-2.5 h-2.5 rounded-full bg-[#FFBD2E] opacity-80" />
                      <div className="w-2.5 h-2.5 rounded-full bg-[#27C93F] opacity-80" />
                    </div>
                    <div className="absolute left-1/2 -translate-x-1/2 text-[11px] font-medium text-emerald-400/40 font-mono tracking-tight opacity-70">
                      omnicontext-engine — status
                    </div>
                  </div>

                  {/* Terminal Content */}
                  <div className="flex-1 p-6 pt-5 font-mono text-[13px] leading-[1.8] flex flex-col relative overflow-hidden">
                    <div className="absolute top-0 left-0 w-full h-32 bg-emerald-500/10 blur-[100px] pointer-events-none" />
                    <div className="text-emerald-400 font-bold text-[14px] mb-5 tracking-tight flex items-center gap-2 relative z-10">
                      <span className="text-emerald-500 opacity-60">❯</span>{" "}
                      omni status --verbose
                    </div>

                    <div className="relative z-10 mb-6 flex flex-col gap-2.5">
                      <div className="text-[10px] text-zinc-500 uppercase tracking-widest font-sans font-bold">
                        System Telemetry
                      </div>
                      <div className="flex flex-col gap-1.5 border-l-2 border-emerald-500/20 pl-4">
                        <div className="flex justify-between items-center text-zinc-400">
                          <span>Binary Runtime</span>
                          <span className="text-zinc-200">Rust / Static</span>
                        </div>
                        <div className="flex justify-between items-center text-zinc-400">
                          <span>Memory RSS</span>
                          <span className="text-zinc-200">84 MB</span>
                        </div>
                        <div className="flex justify-between items-center text-zinc-400">
                          <span>Model Latency</span>
                          <span className="text-zinc-200">14ms (CPU)</span>
                        </div>
                      </div>
                    </div>

                    <div className="relative z-10 flex flex-col gap-2.5">
                      <div className="text-[10px] text-zinc-500 uppercase tracking-widest font-sans font-bold">
                        Index Integrity
                      </div>
                      <div className="flex flex-col gap-1.5 border-l-2 border-primary/20 pl-4">
                        <div className="flex items-center gap-2">
                          <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(34,197,94,0.5)]" />
                          <span className="text-zinc-300">
                            SQLite FTS5 (Fossilized)
                          </span>
                        </div>
                        <div className="flex items-center gap-2">
                          <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(34,197,94,0.5)]" />
                          <span className="text-zinc-300">
                            HNSW Vector Index (MMAP)
                          </span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Block 3: Native MCP */}
            <div className="flex flex-col md:flex-row items-stretch gap-16">
              <div className="flex-1 flex flex-col justify-center py-2">
                <div className="text-[11px] uppercase tracking-widest text-emerald-500 font-semibold mb-4">
                  Agent Protocol
                </div>
                <h3 className="text-3xl md:text-4xl font-semibold text-white tracking-tight mb-5 leading-tight">
                  Native MCP Server Integration.
                </h3>
                <p className="text-[16px] text-zinc-400 leading-relaxed mb-8">
                  OmniContext does not try to be an AI agent; it empowers the
                  ones you already use. It runs fully locally as a standard
                  Model Context Protocol (MCP) server over `stdio` or `sse`.
                </p>
                <ul className="flex flex-col gap-3">
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                    Provides 8 powerful MCP tools
                  </li>
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                    Connects to Claude Code &amp; Cursor
                  </li>
                  <li className="flex items-center gap-3 text-[14px] text-zinc-300 tracking-tight">
                    <div className="w-1 h-1 rounded-full bg-emerald-500" />{" "}
                    Stdio and HTTP SSE transports
                  </li>
                </ul>
                <Link
                  href="/docs"
                  className="inline-flex items-center gap-2 mt-8 text-[14px] text-emerald-500 hover:text-emerald-400 font-medium transition-colors"
                >
                  View MCP Configuration <ChevronRight size={14} />
                </Link>
              </div>
              <div className="flex-[1.2] w-full relative flex flex-col py-4">
                {/* VS Code / IDE Editor Mockup */}
                <div className="w-full h-full min-h-[340px] bg-gradient-to-b from-white/[0.03] to-transparent border border-white/10 rounded-2xl shadow-2xl overflow-hidden font-sans flex flex-col backdrop-blur-3xl group">
                  {/* Editor Tabs & Controls */}
                  <div className="flex items-center h-11 border-b border-white/5 bg-white/[0.02] select-none relative">
                    <div className="flex items-center gap-2 px-5 h-full border-r border-white/5">
                      <div className="w-2.5 h-2.5 rounded-full bg-[#FF5F56] opacity-80" />
                      <div className="w-2.5 h-2.5 rounded-full bg-[#FFBD2E] opacity-80" />
                      <div className="w-2.5 h-2.5 rounded-full bg-[#27C93F] opacity-80" />
                    </div>
                    {/* Active Tab */}
                    <div className="px-5 h-full flex items-center gap-2.5 bg-white/[0.02] border-r border-white/5 relative">
                      <div className="absolute top-0 left-0 w-full h-[2px] bg-emerald-500" />
                      <span className="text-[#FFC107] text-[13px] font-mono">
                        {"{ }"}
                      </span>
                      <span className="text-[12px] text-zinc-300 font-medium tracking-wide">
                        claude_desktop_config.json
                      </span>
                      <div className="w-4 h-4 ml-4 rounded flex items-center justify-center text-[10px] cursor-pointer text-zinc-500 hover:bg-white/10 transition-colors">
                        ✕
                      </div>
                    </div>
                  </div>

                  {/* Editor Content Area */}
                  <div className="flex-1 p-6 flex flex-row relative">
                    {/* Line Numbers */}
                    <div className="flex flex-col text-[13px] font-mono text-zinc-700 pr-5 select-none border-r border-white/5 text-right font-medium leading-[1.7]">
                      <span>1</span>
                      <span>2</span>
                      <span>3</span>
                      <span>4</span>
                      <span>5</span>
                      <span>6</span>
                      <span>7</span>
                      <span>8</span>
                      <span>9</span>
                    </div>

                    {/* Code Body */}
                    <div className="font-mono text-[13px] leading-[1.7] pl-5 flex flex-col w-full overflow-x-auto">
                      <div>
                        <span className="text-zinc-500">{"{"}</span>
                      </div>
                      <div>
                        {"  "}
                        <span className="text-[#9CDCFE]">
                          &quot;mcpServers&quot;
                        </span>
                        <span className="text-zinc-400">:</span>{" "}
                        <span className="text-zinc-500">{"{"}</span>
                      </div>
                      <div>
                        {"    "}
                        <span className="text-[#9CDCFE]">
                          &quot;omnicontext&quot;
                        </span>
                        <span className="text-zinc-400">:</span>{" "}
                        <span className="text-zinc-500">{"{"}</span>
                      </div>
                      <div>
                        {"      "}
                        <span className="text-[#9CDCFE]">
                          &quot;command&quot;
                        </span>
                        <span className="text-zinc-400">:</span>{" "}
                        <span className="text-[#CE9178]">
                          &quot;omnicontext-mcp&quot;
                        </span>
                        <span className="text-zinc-400">,</span>
                      </div>
                      <div>
                        {"      "}
                        <span className="text-[#9CDCFE]">&quot;args&quot;</span>
                        <span className="text-zinc-400">:</span>{" "}
                        <span className="text-zinc-500">[</span>
                        <span className="text-[#CE9178]">
                          &quot;--transport&quot;
                        </span>
                        <span className="text-zinc-400">,</span>{" "}
                        <span className="text-[#CE9178]">
                          &quot;stdio&quot;
                        </span>
                        <span className="text-zinc-400">,</span>{" "}
                        <span className="text-[#CE9178]">
                          &quot;--repo&quot;
                        </span>
                        <span className="text-zinc-400">,</span>{" "}
                        <span className="text-[#CE9178]">&quot;.&quot;</span>
                        <span className="text-zinc-500">]</span>
                        <span className="text-zinc-400">,</span>
                      </div>
                      <div>
                        {"      "}
                        <span className="text-[#9CDCFE]">&quot;env&quot;</span>
                        <span className="text-zinc-400">:</span>{" "}
                        <span className="text-zinc-500">{"{}"}</span>
                      </div>
                      <div>
                        {"    "}
                        <span className="text-zinc-500">{"}"}</span>
                      </div>
                      <div>
                        {"  "}
                        <span className="text-zinc-500">{"}"}</span>
                      </div>
                      <div>
                        <span className="text-zinc-500">{"}"}</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </section>

          <SiteFooterFull />
        </main>
      </div >
    </>
  );
}
