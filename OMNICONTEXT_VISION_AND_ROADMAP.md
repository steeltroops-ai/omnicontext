# OmniContext Vision & Strategic Roadmap

## 1. Executive Summary & Mission Doctrine

The objective is to establish OmniContext as the de facto codebase intelligence layer—the foundational engine that powers AI interactions globally. It must operate as the invisible, zero-latency engine on individual developers' laptops while possessing the architectural scalability to replace expensive, closed-source enterprise indexing solutions (e.g., Sourcegraph, GitHub Copilot Enterprise).

**Our Success Condition:** Total ubiquity. OmniContext becomes the standard MCP (Model Context Protocol) component installed on every machine and deployed in every corporate VPC, offering state-of-the-art context traversal with absolute data privacy.

---

## 2. Current Architectural Audit (The SOTA Baseline)

OmniContext is structurally superior to standard "RAG for Code" patterns. By operating from first principles of code semantics rather than treating code as raw text, it achieves state-of-the-art context assembly.

### A. Parser & Semantic Chunker

- **Implementation:** `tree-sitter` AST generation (`parser/mod.rs`, `chunker/mod.rs`).
- **Evaluation:** **EXCELLENT**. Naive text splitting destroys code context. OmniContext splits strictly at language-aware boundaries (functions, classes) and prepends context headers (module imports, parent class signatures). This guarantees LLMs maintain semantic closure over isolated chunks.

### B. Embedding & Reranking

- **Implementation:** Local ONNX runtime (`embedder/mod.rs`, `reranker/mod.rs`).
- **Evaluation:** **ADVANCED**. The pipeline executes fast vector retrieval using `jina-embeddings-v2-base-code` followed by highly accurate cross-encoder reranking using `ms-marco-MiniLM-L-6-v2`. This explicit pipelining is a massive differentiator over pure Bi-encoder setups mapping sigmoid probabilities correctly.

### C. Dependency Graph Context

- **Implementation:** `petgraph` dependency mapping (`graph/mod.rs`).
- **Evaluation:** **INDUSTRY-LEADING**. OmniContext executes zero-cost contextual enrichment by dynamically altering search scores via proximity boosting. If `Chunk A` is a 1-hop neighbor of a directly queried symbol, its score inflates. This perfectly replicates "Anthropic Contextual Retrieval" without the massive LLM token overhead normally required to generate contextual summaries.

### D. Search Fusion & Context Assembly

- **Implementation:** Hybrid search (Vector + FTS5 Keyword + Symbol Lookup) fused via RRF (`search/mod.rs`).
- **Evaluation:** **ROBUST**. Token limits trigger intelligent file grouping. If multiple chunks hit in a single file, the engine fetches the entire file context, mimicking how senior engineers actually read code.

### E. Extension & IDE Integration (VS Code)

- **Implementation:** TypeScript extension acting as a thin IPC client over the Rust daemon (`editors/vscode/src/extension.ts`).
- **Evaluation:** **HIGH-POTENTIAL**. Operating the extension as a thin client via named pipes (`net.Socket`) to a persistent Rust daemon is the objectively correct architecture for memory bridging. Native hooks into VS Code's `vscode.chat.createChatParticipant` allow OmniContext to silently hijack and enrich Copilot/native chat queries without clunky WebViews. The built-in `runSyncMcp` logic to automatically update `claude_desktop_config.json` positions the extension uniquely as a central "MCP Command Center." The primary flaw is deployment friction: it relies on the user independently compiling or acquiring the `omnicontext` binary, rather than bundling it.

### F. Distribution & Pipeline

- **Implementation:** GitHub Actions matrix builds & automated Semantic Versioning (`.github/workflows/release.yml`).
- **Evaluation:** **COMPETENT BUT FRAGMENTED**. The pipeline brilliantly merges conventional commits to automate Rust binary compilation (Win/Mac/Linux) alongside VSCE/OpenVSX extension publishing. However, distributing raw `.zip` files is not an execution-grade strategy for developers. It lacks package manager deployments (Homebrew tap, NPM wrapper, `cargo-binstall`), which is the primary vector for capturing the individual developer market via zero-friction installation.

---

## 3. Market Comparison

