# OmniContext: Enterprise Strategy, Investor Pitch & Roadmap

> **Internal document. Do not publish.**
> Author: Mayank (steeltroops.ai@gmail.com)
> Date: March 2026

---

## Table of Contents

1. [The Problem: Why This Exists](#1-the-problem)
2. [The Market Moment](#2-the-market-moment)
3. [What OmniContext Is](#3-what-omnicontext-is)
4. [Technical Moat](#4-technical-moat)
5. [Enterprise Product Roadmap](#5-enterprise-product-roadmap)
6. [Go-To-Market Strategy](#6-go-to-market-strategy)
7. [Revenue Model](#7-revenue-model)
8. [Funding Strategy](#8-funding-strategy)
9. [Who to Pitch and What to Say](#9-who-to-pitch)
10. [The Acquisition Thesis](#10-acquisition-thesis)
11. [Realistic Financial Projections](#11-financial-projections)
12. [Team & What We Need to Hire](#12-team)
13. [Risks and Mitigations](#13-risks)
14. [The Dream: What This Becomes](#14-the-dream)

---

## 1. The Problem

### AI Agents Are Blind Inside Large Codebases

Every serious AI coding tool -- Cursor, GitHub Copilot, Cline, Claude -- shares one fundamental, unsolved problem: **they cannot see your full codebase**. They operate on a context window. Your codebase is infinite compared to that window. The agent is flying blind.

This is not a small UX problem. This is the single biggest technical blocker to AI agents actually replacing human developers for complex tasks. Here is the exact failure mode in production today:

```
Developer: "Why is the payment webhook failing for Stripe?"

AI Agent (with full codebase access in theory):
-> Looks at 4 files it guesses are relevant
-> Misses 2 critical utility functions in /lib
-> Misses the environment config that overrides the behavior
-> Gives a wrong answer with confidence
```

The agent didn't fail because the model was bad. It failed because **it couldn't find the right context**.

### The Scale of the Problem

- The average production codebase at a Series B company: **500,000 - 2,000,000 lines of code**
- What a 200K context window can hold: **~10,000-15,000 lines** (with formatting, comments, etc.)
- Ratio: The agent sees **0.5% - 3%** of your codebase at any given time
- Number of AI coding tool users globally (2026 estimate): **25-40 million developers**
- Percentage who regularly hit this context wall: **100%**

### Why Existing Solutions Fail

| Approach                                 | Why It Fails                                                        |
| ---------------------------------------- | ------------------------------------------------------------------- |
| Dump everything in context               | Costs $50+/query at scale, models degrade with too much context     |
| Cloud-based RAG (Sourcegraph Cody, etc.) | Your code leaves your environment. Non-starter for enterprise       |
| IDE-native grep/search                   | No semantic understanding. `function handles payment` finds nothing |
| Vector DBs (Pinecone, Weaviate)          | Require infrastructure, DevOps, API keys, ongoing costs             |
| LLM providers' built-in retrieval        | Vendor lock-in, cloud-only, code data is their training data        |

OmniContext solves all of these simultaneously.

---

## 2. The Market Moment

### Why 2026 is the Right Time

Three forces are converging right now:

**1. AI Agent Explosion**
Every major AI lab released agentic coding products in 2025. Claude with computer use, GPT-4o with tool calling, Gemini 1.5 deeply embedded in IDEs. These agents need context. The better the context layer, the better the agent. Context infrastructure becomes the critical path.

**2. Enterprise AI Adoption Hit the Wall**
Enterprises that moved fast on AI copilots in 2024 are now facing a hard reality: the tools don't work on their private, large codebases. The coding AI that worked great on open-source demos breaks down on a 3M-line internal monolith from 2008. They need a context layer that can handle messy, private, massive codebases.

**3. The MCP Standard Just Won**
Anthropic's Model Context Protocol (MCP) is being adopted by every major AI provider. OmniContext is already an MCP server. This is not an integration detail -- it means **every MCP-compatible AI client in the world can use OmniContext as a plugin with zero code changes**. The distribution moat just appeared.

### Market Size (TAM/SAM/SOM)

| Market                                      | Size         | Basis                                        |
| ------------------------------------------- | ------------ | -------------------------------------------- |
| **TAM**: AI developer tools market          | $25B by 2030 | Multiple analyst reports                     |
| **SAM**: Code intelligence / search infra   | ~$4-6B       | Enterprise IDEs + code search tools          |
| **SOM (3yr)**: Capturable with current moat | $50-150M ARR | 1,000-3,000 enterprise teams at $50K-100K/yr |

This is not a small market. GitHub sold for $7.5B. Sourcegraph is valued at $2.6B. Cursor hit $9B valuation in 2025. The infrastructure layer under these tools -- context -- has no clear winner yet.

---

## 3. What OmniContext Is

### One-Line Definition

> OmniContext is a local-first, AI-native code intelligence engine that gives any AI agent persistent, semantic awareness of your entire codebase -- without ever sending your code to a third party.

### What It Does Today (v0.2)

- **Indexes any codebase** in seconds using hybrid keyword + vector search
- **Runs fully offline** -- ONNX inference, single binary, no API keys
- **Speaks MCP** -- any AI client (Claude, Cursor, Cline) uses it as a tool
- **Handles 20+ languages** with tree-sitter AST parsing
- **Sub-100ms query latency** on a developer laptop
- **Daemon mode** -- background indexing, always up to date

### What It Will Be (v1.0 Enterprise)

- Team knowledge sync -- one index, all developers, shared context
- Server deployment -- hosted indexing with your infrastructure
- Domain context engines -- research papers, API docs, internal wikis
- Access control -- per-repo, per-team, per-user permissions
- Analytics -- what context is your team actually using with AI?

---

## 4. Technical Moat

This section is critical for technical investors and acquirers.

### Why This is Hard to Copy

**1. The Parser Layer**
OmniContext uses tree-sitter, the production-grade parser used inside Neovim, GitHub, and VS Code, to do real AST-level code parsing. We don't chunk by line count or token count -- we chunk by semantic unit (function, class, module). This means embeddings are semantically richer. Competitors doing naive chunking produce worse embeddings regardless of the model.

**2. The Hybrid Search Architecture**
Full-text BM25 + dense vector search + graph-aware re-ranking, all running in-process in Rust. No external databases. No Elasticsearch. No Pinecone. This is the same architecture at the core of Elasticsearch Enterprise and Vespa, but running as a single binary on a laptop. The latency profile is fundamentally different from any cloud-based RAG system.

**3. The Embedder Stack**
We run ONNX inference in-process via `ort`. This means:

- Zero network calls during query
- Deterministic performance (no API rate limits)
- Model upgrades are config changes, not infrastructure changes
- The embedding layer is hot-swappable -- we can run different models per domain

**4. ONNX as the Abstraction Layer**
Every major embedding model in the world can be exported to ONNX format. This means we are model-agnostic at the infrastructure level. When a better model drops (Qodo, Jina, BGE, whatever), we update a URL in a config file. This is not how most RAG systems work -- they are usually coupled to a specific model SDK.

**5. The MCP Distribution Moat**
Being an MCP server means every AI agent, every AI IDE, every AI coding tool that implements MCP (which is becoming the standard) can use OmniContext as a drop-in context provider. We don't need to build integrations with Cursor, Claude, Cline individually -- they integrate with us.

### Dependency Graph (What We Own vs. Consume)

```
OmniContext Stack (owned IP)
├── AST Parser Engine (tree-sitter wrappers, chunking logic)
├── Hybrid Search Engine (BM25 + HNSW vector index)
├── ONNX Inference Layer (model-agnostic embedder)
├── MCP Protocol Server (tool definitions, streaming)
├── Daemon & Watcher (incremental indexing, file events)
└── Model Manager (auto-download, version management)

Consumed Libraries (commodity)
├── tree-sitter (parser generators -- MIT)
├── ONNX Runtime (Microsoft -- MIT)
├── HuggingFace ONNX models (open weights)
└── tantivy (full-text search -- Apache 2.0)
```

The critical owned IP is in layers 1-6. The commodity dependencies are all permissively licensed and widely available.

---

## 5. Enterprise Product Roadmap

### Phase 1: Foundation Complete (Current -- v0.2)

- [x] Local binary, single install, zero config
- [x] MCP server with full tool suite
- [x] Hybrid search (keyword + vector)
- [x] 20+ language support
- [x] VS Code extension (sidebar)
- [x] Daemon mode with incremental indexing

### Phase 2: Enterprise Readiness (Q2-Q3 2026)

**2.1 Server Mode**
Deploy OmniContext as a persistent service on a team's own infrastructure (on-prem or private cloud). One instance indexes a monorepo, the whole team queries it. No more each developer running their own index.

```
Architecture:
Developer Laptops ---query---> OmniContext Server (team-deployed)
                 <---results--       |
                                     |-- indexes from git remote
                                     |-- runs model inference
                                     |-- stores HNSW index on disk
```

**2.2 Auth & Access Control**

- SSO/SAML integration (Okta, Auth0, Azure AD)
- Per-repository access policies
- API key management for programmatic access
- Audit logs (who queried what, when)

**2.3 REST + gRPC API**
Beyond MCP, expose a proper REST API so any tool can query OmniContext. This opens integrations with:

- Custom CI/CD pipelines
- Internal developer portals
- Code review tools (GitHub PR comments with context)

**2.4 Multi-Repo Federation**
An enterprise doesn't have one repo. They have 50-500. OmniContext needs to:

- Index across multiple repos simultaneously
- Answer questions that span repos ("find all places that call the payments API regardless of which service they are in")
- Maintain separate indexes with unified search

### Phase 3: Domain Context Engines (Q4 2026 - Q1 2027)

This is the largest expansion and the most defensible moat.

**The Insight**: The context problem is not unique to code. AI agents working on ANY knowledge-intensive task need a persistent, searchable, semantic memory layer. Code is the first domain. But consider:

| Domain                             | Problem                                         | OmniContext Engine     |
| ---------------------------------- | ----------------------------------------------- | ---------------------- |
| Research Papers                    | Agents hallucinate citations, miss related work | Research Paper Engine  |
| Internal Docs (Notion, Confluence) | AI can't find the right internal doc            | Enterprise Docs Engine |
| API Documentation                  | AI writes wrong API calls, outdated syntax      | API Docs Engine        |
| Regulatory/Legal                   | Compliance AI needs to find exact regulations   | Legal Context Engine   |
| Customer Support Tickets           | AI support agents miss historical resolution    | Support History Engine |

Each domain engine is the same core architecture (AST replaced by domain-specific chunker), same ONNX embedding layer, same MCP interface. Building one domain engine well -- code -- gives you the template for all others.

**Research Paper Engine (First Expansion)**

Target customer: AI labs, academic institutions, R&D teams at companies like DeepMind, Microsoft Research, enterprise ML teams.

Technical differentiator:

- PDF parsing with LaTeX equation handling
- Citation graph traversal (related papers as edges)
- Section-aware chunking (Abstract, Methods, Results treated differently)
- Author/venue/year metadata filtering

This is 10x more useful for AI-assisted research than any general RAG system because it understands paper structure.

### Phase 4: OmniContext Cloud (2027)

For teams that don't want to self-host:

- Hosted indexing service
- Code never stored in cleartext (indexed into vectors only, source not retained)
- Usage-based pricing
- SOC2 / ISO 27001 compliance

This is the cloud transition that every successful on-prem developer tool makes (GitHub -> GitHub.com, GitLab self-hosted -> GitLab.com, Sourcegraph).

---

## 6. Go-To-Market Strategy

### Pilot to Enterprise Flywheel

```
Individual developer discovers OmniContext (open source)
    -> Installs locally, loves it
    -> Shows team in Slack ("this thing is insane")
    -> Team starts using MCP locally
    -> Someone asks "can we run this on a shared server?"
    -> Engineering manager pays for Enterprise license
    -> CTOs at company starts asking "can we do X with this?"
    -> Expand to more repos, more teams
```

This is the exact GTM that tools like Terraform, Sentry, and Grafana used. Open source as the distribution layer, enterprise as the revenue layer.

### Developer-Led Growth (DLG) Tactics

1. **GitHub presence**: OmniContext on GitHub with excellent docs, compelling README, and easy install. Target 5K stars before any fundraise.

2. **Reddit/HN**: Post "Show HN: I built a local MCP server that gives Claude persistent memory of your codebase" -- this is a genuinely novel demo that will resonate.

3. **AI agent community**: The MCP ecosystem is exploding. Be the reference implementation for code context in every MCP tutorial, every Claude setup guide.

4. **YouTube/demos**: A 3-minute demo showing an AI agent correctly answering a complex architectural question on a large open-source codebase (e.g., the Linux kernel, CPython, or Kubernetes) using OmniContext is a viral asset.

5. **Partnerships**: Direct integrations with Cursor, Zed, and VS Code extension marketplace. These are not technical partnerships -- they are distribution agreements. We are already an MCP server, the integration is trivial on their end.

### Target Customer Segments

**Segment 1: AI-Forward SMBs (0-200 engineers)**

- Already using Cursor/Claude heavily
- Don't have a dedicated DevOps team for complex RAG infra
- Price-sensitive but will pay for something that works
- Target price: $500-2,000/month
- ACV: $6,000 - $24,000

**Segment 2: Enterprise Engineering Teams (200-5,000 engineers)**

- Large monorepos where existing tools break down
- Strict security requirements (no cloud code exposure)
- Procurement cycles are slow but deal sizes are large
- Target price: $50,000 - $500,000/year
- ACV: $50K - $500K

**Segment 3: AI Labs & Research Organizations**

- Need the Research Paper Engine expansion
- Willing to be design partners (help us build the right features)
- Well-funded and fast to decide
- ACV: $20K - $100K

---

## 7. Revenue Model

### Tiered Pricing

**Tier 0: Open Source (Free, Forever)**

- Local-only, single user
- Full feature set for individual developers
- No limits on repos or index size
- This is the growth engine, not a revenue killer

**Tier 1: Team ($299/month, up to 10 developers)**

- Shared server deployment
- Up to 5 repos
- Standard support
- Web dashboard for index status

**Tier 2: Business ($999/month, up to 50 developers)**

- Unlimited repos
- SSO/SAML
- REST API access
- Priority support
- Multi-repo federation

**Tier 3: Enterprise (Custom, starts at $50K/year)**

- Self-hosted or private cloud
- SLA guarantees
- Compliance reports (SOC2, ISO 27001)
- Dedicated account manager
- Custom domain engines
- API rate limit controls
- White-label option

**Tier 4: API Usage (for AI platforms integrating OmniContext)**

- Per-query pricing: $0.001-0.01 per search query
- For AI agent platforms that want to embed OmniContext intelligence

### Revenue Milestones

| Milestone      | ARR Target    | What Triggers It                    |
| -------------- | ------------- | ----------------------------------- |
| Pre-seed close | $0 ARR        | 1,000 GitHub stars, working product |
| Seed close     | $50K-200K ARR | 10-20 paying teams                  |
| Series A       | $1M-3M ARR    | 50-100 enterprise customers         |
| Series B       | $10M+ ARR     | proven multi-domain expansion       |

---

## 8. Funding Strategy

### How Much to Raise and When

**Round 1: Pre-Seed -- $300K-700K**

You do not need to raise this. This is bootstrap + angels.

What to use it for:

- Full-time Mayank for 12-18 months
- One additional engineer (Rust or infrastructure)
- Cloud infra for hosted demos and SOC2 prep
- Legal (incorporation, IP assignment, terms of service)

Where to get it:

- Angel investors from the developer tools space (ex-GitHub, ex-Atlassian, ex-JetBrains engineers who became angels)
- Indian angel networks (AngelList India, LetsVenture) -- lower dilution, faster decisions
- Friends & family if applicable
- Bootstrapping through consulting (1-2 enterprise design partner contracts at $20-50K each buys the same runway)

Dilution target: 5-10% if you raise this round. Do not give away more.

---

**Round 2: Seed -- $1.5M-4M**

When to raise: After 5-10 paying teams, clear product-market fit signal, 2,000+ GitHub stars.

What to use it for:

- Engineering team: 3-5 engineers
- First go-to-market hire (developer relations or sales engineer)
- Server infrastructure for enterprise deployments
- Building the Research Paper Engine as second domain

Dilution target: 15-20%

Who to target:

- **Y Combinator** -- Apply to S2026 batch. OmniContext is a strong YC application: Rust, AI, developer tools, solo founder with working product. YC writes $500K checks now. The network effect of YC for enterprise sales is enormous.
- **a16z Seed** -- a16z has a $25M dedicated developer tools fund and they led investments in Sourcegraph, Linear, and Vercel.
- **Craft Ventures** -- Developer tools focused, David Sacks was PayPal and invested in Slack and GitHub ecosystem companies.
- **Accel** -- Strong enterprise SaaS track record in India + US.
- **Boldcap / Chiratae (India)** -- For an India-based founder, these are the top-tier seed funds with US reach.

The pitch framing for seed: "We are building the context layer that every AI agent needs to work inside real enterprise codebases. The context problem is the #1 blocker to AI agents actually shipping code autonomously. We've solved it locally. Now we're scaling it to teams."

---

**Round 3: Series A -- $10M-20M**

When to raise: $1-3M ARR, clear land-and-expand motion, second domain engine live.

Who to target at this stage:

- **Sequoia** -- Early invested in GitHub and Sourcegraph-adjacent tools
- **GV (Google Ventures)** -- Obvious strategic synergy with Google's AI products
- **Index Ventures** -- Strong European/international developer tools portfolio
- **Insight Partners** -- Growth-stage developer tools investors

---

### Bootstrapping Alternative

If raising venture capital is not desired immediately, there is a legitimate bootstrap path:

1. Find 2-3 enterprise design partners willing to pay $10-30K for an annual license + custom features
2. Use contract revenue to fund development
3. Build to $500K ARR before taking any VC money -- at that point you raise from a position of strength, not need

This path takes 18-24 months longer to scale but preserves 35-40% more equity.

---

## 9. Who to Pitch and What to Say

### The Pitch Structure (12 Minutes)

**Minute 1-2: The Pain (Make them feel it)**

> "You're using Claude or Cursor to work on your codebase. You ask it something about how your authentication service connects to the billing system. It gives you a confident answer that's completely wrong -- because it only saw 3 of the 47 files it needed to see. You've hit the context wall. Every developer using AI tools hits this wall, every day, on every complex task. The AI is smart. It just can't see your code."

**Minute 3-4: The Market**

> "This is not a niche problem. 30 million developers are now using AI coding tools. 100% of them work in codebases too large to fit in a context window. The AI agent market is projected to be $50B by 2030. All of it is bottlenecked by context. Whoever builds the best context layer doesn't just win a tool -- they win the infrastructure position in the AI coding stack."

**Minute 5-7: The Product Demo**

Live demo. Take a large public codebase (Kubernetes, CPython). Show asking Claude "how does the scheduler reconcile pod states?" WITHOUT OmniContext (Claude guesses, gets it partially wrong). Then WITH OmniContext (Claude cites exact files, exact functions, precise answer in seconds).

This demo closes rooms. It is visceral and immediately relatable to any technical investor.

**Minute 8-9: The Technical Moat**

> "We run all of this locally in a single Rust binary. No API keys. No cloud dependency. No code leaves the developer's machine. We are an MCP server, so we plug into every AI tool that speaks MCP -- Claude, Cursor, Cline, and every future AI agent that will exist. Our moat is the combination of AST-level parsing, hybrid search, and local ONNX inference running at sub-100ms latency. No one else has packaged all of this together."

**Minute 10-11: Business Model & Traction**

Traction to show before each round:

- Stars on GitHub
- Weekly active users / installs
- Design partner names (anonymized if needed)
- ARR / pipeline

**Minute 12: The Ask**

Be specific. "We are raising $X at a $Y pre-money valuation. We will use this to [3 specific things]. In 18 months, we will have [3 specific outcomes]."

---

### Investor List by Stage

**Angels to target first (developer tools credibility)**

| Name                                            | Why                             | How to Reach                      |
| ----------------------------------------------- | ------------------------------- | --------------------------------- |
| Ex-GitHub engineering VPs                       | Deep network in dev tools       | LinkedIn, Twitter/X               |
| Ex-Sourcegraph founders                         | Direct domain experience        | Cold email with demo video        |
| Developer tool angels (Beyang Liu, Quinn Slack) | Built this market               | AngelList, Crunchbase, warm intro |
| Indian dev tool founders who exited             | Nagarro, FusionCharts ecosystem | LinkedIn, AngelList India         |

**Seed VCs with developer tools focus**

- Primary: Y Combinator, a16z Seed, Craft Ventures, Boldcap
- Secondary: Nexus VP, Stellaris VP, Accel India

**Strategic Angels (potential acquirers sending scouts)**

Anthropic, Microsoft M12, JetBrains Investors -- these are corporate VCs who invest in companies they might later acquire. Having them on your cap table is optionality.

---

## 10. Acquisition Thesis

### Why Larger Companies Would Buy OmniContext

This is the analysis that sophisticated investors care about most: what is the realistic acquisition outcome, who buys it, and at what price?

### Tier 1 Acquirers (Most Likely, $100M-500M range)

**GitHub / Microsoft**

- They own the world's code. Their AI product (Copilot) struggles with large private codebases.
- OmniContext is exactly the missing context layer for GitHub Copilot Enterprise.
- Microsoft has a track record of acquiring developer tool infrastructure (npm, Semmle/CodeQL).
- Strategic fit: 10/10. GitHub Copilot with an OmniContext context layer becomes categorically better at private enterprise code.
- Acquisition signal: If Microsoft launches a "Copilot Workspace" product that needs persistent context, they either build or buy. Building this takes 18-36 months. Buying OmniContext is 6 months.

**Anthropic**

- Their entire enterprise business runs on Claude being useful inside real codebases.
- They are building tool integrations, MCP is their protocol.
- Acquiring the best MCP-native context engine is a defensive move against OpenAI.
- They are well-capitalized and have been acquiring talent and tools.
- Acquisition signal: If Anthropic launches an "enterprise codebase assistant" product, they need this.

**Cursor**

- Cursor's core product is AI pair programming. Their biggest user complaint is "it doesn't understand my full codebase."
- They are venture-backed with strong revenue ($100M+ ARR in 2025).
- Acquiring OmniContext would be an instant product differentiation from GitHub Copilot.
- Acquisition price: $50M-200M range (strategic acquisition, not financial).

### Tier 2 Acquirers ($50M-200M range)

**Sourcegraph**

- Direct competitor, but they are cloud-first and our architecture is the complement to theirs.
- They could acquire to enter the local-first / air-gapped enterprise market.
- Their existing Cody product needs a better context layer.
- Risk: They are also a competitor, so this has antitrust/culture risk.

**JetBrains**

- The world's leading IDE suite. Deep integration with developer workflow.
- AI features in IntelliJ/PyCharm are maturing. They need a context layer.
- They are profitable ($400M+ revenue) and have cash to deploy.
- They historically acquire (Datalore, etc.) rather than build from scratch.

**Atlassian**

- They own developer workflow (Jira, Confluence, Bitbucket).
- Their AI product "Atlassian Intelligence" needs to understand code context.
- The Research Paper Engine expansion also fits their documentation/knowledge use case.

### Tier 3 Acquirers (Longer term, $250M-1B range)

**Salesforce / Slack**

- Slack is where developer teams communicate. Context-aware dev tools inside Slack is a real product vision.
- Salesforce's AI platform (Einstein / Agentforce) needs code context for developer tools.

**Google / DeepMind**

- Google has Gemini Code Assist, Android Studio, and a massive enterprise cloud business.
- They would acquire to prevent Microsoft from having a monopoly on developer AI tools.
- Google has deep pockets and tends to acquire infrastructure companies.

### What Makes the Acquisition Attractive

1. **Working technology** -- not a prototype, a production binary with real users
2. **Defensible IP** -- the AST parsing + hybrid search + ONNX stack is non-trivial to replicate
3. **MCP network effects** -- as the ecosystem grows, OmniContext becomes more entrenched
4. **Talent** -- a Rust engineer who understands compilers, search, and AI inference is rare
5. **Data flywheel** -- usage patterns from enterprise customers are signal about what context AI agents need

### Acquisition Valuation Framework

At $3M ARR with strong growth: expect 15-25x ARR multiple = $45M-75M acquisition
At $10M ARR with expansion proof: expect 20-35x ARR multiple = $200M-350M acquisition
At $30M ARR as category leader: expect 25-50x ARR multiple = $750M - $1.5B acquisition

Comparable exits: Semmle (CodeQL) acquired by GitHub for ~$500M. Semgrep raised at $1B+ valuation. Snyk at $8.5B. All of these are developer tools with similar infrastructure positioning.

---

## 11. Financial Projections

### Conservative Case

| Year       | Customers | ARR       | Headcount   | Burn          |
| ---------- | --------- | --------- | ----------- | ------------- |
| 2026 (now) | 0 paying  | $0        | 1 (founder) | ~$2K/mo infra |
| 2026 Q4    | 5 teams   | $60K ARR  | 2           | $15K/mo       |
| 2027 Q2    | 25 teams  | $500K ARR | 4           | $40K/mo       |
| 2027 Q4    | 80 teams  | $2M ARR   | 8           | $120K/mo      |
| 2028 Q4    | 250 teams | $8M ARR   | 20          | $400K/mo      |

### Base Case (with Seed raise in mid-2026)

| Year    | Customers | ARR       | Headcount |
| ------- | --------- | --------- | --------- |
| 2026 Q4 | 15 teams  | $200K ARR | 3         |
| 2027 Q2 | 60 teams  | $1.5M ARR | 6         |
| 2027 Q4 | 200 teams | $4M ARR   | 12        |
| 2028 Q4 | 600 teams | $15M ARR  | 30        |

### Key Assumptions

- Average ACV of $20K (mix of Team and Business tiers)
- 85% gross margin (software + infra)
- 120% net revenue retention (expansion within accounts)
- 12-month sales cycle for Enterprise tier
- Monthly trials-to-paid conversion: 10-15%

---

## 12. Team and What We Need to Hire

### Current State: Founder-Only

Mayank has built the entire technical stack. This is both strength (full ownership, complete understanding) and risk (single point of failure, limits velocity).

### Hire 1: Senior Rust / Systems Engineer

**Why first**: The core engine needs to scale from single-user to multi-tenant. This requires someone who can handle HNSW index persistence, concurrent access patterns, and the server-mode architecture. This is not a junior position.

**What to look for**: Rust experience with search/database internals OR ex-Elasticsearch, ex-RocksDB, ex-ClickHouse engineers. They do not need AI/ML experience -- that is your domain.

**Compensation**: $120K-180K base + meaningful equity (5-8% for hire #1 ideally)

**Where to find**: Oxide Computer (Rust-focused but expensive), PingCAP (TiKV is Rust), Neon (Rust database), Rustaceans Discord/community.

### Hire 2: Developer Relations / Head of Community

**Why second**: The open source community is the GTM engine. Someone who can write excellent documentation, create demo videos, be present on Reddit/HN/Twitter/Discord, and translate developer feedback into prioritized features.

**What to look for**: Ex-developer at a startup who loves writing and community. NOT a traditional marketing person.

**Compensation**: $80K-120K + equity

### Hire 3: Enterprise Sales Engineer

**Why third**: When you have 10+ enterprise conversations happening simultaneously, you need someone who can run technical evaluations, POCs, and security reviews without your involvement on every call.

**What to look for**: Sold to engineering teams before. Understands Procurement, InfoSec, and legal at enterprise companies.

**Compensation**: $100K base + commission + equity

### What Mayank Should NOT Delegate (for 18 months)

- Core search algorithm decisions
- Model integration strategy
- Technical architecture of new domain engines
- Key investor/acquirer relationships
- Product vision and roadmap

---

## 13. Risks and Mitigations

### Risk 1: OpenAI / Anthropic Build This Themselves

**Probability**: Medium  
**Impact**: High  
**Mitigation**: Move fast on enterprise features (SSO, on-prem, compliance). OpenAI and Anthropic build for their cloud. They cannot easily build a credible on-prem solution for air-gapped enterprise. The enterprise segment that cares about code not leaving their environment is structurally protected.

### Risk 2: GitHub Copilot Workspace Catches Up

**Probability**: High (they are working on this)  
**Impact**: High  
**Mitigation**: GitHub's version will be cloud-only and GitHub-centric. We are multi-repo, multi-platform, local-first. Also, this increases the acquisition probability -- if GitHub is building this, they might prefer to acquire.

### Risk 3: Not Enough Revenue to Raise Next Round

**Probability**: Medium  
**Impact**: High  
**Mitigation**: Design partner contracts before the product is complete. Get 2-3 companies to pay $10-30K for early access and custom features. Use that ARR as proof for seed investors.

### Risk 4: Larger Embedding Models Make Current Architecture Obsolete

**Probability**: Low (3yr horizon)  
**Impact**: Medium  
**Mitigation**: The architecture is model-agnostic by design. Swapping the embedding model is a config change. The search, parsing, and indexing layers have independent value regardless of which model runs inference.

### Risk 5: Solo Founder Risk

**Probability**: Present (not a future risk)  
**Impact**: Medium  
**Mitigation**: Investor confidence decreases with solo founders. The mitigations are: (1) build strong investor relationships that function as a sounding board, (2) hire engineer #1 fast after initial raise, (3) be transparent with investors about this risk and the mitigation plan.

---

## 14. The Dream: What This Becomes

Let me be direct about the long arc. This is what you put in front of investors who want to understand the maximum outcome.

### The Context Layer of the AI-Native Enterprise

Today: OmniContext helps a developer ask Claude a better question about their codebase.

18 months: OmniContext runs on enterprise servers, giving entire engineering teams persistent, semantic awareness of 10M+ line codebases.

3 years: OmniContext has domain engines for code, research papers, internal docs, regulatory documents, customer support -- any knowledge-intensive domain where AI agents need context. Enterprises run their own private AI brain that understands their business.

5 years: OmniContext is the infrastructure layer -- the "Elasticsearch for AI agents." Every autonomous AI agent that operates inside an enterprise queries OmniContext for context before acting. The product is invisible, but it is in the critical path of every AI action taken inside a Fortune 500 company.

### The Analogy That Resonates with Investors

> Think about what Elasticsearch did for log search and observability. Before it, companies were grep-ing through logs. After it, entire industries of monitoring tools (Datadog, Splunk, New Relic) were built on top of it. Elasticsearch became infrastructure.
>
> OmniContext is doing the same thing for AI context. Today there is no standard, fast, private way to give an AI agent memory of an organization's knowledge. We are building that standard. The companies that build on top of us -- AI agents, AI coding tools, AI research assistants -- will be the next generation of Datadogs.
>
> We are not building a product. We are building the infrastructure layer that the AI agent economy runs on.

### Why This Specifically Can Be Worth $1B+

The value of context infrastructure scales with the number and capability of AI agents. In 2026, there are hundreds of millions of AI queries being made by agent frameworks. In 2030, there will be trillions. Every single one of those queries needs context. If OmniContext is the system that provides that context for even 0.1% of enterprise AI queries at $0.001 each, that is $X million per day.

More importantly: once an enterprise's knowledge is indexed in OmniContext, migration is painful. The switching cost is high. Usage compounds as more agents adopt the context layer. This is the definition of an infrastructure business with durable revenue.

---

## Immediate Next Steps (Priority Order)

1. **GitHub stars push**: Ensure public repo is live, README is excellent, post to HN and relevant subreddits. Target 1K stars before any investor conversation.

2. **Design partner outreach**: Email 20 engineering managers at startups you respect. Offer 6 months free Enterprise access in exchange for weekly feedback. Get 2-3 to commit.

3. **YC application**: Apply to S2026 or W2027 batch. Deadline typically in Jan (W) or July (S). The application itself forces clarity of thinking.

4. **Legal foundation**: Delaware C-Corp incorporation (if doing US fundraising), IP assignment agreement, terms of service, privacy policy for the hosted product.

5. **The killer demo**: Record a 3-minute video showing OmniContext transforming an AI agent's answer quality on a real, complex, public codebase. This is your top-of-funnel asset.

6. **Research Paper Engine MVP**: Pick one famous AI paper (Attention is All You Need, or the RAFT paper), build a demo where an AI agent answers deep technical questions about it using OmniContext. This shows the domain expansion thesis.

---

_This document should be treated as a living strategy document. Review and update quarterly as traction data, competitive landscape, and funding market conditions evolve._

_For investor conversations: this full document is internal. The pitch deck (separate file) should be a 10-15 slide summary drawing from sections 1, 2, 3, 6, 8, 10 of this document._
