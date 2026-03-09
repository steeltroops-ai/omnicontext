"use client";

import { useEffect, useState } from "react";

interface Heading {
    id: string;
    text: string;
    level: number;
}

interface TOCProps {
    headings: Heading[];
}

export function TOC({ headings }: TOCProps) {
    const [activeId, setActiveId] = useState<string>("");

    useEffect(() => {
        const observer = new IntersectionObserver(
            (entries) => {
                entries.forEach((entry) => {
                    if (entry.isIntersecting) {
                        setActiveId(entry.target.id);
                    }
                });
            },
            {
                rootMargin: "-80px 0px -80% 0px",
                threshold: 1.0,
            }
        );

        // Observe all headings
        headings.forEach((heading) => {
            const element = document.getElementById(heading.id);
            if (element) {
                observer.observe(element);
            }
        });

        return () => {
            observer.disconnect();
        };
    }, [headings]);

    if (headings.length === 0) return null;

    return (
        <aside
            className="w-[240px] shrink-0 p-10 overflow-y-auto custom-scrollbar border-l border-white/5 hidden xl:block bg-[#09090B] xl:fixed xl:right-0 xl:top-14 xl:bottom-0"
            data-lenis-prevent
        >
            <div className="text-[12px] font-semibold uppercase tracking-wider text-zinc-600 mb-6">
                On this page
            </div>
            <nav className="flex flex-col gap-4 text-[13px] tracking-tight">
                {headings
                    .filter((h) => h.level <= 3)
                    .map((heading) => (
                        <a
                            key={heading.id}
                            href={`#${heading.id}`}
                            className={`transition-colors duration-200 ${heading.level === 3 ? "pl-4" : ""
                                } ${activeId === heading.id
                                    ? "text-emerald-400 font-medium"
                                    : heading.level === 2
                                        ? "text-zinc-500 hover:text-zinc-300"
                                        : "text-zinc-600 hover:text-zinc-400"
                                }`}
                        >
                            {heading.text}
                        </a>
                    ))}
            </nav>
        </aside>
    );
}