| Feature                   | OmniContext (Local)         | Cursor (Codebase Indexing)   | Sourcegraph Cody            | GitHub Copilot Workspace |
| :------------------------ | :-------------------------- | :--------------------------- | :-------------------------- | :----------------------- |
| **Parsing Strategy**      | Strict AST (`tree-sitter`)  | AST / Heuristic              | Strict AST (SCIP)           | Heuristic / Fragmented   |
| **Retrieval Type**        | Hybrid RRF + Cross-Encoder  | Hybrid + Reranking           | Hybrid (Vector + Keyword)   | Dense Vector + BM25      |
| **Dependency Context**    | Dynamic Graph 1-hop         | Limited local context        | Global Code Graph           | Limited workspace graphs |
| **Execution Environment** | **100% Local / Air-gapped** | Cloud (Server-side indexing) | Cloud or Enterprise On-Prem | Cloud                    |
| **Token Optimization**    | Intelligent File Grouping   | Chunk-based                  | Dynamic                     | Snippet extraction       |

---

## 4. Phase 1: The Universal Developer Standard

To dominate the individual developer market, OmniContext must optimize for **Zero Friction, Resource Efficiency, and Universal Integration**.

### 4.1. The "Zero-Config" MCP Trojan Horse

Everyday developers do not want to run standalone indexing services. OmniContext serves as the definitive **Local MCP Server**. Wait for Claude Desktop, Cursor, or Continue.dev to query it silently. The installation must be a single command that configures the MCP settings out of the box.

### 4.2. Aggressive Resource Optimization

- **INT8 Quantization:** Move all ONNX models (`f32`) to `int8` quantization. This reduces the VRAM/RAM footprint by ~4x with negligible accuracy loss, ensuring laptop batteries and build systems are unaffected.
- **Idle Unloading:** The ONNX `session_pool` must drop models from memory instantly if no searches occur within 5 minutes.
- **Debounced AST Parsing:** The file watcher cannot re-index the graph globally on every save. It will compute and push AST diffs to SQLite incrementally holding CPU utilization around ~1-5%.

### 4.3. The Professional "Command Center" Sidebar (Anti-Black-Box UX)

The current VS Code Webview sidebar (`sidebarProvider.ts`) displays basic daemon stats (memory, cache hits). This is insufficient for an enterprise-grade AI tool. The sidebar must be transformed into an interactive "Command Center" that controls the engine's physics and demystifies its logic.

**Required Sidebar Components:**

1. **The Context Inspector (Critical):** The sidebar must expose the routing logic. When an LLM triggers a query, the sidebar must visually log the results: _"Injecting `auth.rs (lines 40-90)` (Semantic Match 0.92, 1-Hop Dependency)"_. Showing this data builds absolute trust; developers can see _why_ the AI knows their code.
2. **Environment & Health Hub:** One-click diagnostics. Show ONNX runtime status, local vector db health, and expose the existing "Repair Environment" logic via a prominent UI button.
3. **Index Management:** Visual timeline/progress bars for background AST chunking. UI toggles to force a Re-Index or Clear Index, and visual indicators of how many tokens are currently burned into the SQLite DB.
4. **Context Budget Sliders:** Live UI slider to adjust the `Token Budget` (e.g., 2K, 8K, 32K) so developers can restrict context injections if their LLM is hallucinating or costs are too high.
5. **Client Sync Dashboard:** Visual indicators showing exactly which local AI clients (Cursor, Claude Desktop, Continue) are currently synced to the OmniContext MCP port.

### 4.4. Interactive Intent Search & Framework Heuristics

- Replace standard `Cmd+Shift+F` with Intent Search (`Cmd+Shift+O`).
- **Pre-baked Framework Weights:** Inject semantic structural weights `+0.2` if `page.tsx` (Next.js), `main.rs` (Axum), or `app.py` (FastAPI) are discovered. OmniContext exhibits "senior-level intuition" globally for standard architectures out of the box.

---

## 5. Phase 2: The Enterprise Standard (BYOC)

To supplant commercial SaaS, OmniContext must transition from a local SQLite application to a distributed corporate indexing layer.

### 5.1. The Multi-Repository Boundary (Global Code Graph)

- **The Problem:** Enterprise services are fractured across hundreds of micro-repositories.
- **The Solution:** Establish a unified "Project Workspace" that spans multiple physical Git repositories. `petgraph` dependency graphs must resolve cross-repo imports, defining a global topology synonymous with SCIP (Shared Code Information Protocol).

### 5.2. Client-Server Architecture (Headless Serving)

