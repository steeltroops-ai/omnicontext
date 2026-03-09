import { notFound } from 'next/navigation';
import { getDocBySlug, getAllDocs } from '@/lib/markdown';
import { MarkdownRenderer } from '@/components/markdown-renderer';

export async function generateStaticParams() {
    const docs = getAllDocs();
    return docs.map((doc) => ({
        slug: doc.slug.split('/'),
    }));
}

export default async function DocPage({
    params
}: {
    params: Promise<{ slug: string[] }>
}) {
    const resolvedParams = await params;
    const doc = getDocBySlug(resolvedParams.slug);

    if (!doc) {
        notFound();
    }

    return (
        <div className="flex-1 flex h-full">
            {/* Article Content */}
            <div className="flex-1 px-10 md:px-20 py-16 flex justify-center bg-[#09090B] xl:mr-[240px]">
                <article className="max-w-[760px] w-full">
                    {doc.metadata.category && (
                        <div className="text-[12px] font-semibold tracking-wider uppercase text-zinc-600 mb-6">
                            {doc.metadata.category}
                        </div>
                    )}

                    <MarkdownRenderer content={doc.content} />
                </article>
            </div>

            {/* Right TOC Sidebar */}
            {doc.headings.length > 0 && (
                <aside
                    className="w-[240px] shrink-0 p-10 overflow-y-auto custom-scrollbar border-l border-white/5 hidden xl:block bg-[#09090B] xl:fixed xl:right-0 xl:top-14 xl:bottom-0"
                    data-lenis-prevent
                >
                    <div className="text-[12px] font-semibold uppercase tracking-wider text-zinc-600 mb-6">
                        On this page
                    </div>
                    <nav className="flex flex-col gap-4 text-[13px] tracking-tight">
                        {doc.headings
                            .filter((h) => h.level <= 3)
                            .map((heading, idx) => (
                                <a
                                    key={heading.id}
                                    href={`#${heading.id}`}
                                    className={`hover:text-zinc-300 transition-colors duration-200 ${heading.level === 2
                                        ? idx === 0
                                            ? 'text-zinc-200 font-medium'
                                            : 'text-zinc-500'
                                        : 'text-zinc-600 pl-4'
                                        }`}
                                >
                                    {heading.text}
                                </a>
                            ))}
                    </nav>
                </aside>
            )}
        </div>
    );
}
