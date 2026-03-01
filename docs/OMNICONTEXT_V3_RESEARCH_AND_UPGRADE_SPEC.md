# OmniContext v3: Enterprise Code Intelligence Research & Advanced Upgrade Specification

**Document Status:** DRAFT / RESEARCH  
**Target Architecture:** OmniContext v3 (Late 2025 / Early 2026 Target)  
**Primary Objective:** Evolve OmniContext from a simple RAG implementation to an enterprise-grade "Infinite Context Engine" leveraging state-of-the-art 2024-2025 research in Retrieval-Augmented Code Generation (RACG), GraphRAG, and Cross-Encoder architectures.

---

## 1. Executive Summary: The Code Intelligence Paradigm Shift

The landscape of AI coding assistants is shifting rapidly from flat, text-based semantic search to multi-dimensional, graph-augmented structural cognition. Based on critical analysis of industry leaders (Augment Code, Cursor AI, Sourcegraph Cody) and latest academic research (DeepCodeSeek, Graphcoder, CODEXGRAPH), OmniContext v2's current architecture has several immediate and critical gaps blocking enterprise-level accuracy.

OmniContext v3 must transition from **Keyword/Vector "Pull" RAG** to **Graph-Enriched "Push" Agentic RAG**. This document exhaustively defines the algorithmic research, the identified gaps, and the specific target outcomes required to achieve parity and establish superiority in the code intelligence space.

---

## 2. Critical Capability Gaps & Enterprise Target Outcomes

### 2.1 Code Representation: AST Micro-Chunking & Overlapping Context

**Current State (v2):** `Tree-sitter` parser currently extracts strict, isolated AST blocks with zero overlap.
**The Gap:** When the LLM retrieves a specific block, it lacks the surrounding declarative context (e.g., retrieving a loop without the enclosing function signature, or retrieving a struct without its trait definitions).
**2025 Research Consensus:** Papers like _CodeGRAG (2024)_ and recent systematic reviews on Code RAG emphasize that **AST Micro-chunking with overlapping contexts** is critical. Simply fragmenting AST nodes creates disjointed semantic breaks. Overlapping ensures the continuous narrative of the code's data flow is preserved across vector boundaries.
**Target Outcome (v3):**

- **Action:** Rewrite the `omni-core::Chunker` pipeline.
- **Algorithm:** Implement CAST (Chunking via Abstract Syntax Trees) logic with a defined token-overlap margin (e.g., 100-200 tokens). Each function/class chunk MUST overlap to include standard module-level declarations securely.
- **KPI Tracking:** Decrease entirely orphaned (contextless) code chunks to 0%. Prevent LLM type-hallucinations caused by missing localized context.

### 2.2 Retrieval Accuracy: The Transition to Two-Stage Pipelines

**Current State (v2):** A single-stage hybrid pipeline uniting BM25 and generic flat vector embeddings (ONNX generic). Results in uniform score distributions causing high token wastage.
**The Gap:** Lacks semantic understanding of context bounds and precise relevance scoring for code nuances.
**2025 Research Consensus:** Papers like _DeepCodeSeek (2025)_ and _Granite Embedding R2 Models (2025)_ unequivocally demonstrate that **Cross-Encoder Reranking** is mandatory. A bi-encoder (vector search) is used for fast recall, while a cross-encoder evaluates the exact code chunk against the query simultaneously, preventing LLM hallucination and drastically increasing MRR.
**Target Outcome (v3):**

- **Action:** Implement a Two-Stage Retrieval Pipeline via a new `omni-reranker` crate subsystem.
- **Algorithm:** Initial fast retrieval via HNSW + BM25, bounded to top-100 candidates. Post-processing via an ONNX-optimized Cross-Encoder model (e.g., ms-marco-MiniLM-L-6-v2 or a specialized code reranker).
- **KPI Tracking:** Increase MRR@5 from baseline (est. ~15%) to **>75%**. NDCG@10 must exceed **0.70**.

### 2.3 Semantic Understanding: GraphRAG and Code Graph Models (CGMs)

**Current State (v2):** `DependencyGraph` exists but operates purely on naive symbol names without structural tracking (0 populated edges).
**The Gap:** Treats code as flat text, entirely missing intent, data-flow, and execution paths.
**2025 Research Consensus:** 2024/2025 research including _Graphcoder_, _CodeGRAG_, and _CODEXGRAPH_ highlight that pure vector databases fail at structural reasoning. To solve this, **Program Dependence Graphs (PDGs)** must be extracted at AST-level, representing data-flow, control-flow, and module bounds.
**Target Outcome (v3):**

- **Action:** Deploy an enriched Hypergraph Store mapping AST relationships.
- **Algorithm:** Implement Graph-Based Relevance Propagation. Compute search hits and traverse outgoing edges (e.g., "calls", "instantiates", "implements") using custom weighting (e.g., PageRank influenced by initial BM25 score).
- **Scale:** Populate 5000+ deterministic edges on a medium repository to ensure responses know _exactly_ which structures are connected.

### 2.4 Local Execution at Scale: Scalar Quantization

**Current State (v2):** Full precision f32 embeddings generating ~1.5KB per chunk. Limits operational viability on standard developer laptops for multi-repo environments (100k+ files).
**The Gap:** A 100k-file repository consumes gigabytes of memory purely for the indexing cache.
**2025 Research Consensus:** 2025 high-performance vector implementations rely heavily on quantization to balance search accuracy and memory utilization. Systems like Sourcegraph are boasting 8x memory reductions via quantified vector algorithms.
**Target Outcome (v3):**

