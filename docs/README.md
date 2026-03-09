---
title: Documentation System
description: How to add and manage documentation
category: Meta
order: 999
---

# Documentation System

This directory contains all documentation for OmniContext. The website automatically generates navigation and pages from markdown files in this directory.

## Adding a new doc page

1. Create a markdown file in the appropriate subdirectory (e.g., `docs/getting-started/my-page.md`)
2. Add frontmatter with metadata:

```markdown
---
title: My Page Title
description: Brief description for SEO
category: Getting Started
order: 10
---

# My Page Title

Your content here...
```

3. The page will automatically appear in the docs navigation at `/docs/getting-started/my-page`

## Frontmatter fields

- `title` (required): Page title shown in navigation and heading
- `description` (optional): SEO description
- `category` (required): Navigation section (e.g., "Getting Started", "Core Concepts")
- `order` (required): Sort order within category (lower numbers appear first)

## Supported markdown features

- Headings (H1-H6)
- Paragraphs and line breaks
- Lists (ordered and unordered)
- Code blocks with syntax highlighting
- Inline code
- Links (internal and external)
- Blockquotes
- Tables
- Horizontal rules

## Code blocks

Use fenced code blocks with language identifiers:

\`\`\`rust
fn main() {
    println!("Hello, world!");
}
\`\`\`

Supported languages: rust, typescript, javascript, python, bash, json, toml, yaml, and more.

## Internal links

Link to other docs pages using relative paths:

```markdown
See [Installation](/docs/getting-started/installation) for setup instructions.
```

## Categories

Current categories (in order):
- Getting Started (order: 0-10)
- Core Concepts (order: 11-20)
- MCP Integration (order: 21-30)
- Architecture (order: 31-40)
- Enterprise (order: 41-50)

Add new categories by using them in frontmatter. They'll appear automatically in navigation.

## File organization

Organize files by topic in subdirectories:

```
docs/
├── getting-started/
│   ├── installation.md
│   └── quickstart.md
├── core-concepts/
│   ├── indexing.md
│   └── search.md
└── api-reference/
    └── mcp-tools.md
```

The URL structure mirrors the file structure: `docs/getting-started/installation.md` → `/docs/getting-started/installation`

## Table of contents

The right sidebar automatically generates a table of contents from H2 and H3 headings in your markdown.

## Styling

All markdown is automatically styled to match the website design. No custom CSS needed.
