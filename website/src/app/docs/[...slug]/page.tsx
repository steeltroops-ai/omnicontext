import { notFound } from 'next/navigation';
import { getDocBySlug, getAllDocs } from '@/lib/markdown';
import { MarkdownRenderer } from '@/components/markdown-renderer';
import { TOC } from '@/components/toc';

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
            <TOC headings={doc.headings} />
        </div>
    );
}
