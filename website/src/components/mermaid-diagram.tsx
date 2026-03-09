"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import mermaid from "mermaid";
import { ZoomIn, ZoomOut, Maximize2, RotateCcw } from "lucide-react";

// Interactive mermaid diagram with zoom, pan, and fullscreen controls

interface MermaidDiagramProps {
    chart: string;
    id: string; // Make id required instead of optional
}

export function MermaidDiagram({ chart, id }: MermaidDiagramProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const contentRef = useRef<HTMLDivElement>(null);
    const [zoom, setZoom] = useState(1);
    const [isFullscreen, setIsFullscreen] = useState(false);
    const [svg, setSvg] = useState<string>("");
    const [isDragging, setIsDragging] = useState(false);
    const [isFocused, setIsFocused] = useState(false);
    const [position, setPosition] = useState({ x: 0, y: 0 });
    const dragStart = useRef({ x: 0, y: 0 });
    const lastPosition = useRef({ x: 0, y: 0 });

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
                // Use the provided stable ID instead of generating random one
                const { svg: renderedSvg } = await mermaid.render(id, chart);
                setSvg(renderedSvg);
            } catch (error) {
                console.error("Mermaid rendering error:", error);
            }
        };

        renderDiagram();
    }, [chart, id]);

    const handleZoomIn = useCallback(() => {
        setZoom((prev) => Math.min(prev + 0.2, 3));
    }, []);

    const handleZoomOut = useCallback(() => {
        setZoom((prev) => Math.max(prev - 0.2, 0.5));
    }, []);

    const handleFullscreen = useCallback(() => {
        setIsFullscreen((prev) => !prev);
        if (!isFullscreen) {
            setZoom(1);
            setPosition({ x: 0, y: 0 });
            lastPosition.current = { x: 0, y: 0 };
        }
    }, [isFullscreen]);

    const handleReset = useCallback(() => {
        setZoom(1);
        setPosition({ x: 0, y: 0 });
        lastPosition.current = { x: 0, y: 0 };
    }, []);

    const handleWheel = useCallback((e: WheelEvent) => {
        // Only zoom if diagram is focused (user has clicked on it)
        if (!isFocused) return;

        e.preventDefault();
        e.stopPropagation();

        const delta = e.deltaY > 0 ? -0.1 : 0.1;
        setZoom((prev) => Math.max(0.5, Math.min(3, prev + delta)));
    }, [isFocused]);

    const handleMouseDown = useCallback((e: MouseEvent) => {
        // Only start dragging on left click
        if (e.button !== 0) return;

        e.preventDefault();
        e.stopPropagation();

        // Focus the diagram when clicked
        setIsFocused(true);
        setIsDragging(true);
        dragStart.current = {
            x: e.clientX - lastPosition.current.x,
            y: e.clientY - lastPosition.current.y,
        };
    }, []);

    const handleMouseMove = useCallback((e: MouseEvent) => {
        if (!isDragging) return;

        e.preventDefault();
        e.stopPropagation();

        const newX = e.clientX - dragStart.current.x;
        const newY = e.clientY - dragStart.current.y;

        lastPosition.current = { x: newX, y: newY };
        setPosition({ x: newX, y: newY });
    }, [isDragging]);

    const handleMouseUp = useCallback((e: MouseEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setIsDragging(false);
    }, []);

    // Set up event listeners
    useEffect(() => {
        const container = containerRef.current;
        if (!container) return;

        // Add wheel listener with passive: false to allow preventDefault
        container.addEventListener('wheel', handleWheel, { passive: false });
        container.addEventListener('mousedown', handleMouseDown);

        // Add global listeners for mouse move and up (so dragging works even outside container)
        const handleGlobalMouseMove = (e: MouseEvent) => {
            if (isDragging) {
                handleMouseMove(e);
            }
        };

        const handleGlobalMouseUp = (e: MouseEvent) => {
            if (isDragging) {
                handleMouseUp(e);
            }
        };

        // Handle clicks outside to unfocus
        const handleClickOutside = (e: MouseEvent) => {
            if (container && !container.contains(e.target as Node)) {
                setIsFocused(false);
            }
        };

        document.addEventListener('mousemove', handleGlobalMouseMove);
        document.addEventListener('mouseup', handleGlobalMouseUp);
        document.addEventListener('mousedown', handleClickOutside);

        return () => {
            container.removeEventListener('wheel', handleWheel);
            container.removeEventListener('mousedown', handleMouseDown);
            document.removeEventListener('mousemove', handleGlobalMouseMove);
            document.removeEventListener('mouseup', handleGlobalMouseUp);
            document.removeEventListener('mousedown', handleClickOutside);
        };
    }, [handleWheel, handleMouseDown, handleMouseMove, handleMouseUp, isDragging]);

    return (
        <div
            className={`relative bg-[#0E0E11] border rounded-xl overflow-hidden mb-8 select-none transition-colors ${isFocused ? "border-emerald-500/50" : "border-white/5"
                } ${isFullscreen ? "fixed inset-4 z-50" : ""}`}
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
                    <RotateCcw size={16} className="text-zinc-400" />
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
                {isFocused ? "Click & drag to pan • Scroll to zoom" : "Click to activate controls"}
            </div>

            {/* Diagram Container */}
            <div
                ref={containerRef}
                className={`overflow-hidden ${isDragging ? "cursor-grabbing" : isFocused ? "cursor-grab" : "cursor-pointer"
                    }`}
                style={{
                    maxHeight: isFullscreen ? "calc(100vh - 2rem)" : "600px",
                    touchAction: "none",
                }}
            >
                <div
                    ref={contentRef}
                    className="p-8 pointer-events-none"
                    style={{
                        transform: `translate(${position.x}px, ${position.y}px) scale(${zoom})`,
                        transformOrigin: "0 0",
                        transition: isDragging ? "none" : "transform 0.1s ease-out",
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
