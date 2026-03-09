"use client";

import { useEffect, useRef, useState } from "react";
import mermaid from "mermaid";
import { ZoomIn, ZoomOut, Maximize2, Move } from "lucide-react";

interface MermaidDiagramProps {
    chart: string;
    id?: string;
}

export function MermaidDiagram({ chart, id }: MermaidDiagramProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const contentRef = useRef<HTMLDivElement>(null);
    const [zoom, setZoom] = useState(1);
    const [isFullscreen, setIsFullscreen] = useState(false);
    const [svg, setSvg] = useState<string>("");
    const [isPanning, setIsPanning] = useState(false);
    const [position, setPosition] = useState({ x: 0, y: 0 });
    const [startPos, setStartPos] = useState({ x: 0, y: 0 });

    useEffect(() => {
        mermaid.initialize({
            startOnLoad: true,
            theme: "dark",
            themeVariables: {
                primaryColor: "#10b981",
                primaryTextColor: "#fff",
                primaryBorderColor: "#059669",
                lineColor: "#6b7280",
                secondaryColor: "#1f2937",
                tertiaryColor: "#111827",
                background: "#0E0E11",
                mainBkg: "#0E0E11",
                secondBkg: "#1a1a1f",
                tertiaryBkg: "#27272a",
                textColor: "#e4e4e7",
                border1: "#3f3f46",
                border2: "#52525b",
                fontSize: "14px",
            },
            flowchart: {
                useMaxWidth: false,
                htmlLabels: true,
                curve: "basis",
            },
        });

        const renderDiagram = async () => {
            try {
                const uniqueId = id || `mermaid-${Math.random().toString(36).substr(2, 9)}`;
                const { svg: renderedSvg } = await mermaid.render(uniqueId, chart);
                setSvg(renderedSvg);
            } catch (error) {
                console.error("Mermaid rendering error:", error);
            }
        };

        renderDiagram();
    }, [chart, id]);

    const handleZoomIn = () => setZoom((prev) => Math.min(prev + 0.2, 3));
    const handleZoomOut = () => setZoom((prev) => Math.max(prev - 0.2, 0.5));
    const handleFullscreen = () => {
        setIsFullscreen(!isFullscreen);
        setZoom(1);
        setPosition({ x: 0, y: 0 });
    };

    const handleWheel = (e: React.WheelEvent) => {
        e.preventDefault();
        const delta = e.deltaY > 0 ? -0.1 : 0.1;
        setZoom((prev) => Math.max(0.5, Math.min(3, prev + delta)));
    };

    const handleMouseDown = (e: React.MouseEvent) => {
        if (e.button === 0) {
            setIsPanning(true);
            setStartPos({
                x: e.clientX - position.x,
                y: e.clientY - position.y,
            });
        }
    };

    const handleMouseMove = (e: React.MouseEvent) => {
        if (isPanning) {
            setPosition({
                x: e.clientX - startPos.x,
                y: e.clientY - startPos.y,
            });
        }
    };

    const handleMouseUp = () => {
        setIsPanning(false);
    };

    const handleMouseLeave = () => {
        setIsPanning(false);
    };

    const handleReset = () => {
        setZoom(1);
        setPosition({ x: 0, y: 0 });
    };

    return (
        <div
            className={`relative bg-[#0E0E11] border border-white/5 rounded-xl overflow-hidden mb-8 ${isFullscreen ? "fixed inset-4 z-50" : ""
                }`}
        >
            {/* Controls */}
            <div className="absolute top-3 right-3 flex gap-2 z-10">
                <button
                    onClick={handleZoomOut}
                    className="p-2 bg-zinc-900/90 hover:bg-zinc-800 border border-white/10 rounded-lg transition-colors backdrop-blur-sm"
                    title="Zoom Out"
                >
                    <ZoomOut size={16} className="text-zinc-400" />
                </button>
                <button
                    onClick={handleZoomIn}
                    className="p-2 bg-zinc-900/90 hover:bg-zinc-800 border border-white/10 rounded-lg transition-colors backdrop-blur-sm"
                    title="Zoom In"
                >
                    <ZoomIn size={16} className="text-zinc-400" />
                </button>
                <button
                    onClick={handleReset}
                    className="p-2 bg-zinc-900/90 hover:bg-zinc-800 border border-white/10 rounded-lg transition-colors backdrop-blur-sm"
                    title="Reset View"
                >
                    <Move size={16} className="text-zinc-400" />
                </button>
                <button
                    onClick={handleFullscreen}
                    className="p-2 bg-zinc-900/90 hover:bg-zinc-800 border border-white/10 rounded-lg transition-colors backdrop-blur-sm"
                    title="Fullscreen"
                >
                    <Maximize2 size={16} className="text-zinc-400" />
                </button>
            </div>

            {/* Zoom indicator */}
            <div className="absolute top-3 left-3 px-3 py-1.5 bg-zinc-900/90 border border-white/10 rounded-lg text-[11px] text-zinc-400 font-mono backdrop-blur-sm z-10">
                {Math.round(zoom * 100)}%
            </div>

            {/* Instructions */}
            <div className="absolute bottom-3 left-3 px-3 py-1.5 bg-zinc-900/90 border border-white/10 rounded-lg text-[11px] text-zinc-500 backdrop-blur-sm z-10">
                Drag to pan • Scroll to zoom
            </div>

            {/* Diagram Container */}
            <div
                ref={containerRef}
                className={`overflow-hidden ${isPanning ? "cursor-grabbing" : "cursor-grab"}`}
                style={{
                    maxHeight: isFullscreen ? "calc(100vh - 2rem)" : "600px",
                }}
                onWheel={handleWheel}
                onMouseDown={handleMouseDown}
                onMouseMove={handleMouseMove}
                onMouseUp={handleMouseUp}
                onMouseLeave={handleMouseLeave}
            >
                <div
                    ref={contentRef}
                    className="p-8"
                    style={{
                        transform: `translate(${position.x}px, ${position.y}px) scale(${zoom})`,
                        transformOrigin: "0 0",
                        transition: isPanning ? "none" : "transform 0.1s ease-out",
                        minWidth: "100%",
                        minHeight: "100%",
                    }}
                    dangerouslySetInnerHTML={{ __html: svg }}
                />
            </div>

            {/* Fullscreen overlay backdrop */}
            {isFullscreen && (
                <div
                    className="fixed inset-0 bg-black/80 -z-10"
                    onClick={handleFullscreen}
                />
            )}
        </div>
    );
}