- **The Problem:** Indexing 10-million line codebases locally is mathematically unscalable.
- **The Solution:** Decouple the client. Deploy OmniContext Server to the enterprise's internal Kubernetes cluster (Bring Your Own Compute). It listens to Git webhooks, computes massive embeddings on corporate GPUs, and acts as the central intelligence node. The developer's MCP operates as a thin client via gRPC/REST.

### 5.3. Document-Level Security (Role-Based Access Control)

- **Security Imperative:** Developers cannot traverse code they are not authorized to view.
- **The Fix:** Inject Access Control Lists (ACLs) into every SQLite chunk and vector. The `search/mod.rs` retrieval pipeline must enforce strict DLS pre-filtering before LLM context assembly.

### 5.4. Pluggable Storage Backends

- Expand beyond local SQLite (`MetadataIndex`) and In-Memory HashMaps (`VectorIndex`).
- Introduce abstract storage drivers enabling PostgreSQL metadata storage, and Qdrant/Milvus enterprise scalable vector databases for terabyte-level vector querying.

### 5.5. Semantic Observability & Telemetry

- Deploy `PlattCalibration` via live telemetry loops. Provide enterprise admins dashboards illustrating LLM context utility vs. ignorance, auto-tuning the Reranker to match internal codebases structure dynamically and perfectly.

---

## 6. Execution Milestones

1. **Q1: Local Domination**
   - Ship VS Code Sidebar GUI / OmniContext Inspector.
   - Implement INT8 ONNX Quantization.
   - Solidify the MCP standard for all AI-enabled IDEs.

2. **Q2: The Free Enterprise Standard (BYOC Pitch)**
   - Architect Headless Mode (Docker Compose / Helm chart).
   - Pluggable PostgreSQL & Qdrant integration.
   - Launch self-hosted "Air-Gapped" enterprise deployments.

3. **Q3/Q4: Global Topology & Telemetry**
   - Cross-Repository `petgraph` integration.
   - Full RBAC integration into retrieval pipelines.
   - Telemetry-based Reranker Auto-Tuning (`PlattCalibration` live loop).

---

## 7. Future Intelligence Architecture (SOTA Frontier)

To maintain absolute superiority over emerging Copilot updates and enterprise vectors, OmniContext's core physics must incrementally adopt the definitive State-of-the-Art (2025/2026).

### A. Binary Vector Quantization (Immediate Leverage)

- **Current Limitation:** Sparse matrices (`f32` vectors) waste RAM.
- **The SOTA Upgrade:** Evolve the `embedder` layer to utilize 1-bit **Binary Vectors** (using models like `nomic-embed-text-v1.5`). Distance is calculated via `Hamming Distance` (XOR logic) rather than Cosine Similarity.
- **The Outcome:** 10,000,000 vectors searched in ~5 milliseconds on a MacBook CPU, consuming mere kilobytes of RAM instead of gigabytes.

### B. Graph RAG (Hierarchical Code Traversal)

- **Current Limitation:** `petgraph` executes 1-hop static boundaries (A calls B). Multihop tracing fails because isolated variable flows drop out of keyword space.
- **The SOTA Upgrade:** Extract a highly typed Semantic Knowledge Graph. Extract entities (Classes, Traits, DB schemas) and serialize the _execution control flow_ (CFG).
- **The Outcome:** When a query says "How does an API request flow to the database?", OmniContext calculates the subgraph execution trace, natively answering deep architectural questions that basic vector RAG fundamentally fails at.

### C. ColBERT (Late Interaction Encoding)

- **Current Limitation:** Dense vectors crush entire functions into a single 256/768-D representation, destroying exact variable names and specific syntax nuances.
- **The SOTA Upgrade:** Integrate ColBERT architecture (`jina-colbert-v1-en`). ColBERT executes "Late Interaction" by encoding _every single token_ individually.
- **The Outcome:** Unprecedented accuracy. It merges keyword-level exact matching (BM25) with vector-level semantic comprehension in a monolithic retrieval pass over code syntax.

### D. DiskANN / Vamana Index

- **Current Limitation:** HNSW vector graphs must exist entirely in memory. This physically limits the size of the monorepo an individual developer can locally index.
- **The SOTA Upgrade:** Replace `hnsw.rs` with a **Vamana Graph (DiskANN)** implementation.
- **The Outcome:** DiskANN operates with 95% of the vectors persisted dynamically to inexpensive SSDs while servicing sub-3ms billion-scale retrievals. OmniContext instantly scales from "Local Workspaces" to "Terabyte Enterprise Histories" without crashing the host CPU.
