/**
 * Markdown processing utilities for docs
 */

import fs from 'fs';
import path from 'path';
import matter from 'gray-matter';

export interface DocMetadata {
    title: string;
    description?: string;
    category?: string;
    order?: number;
    slug: string;
    filePath: string;
}

export interface DocContent {
    metadata: DocMetadata;
    content: string;
    headings: { id: string; text: string; level: number }[];
}

const DOCS_PATH = path.join(process.cwd(), 'website', 'docs');

/**
 * Get all markdown files recursively from docs directory
 */
export function getAllDocs(): DocMetadata[] {
    const docs: DocMetadata[] = [];

    function scanDirectory(dir: string, baseSlug = '') {
        const entries = fs.readdirSync(dir, { withFileTypes: true });

        for (const entry of entries) {
            const fullPath = path.join(dir, entry.name);

            if (entry.isDirectory()) {
                scanDirectory(fullPath, `${baseSlug}/${entry.name}`);
            } else if (entry.name.endsWith('.md')) {
                const fileContents = fs.readFileSync(fullPath, 'utf8');
                const { data } = matter(fileContents);

                const slug = `${baseSlug}/${entry.name.replace(/\.md$/, '')}`.replace(/^\//, '');

                docs.push({
                    title: data.title || entry.name.replace(/\.md$/, ''),
                    description: data.description,
                    category: data.category,
                    order: data.order || 999,
                    slug,
                    filePath: fullPath,
                });
            }
        }
    }

    if (fs.existsSync(DOCS_PATH)) {
        scanDirectory(DOCS_PATH);
    }

    return docs.sort((a, b) => (a.order ?? 999) - (b.order ?? 999));
}

/**
 * Get doc content by slug
 */
export function getDocBySlug(slug: string[] | undefined): DocContent | null {
    // Handle undefined or empty slug
    if (!slug || slug.length === 0 || (slug.length === 1 && slug[0] === '')) {
        const indexPath = path.join(DOCS_PATH, 'index.md');
        if (fs.existsSync(indexPath)) {
            const fileContents = fs.readFileSync(indexPath, 'utf8');
            const { data, content } = matter(fileContents);
            const headings = extractHeadings(content);

            return {
                metadata: {
                    title: data.title || 'Introduction',
                    description: data.description,
                    category: data.category,
                    order: data.order || 0,
                    slug: '',
                    filePath: indexPath,
                },
                content,
                headings,
            };
        }
        return null;
    }

    const filePath = path.join(DOCS_PATH, ...slug) + '.md';

    if (!fs.existsSync(filePath)) {
        return null;
    }

    const fileContents = fs.readFileSync(filePath, 'utf8');
    const { data, content } = matter(fileContents);

    // Extract headings for TOC
    const headings = extractHeadings(content);

    return {
        metadata: {
            title: data.title || slug[slug.length - 1],
            description: data.description,
            category: data.category,
            order: data.order || 999,
            slug: slug.join('/'),
            filePath,
        },
        content,
        headings,
    };
}

/**
 * Extract headings from markdown content
 */
function extractHeadings(markdown: string): { id: string; text: string; level: number }[] {
    const headingRegex = /^(#{1,6})\s+(.+)$/gm;
    const headings: { id: string; text: string; level: number }[] = [];

    let match;
    while ((match = headingRegex.exec(markdown)) !== null) {
        const level = match[1].length;
        const text = match[2].trim();
        const id = text.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

        headings.push({ id, text, level });
    }

    return headings;
}

/**
 * Group docs by category for navigation
 */
export function getDocsByCategory(): Record<string, DocMetadata[]> {
    const allDocs = getAllDocs();
    const grouped: Record<string, DocMetadata[]> = {};

    for (const doc of allDocs) {
        const category = doc.category || 'Uncategorized';
        if (!grouped[category]) {
            grouped[category] = [];
        }
        grouped[category].push(doc);
    }

    return grouped;
}
