# OmniContext: Investor Pitch Deck Outline

> Condensed version of ENTERPRISE_STRATEGY.md for investor conversations.
> Slide-by-slide structure for a 12-minute pitch + 8-minute Q&A.

---

## Slide 1 -- Title

**OmniContext**
_The Context Layer for AI Agents_

Mayank | steeltroops.ai@gmail.com | github.com/steeltroops-ai/omnicontext

---

## Slide 2 -- The Problem (Make it visceral)

**AI agents are smart. They just can't see.**

Every developer using Cursor, Claude, or Copilot hits the same wall:

> "Why is the payment webhook failing?"
> AI gives a confident, wrong answer.
> Not because the AI is weak. Because it only saw 0.5% of your codebase.

- Average production codebase: 500K-2M lines of code
- Average AI context window: 10-15K lines
- The AI is flying blind on **97-99.5% of your code**, every query, every day

---

## Slide 3 -- Why Now

**Three forces converging in 2026:**

1. **AI Agent Explosion** -- Claude, GPT, Gemini are being deployed as autonomous agents inside enterprise dev workflows. They are blocked by the context problem.

2. **Enterprise AI Hit the Wall** -- Companies that adopted AI copilots in 2024 are finding they fail on large, private, messy codebases. The next phase of adoption requires solving context.

3. **MCP Won** -- Anthropic's Model Context Protocol is becoming the universal plugin standard for AI tools. OmniContext is already an MCP server. We are already integrated with every AI tool that matters.

---

## Slide 4 -- The Solution

**OmniContext: Local-first, AI-native code intelligence**

One binary. Zero config. Instant setup.

- **Indexes your entire codebase** using real AST parsing (not naive chunking)
- **Hybrid search**: keyword + semantic vector at sub-100ms latency
- **Runs fully offline**: No API keys. No cloud. Your code never leaves your machine.
- **MCP native**: Any AI agent (Claude, Cursor, Cline) queries it as a tool

```
Before OmniContext:  Agent sees 50 files out of 5,000
After OmniContext:   Agent sees the exactly right 5 files out of 5,000
```

[DEMO VIDEO / LIVE DEMO HERE]

---

## Slide 5 -- Technical Moat

**Why this is hard to copy:**

| Layer        | What We Built                             | Why It Matters                                             |
| ------------ | ----------------------------------------- | ---------------------------------------------------------- |
| Parser       | Tree-sitter AST chunking                  | Semantic units, not line-count chunks                      |
| Search       | BM25 + HNSW hybrid                        | Same architecture as production Elasticsearch, in a binary |
| Inference    | ONNX Runtime in-process                   | Zero-latency model inference, model-agnostic               |
| Protocol     | MCP server                                | Plugs into every AI tool, now and in the future            |
| Distribution | Single static binary, auto-model-download | No DevOps, no setup friction                               |

The combination of all five layers in a single local binary -- this does not exist elsewhere.

---

## Slide 6 -- Market Size

**Context infrastructure is the foundational layer of the AI coding market**

|                                       |              |
| ------------------------------------- | ------------ |
| TAM: AI developer tools               | $25B by 2030 |
| SAM: Code intelligence infrastructure | ~$5B         |
| SOM: Capturable in 3 years            | $50-150M ARR |

Comparable companies:

- Sourcegraph: $2.6B valuation (code intelligence)
- Cursor: $9B valuation (AI coding)
- GitHub: $7.5B acquisition (code infrastructure)

**Context is the infrastructure position in this market. No clear winner yet.**

---

## Slide 7 -- Product Roadmap

**Phase 1** (Done) -- Local, single-user, open source
**Phase 2** (2026) -- Team server, SSO, multi-repo, REST API
**Phase 3** (2027) -- Domain context engines: Research Papers, Docs, Legal  
**Phase 4** (2028) -- OmniContext Cloud: hosted, SOC2, usage-based

The domain engine expansion is the key strategic insight:

> The context problem is not unique to code.
> Research teams, legal teams, support teams -- all have AI agents that cannot find the right document.
> OmniContext's architecture generalizes to any knowledge domain.

---

## Slide 8 -- Business Model

**Developer-Led Growth -> Enterprise Expansion**

| Tier        | Price        | Target                          |
| ----------- | ------------ | ------------------------------- |
| Open Source | Free         | Individual devs (growth engine) |
| Team        | $299/mo      | 2-10 developer teams            |
| Business    | $999/mo      | 50 developer orgs               |
| Enterprise  | $50K-500K/yr | Large engineering orgs          |

**GTM**: Open source as distribution. Individual developer love -> team adoption -> enterprise contract.

This is the exact playbook of Terraform, Sentry, Grafana, and Sourcegraph.

---

## Slide 9 -- Traction

[Insert at time of pitch]

- GitHub Stars: \_\_\_
- Weekly active users: \_\_\_
- Design partners: \_\_\_ (with company names if permitted)
- ARR / Pipeline: \_\_\_
- Notable users: \_\_\_

---

## Slide 10 -- The Acquisition Scenario

**Who buys OmniContext and why:**

| Acquirer           | Why They Buy                         | Price Range |
| ------------------ | ------------------------------------ | ----------- |
| GitHub / Microsoft | Context layer for Copilot Enterprise | $200M-500M  |
| Anthropic          | Best MCP-native context engine       | $100M-300M  |
| Cursor             | Eliminates their #1 product weakness | $50M-200M   |
| JetBrains          | AI context for IntelliJ/PyCharm      | $100M-300M  |

Comparable: GitHub acquired Semmle (CodeQL) for ~$500M. Similar infrastructure positioning.

**We are not building a product. We are building the infrastructure that AI coding tools run on.**

---

## Slide 11 -- Team

**Mayank** -- Founder & Engineer

- Built the entire OmniContext stack: Rust, ONNX inference, MCP, AST parsing
- [Add relevant background]
- GitHub: steeltroops-ai | Portfolio: steeltroops.vercel.app

**Looking to hire**: Senior Rust/Systems Engineer, Developer Relations

---

## Slide 12 -- The Ask

**Raising: $[X] at $[Y] pre-money valuation**

Use of funds:

1. **Engineering** (60%): Hire Senior Rust engineer, build server mode and enterprise auth
2. **Growth** (25%): Developer relations, documentation, community, conference presence
3. **Operations** (15%): Legal, cloud infrastructure, compliance groundwork

In 18 months:

- $[X] ARR with [N] enterprise customers
- Research Paper Engine live and generating revenue
- Series A ready with proven land-and-expand motion

---

## Appendix: Key Questions We Expect

**"Why won't OpenAI/Anthropic build this themselves?"**

> They build for their cloud. Air-gapped enterprise that will not allow code to leave their environment is structurally outside their business model. That market is ours.

**"What if Microsoft adds this to Copilot?"**

> That increases our acquisition probability. But we move faster, we are multi-platform, and enterprises with mixed AI tool stacks will always need a neutral context layer.

**"Why Rust? Is this a technical choice that limits hiring?"**

> Rust is the right choice: memory safety, performance, single static binary. It is a hiring challenge at scale, but at seed stage it is an asset -- Rust engineers are senior, pragmatic, and deeply motivated by the problem domain.

**"What is the defensibility once you have a first-mover advantage?"**

> Once an enterprise's codebase is indexed, migration is painful. The more AI queries run against OmniContext, the more usage patterns are understood and optimized. The switching cost is high and grows over time.
