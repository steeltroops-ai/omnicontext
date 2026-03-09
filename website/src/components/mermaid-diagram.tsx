"use client";

import { useEffect, useRef, useState } from "react";
import mermaid from "mermaid";
import { ZoomIn, ZoomOut, Maximize2 } from "lucide-react";

interface MermaidDiagramProps {
    chart: string;
    id?: string;
}

export function MermaidDiagram({ chart, id }: MermaidDiagramProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const [zoom, setZoom] = useState(1);
    const [isFullscreen, setIsFullscreen] = useState(false);
    const [svg, setSvg] = useState<string>("");

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
    const handleFullscreen = () => setIsFullscreen(!isFullscreen);

    return (
        <div
            className={`relative bg-[#0E0E11] border border-white/5 rounded-xl overflow-hidden mb-8 ${isFullscreen ? "fixed inset-4 z-50" : ""
                }`}
        >
            {/* Controls */}
            <div className="absolute top-3 right-3 flex gap-2 z-10">
                <button
                    onClick={handleZoomOut}
                    className="p-2 bg-zinc-900/80 hover:bg-zinc-800 border border-white/10 rounded-lg transition-colors"
                    title="Zoom Out"
                >
                    <ZoomOut size={16} className="text-zinc-400" />
                </button>
                <button
                    onClick={handleZoomIn}
                    className="p-2 bg-zinc-900/80 hover:bg-zinc-800 border border-white/10 rounded-lg transition-colors"
                    title="Zoom In"
                >
                    <ZoomIn size={16} className="text-zinc-400" />
                </button>
                <button
                    onClick={handleFullscreen}
                    className="p-2 bg-zinc-900/80 hover:bg-zinc-800 border border-white/10 rounded-lg transition-colors"
                    title="Fullscreen"
                >
                    <Maximize2 size={16} className="text-zinc-400" />
                </button>
            </div>

            {/* Diagram Container */}
            <div
                ref={containerRef}
                className="p-8 overflow-auto custom-scrollbar"
                style={{
                    maxHeight: isFullscreen ? "calc(100vh - 2rem)" : "600px",
                }}
            >
                <div
                    style={{
                        transform: `scale(${zoom})`,
                        transformOrigin: "top left",
                        transition: "transform 0.2s ease-out",
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
