"use client";

import React from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import type { DocMetadata } from "@/lib/markdown";

interface DocsSidebarProps {
    docsByCategory: Record<string, DocMetadata[]>;
}

export function DocsSidebar({ docsByCategory }: DocsSidebarProps) {
    const pathname = usePathname();

    const isLinkActive = (slug: string) => {
        const docPath = `/docs/${slug}`;
        return pathname === docPath
            ? "bg-zinc-900 text-zinc-100 font-medium"
            : "text-zinc-500 hover:text-zinc-300 hover:bg-zinc-900/30 font-normal";
    };

    // Sort categories by order of first doc in each category
    const sortedCategories = Object.entries(docsByCategory).sort(
        ([, docsA], [, docsB]) => {
            const minOrderA = Math.min(...docsA.map((d) => d.order ?? 999));
            const minOrderB = Math.min(...docsB.map((d) => d.order ?? 999));
            return minOrderA - minOrderB;
        }
    );

    return (
        <>
            {sortedCategories.map(([category, docs]) => (
                <div key={category} className="mb-10">
                    <div className="text-[11px] uppercase tracking-widest text-zinc-600 font-semibold mb-4 px-2">
                        {category}
                    </div>
                    {docs.map((doc) => (
                        <Link
                            key={doc.slug}
                            href={`/docs/${doc.slug}`}
                            className={`flex items-center px-3 py-2 mb-1 rounded-lg text-[13px] transition-all duration-200 ${isLinkActive(doc.slug)}`}
                        >
                            {doc.title}
                        </Link>
                    ))}
                </div>
            ))}
        </>
    );
}