- **Action:** Optimize `omni-core::vector`.
- **Algorithm:** uint8 scalar quantization for embeddings. We convert f32 float vectors to 1-byte representations leveraging min/max normalization, accelerating dot product computations while reducing memory overhead by 4x.
- **KPI Tracking:** RAM utilization for 100,000 chunks must stay under **40MB** while maintaining a sub-200ms p95 querying latency.

### 2.5 Context Delivery: Intent-Aware Agentic RAG

**Current State (v2):** Pull model where LLM explicitely utilizes MCP `search_code` when it deems necessary. Static token budgeting.
**The Gap:** LLMs are often unaware of _when_ to search or how to structure queries properly for enterprise codebases.
**2025 Research Consensus:** Cursor and Augment Code utilize "Push" delivery and Agentic RAG. Specifically, _intent classification_ (Edit vs Explain vs Debug) dynamically shapes the context retrieval process without developer intervention.
**Target Outcome (v3):**

- **Action:** Expand Pre-Flight Context Injection (`omni-daemon`) to utilize Intent Classification.
- **Implementation:** Pre-fetch contexts proactively based on IDE cursor movements via speculative loading. If a user presses `Cmd+K` (Edit), fetch surrounding AST scope. If a user opens a chat (Explain), fetch the Module Map and Graph callers.
- **Speed Constraint:** Pre-flight daemon context assembly must be completely hidden from the user, operating in `< 100ms`.

### 2.6 Temporal Intelligence: Co-Change Graphing (Context Lineage)

**Current State (v2):** Unused Git integration.
**The Gap:** Search engines frequently return highly accurate but legacy (deprecated) code structures simply because they are highly linked.
**2025 Research Consensus:** Advanced analysis introduces _temporal edges_ into the Semantic Graph—ranking code chunks not just by matching text, but by their co-change frequency in version control (e.g., "These two files were modified in exactly the same commits 6 times over the last year").
**Target Outcome (v3):**

- **Action:** Temporal Indexing in `omni-core::commits`.
- **Algorithm:** Decrease the relevance score of code portions lacking recent temporal updates. Inject metadata detailing the last modifier and co-changed files directly into the LLM system prompt.

---

## 3. Algorithm Specification: Hybrid Contextual Retrieval

To achieve these benchmarks, OmniContext v3 will transition from a flat query lookup to the following execution sequence per request:

1.  **Query Decomposition & Intent Parsing**
    - _Natural language parsing_ into keywords.
    - _Synonym Expansion_ utilizing static code-concept maps (e.g., mapping "auth" -> ["authenticate", "verify", "JWT"]).
2.  **Broad Recall (The k-NN Bi-Encoder Stage)**
    - Use quantized HNSW Vector index alongside FTS5 SQLite index to grab the top `N=100` relevant code chunks in `< 20ms`.
3.  **Semantic Propagated Enrichment (The Graph Stage)**
    - Evaluate the `N` results against the repo Hypergraph.
    - Inject critical _sibling nodes_ (e.g., if a struct is found, auto-include its trait implementation) using PageRank-like distribution scores.
4.  **Deep Semantic Reranking (The Cross-Encoder Stage)**
    - Forward the `N` context pairs (Query + Enriched Chunk) into the Cross-Encoder ONNX network.
    - Calculate discrete relevancy scores. Sort and crop array to standard budget limits.
5.  **Assembly & Pre-Flight Delivery**
    - Pack the context perfectly against the dynamic `max_tokens` layout of the active Agent session format (e.g. Anthropic Claude 3.5 Sonnet context format parameters).

---

## 4. Implementation Phasing Roadmap

We must build Enterprise features systematically to preserve engine stability.

| Phase       | Milestone Name                    | High-Level Scope                                                                                                   | Targeted Gap Addressed       |
| :---------- | :-------------------------------- | :----------------------------------------------------------------------------------------------------------------- | :--------------------------- |
| **Phase 1** | **Two-Stage Retrieval**           | Integrate `omni-reranker` with ONNX bindings for Cross-Encoder re-score.                                           | 2.2 (Retrieval Accuracy)     |
| **Phase 2** | **Full CGMs & AST Edges**         | Overhaul Tree-sitter parsers. Implement AST Overlapping Micro-chunks. Feed to Graph Database.                      | 2.1 / 2.3                    |
| **Phase 3** | **Quantization At Scale**         | `uint8` vector conversion pipeline + indexing delta detection.                                                     | 2.4 (Local Execution Limits) |
| **Phase 4** | **Speculative & Context Lineage** | Background daemon IDE cursor tracking, Git history semantic analysis algorithm mapping.                            | 2.5 / 2.6                    |
| **Phase 5** | **Enterprise Deployment**         | Benchmarking across 500k+ file monorepos, MRR integration tests, multi-session parallel Agent concurrency via IPC. | All Enterprise Readiness     |

## 5. Bibliography & Foundational 2024-2025 Papers to Evaluate

To keep development strictly aligned with modern computer science, engine developers must reference the following publications continuously:

1.  **Esakkiraja et al. (2025).** _DeepCodeSeek: Real-Time API Retrieval for Context-Aware Code Generation._ (Cross-encoder reranking algorithms for extreme code precision).
2.  **Microsoft Research (2024).** _Project GraphRAG / From Local to Global: A Graph RAG Approach._ (Strategies for injecting topological knowledge back into text parameters).
3.  **Zhang et al. (2025).** _Codegrag: Bridging the gap between natural language and programming language via graphical retrieval augmented generation._ (AST-to-Graph conversion theories).
4.  **IBM (2025).** _Granite Embedding R2 Models architecture details._ (Optimizations for bi-encoder/cross-encoder splits).
5.  **Qiu et al. (2025).** _CODEXGRAPH: Bridging Large Language Models and Code Repositories via Code Graph Databases._ (Hypergraph structures dedicated explicitly to source-code scale mapping).
