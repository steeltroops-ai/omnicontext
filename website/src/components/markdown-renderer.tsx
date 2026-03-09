"use client";

import React from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { vscDarkPlus } from "react-syntax-highlighter/dist/esm/styles/prism";
import { Copy, Check, Info, AlertTriangle, AlertCircle, CheckCircle } from "lucide-react";
import { MermaidDiagram } from "./mermaid-diagram";

interface MarkdownRendererProps {
    content: string;
}

export function MarkdownRenderer({ content }: MarkdownRendererProps) {
    const [copied, setCopied] = React.useState<string | null>(null);

    const copyToClipboard = (text: string, id: string) => {
        navigator.clipboard.writeText(text);
        setCopied(id);
        setTimeout(() => setCopied(null), 2000);
    };

    return (
        <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            components={{
                h1: ({ children }) => (
                    <h1 className="text-4xl md:text-5xl font-semibold text-white tracking-tighter mb-6 leading-tight mt-8">
                        {children}
                    </h1>
                ),
                h2: ({ children }) => {
                    const id = String(children).toLowerCase().replace(/[^a-z0-9]+/g, '-');
                    return (
                        <h2 id={id} className="text-[26px] text-white mt-12 mb-4 font-semibold tracking-tight">
                            {children}
                        </h2>
                    );
                },
                h3: ({ children }) => {
                    const id = String(children).toLowerCase().replace(/[^a-z0-9]+/g, '-');
                    return (
                        <h3 id={id} className="text-[22px] text-white mt-10 mb-3 font-semibold tracking-tight">
                            {children}
                        </h3>
                    );
                },
                h4: ({ children }) => (
                    <h4 className="text-[18px] text-white mt-8 mb-2 font-semibold tracking-tight">
                        {children}
                    </h4>
                ),
                p: ({ children }) => (
                    <p className="text-[16px] text-zinc-400 leading-relaxed mb-6 tracking-tight">
                        {children}
                    </p>
                ),
                ul: ({ children }) => (
                    <ul className="list-none space-y-2 mb-6 text-[15px] text-zinc-400">
                        {children}
                    </ul>
                ),
                ol: ({ children }) => (
                    <ol className="list-decimal list-inside space-y-2 mb-6 text-[15px] text-zinc-400">
                        {children}
                    </ol>
                ),
                li: ({ children }) => (
                    <li className="flex items-start gap-2">
                        <span className="w-1 h-1 rounded-full bg-emerald-500 mt-2 shrink-0" />
                        <span className="flex-1">{children}</span>
                    </li>
                ),
                code: ({ inline, className, children, ...props }: any) => {
                    const match = /language-(\w+)/.exec(className || '');
                    const codeString = String(children).replace(/\n$/, '');
                    const codeId = `code-${Math.random().toString(36).substr(2, 9)}`;

                    // Handle mermaid diagrams
                    if (!inline && match && match[1] === 'mermaid') {
                        return <MermaidDiagram chart={codeString} id={codeId} />;
                    }

                    if (!inline && match) {
                        return (
                            <div className="relative bg-[#0E0E11] border border-white/5 rounded-xl p-5 font-mono text-[13px] mb-6 group">
                                <div className="absolute top-3 right-3">
                                    <button
                                        onClick={() => copyToClipboard(codeString, codeId)}
                                        className="text-zinc-600 hover:text-zinc-300 transition-colors p-1"
                                    >
                                        {copied === codeId ? <Check size={14} /> : <Copy size={14} />}
                                    </button>
                                </div>
                                <div className="text-[10px] text-zinc-600 uppercase tracking-widest mb-3 font-sans font-semibold">
                                    {match[1]}
                                </div>
                                <SyntaxHighlighter
                                    style={vscDarkPlus}
                                    language={match[1]}
                                    PreTag="div"
                                    customStyle={{
                                        margin: 0,
                                        padding: 0,
                                        background: 'transparent',
                                        fontSize: '13px',
                                        lineHeight: '1.8',
                                    }}
                                    {...props}
                                >
                                    {codeString}
                                </SyntaxHighlighter>
                            </div>
                        );
                    }

                    return (
                        <code className="bg-white/5 px-1.5 py-0.5 rounded text-zinc-300 text-[13px] font-mono">
                            {children}
                        </code>
                    );
                },
                blockquote: ({ children }) => (
                    <blockquote className="border-l-4 border-emerald-500/30 pl-4 py-2 my-6 text-zinc-400 italic">
                        {children}
                    </blockquote>
                ),
                a: ({ href, children }) => (
                    <a
                        href={href}
                        className="text-emerald-400 hover:text-emerald-300 underline underline-offset-2 transition-colors"
                        target={href?.startsWith('http') ? '_blank' : undefined}
                        rel={href?.startsWith('http') ? 'noopener noreferrer' : undefined}
                    >
                        {children}
                    </a>
                ),
                table: ({ children }) => (
                    <div className="overflow-x-auto mb-8">
                        <div className="bg-[#0E0E11] border border-white/5 rounded-xl overflow-hidden inline-block min-w-full">
                            <table className="w-full text-[13px] border-collapse">{children}</table>
                        </div>
                    </div>
                ),
                thead: ({ children }) => (
                    <thead className="bg-white/[0.02]">{children}</thead>
                ),
                tbody: ({ children }) => (
                    <tbody>{children}</tbody>
                ),
                tr: ({ children }) => (
                    <tr className="border-b border-white/5 last:border-b-0">{children}</tr>
                ),
                th: ({ children }) => (
                    <th className="text-left px-4 py-3 text-zinc-400 font-semibold tracking-tight text-[12px] uppercase">
                        {children}
                    </th>
                ),
                td: ({ children }) => (
                    <td className="px-4 py-3 text-zinc-400">
                        {children}
                    </td>
                ),
                hr: () => <hr className="border-white/5 my-8" />,
                strong: ({ children }) => (
                    <strong className="text-zinc-200 font-semibold">{children}</strong>
                ),
                em: ({ children }) => (
                    <em className="text-zinc-300 italic">{children}</em>
                ),
                del: ({ children }) => (
                    <del className="text-zinc-500 line-through">{children}</del>
                ),
                input: ({ checked, ...props }: any) => (
                    <input
                        type="checkbox"
                        checked={checked}
                        disabled
                        className="mr-2 accent-emerald-500"
                        {...props}
                    />
                ),
            }}
        >
            {content}
        </ReactMarkdown>
    );
}
