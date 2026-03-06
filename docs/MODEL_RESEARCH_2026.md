# Code Embedding Model Research (2024-2026)

## Executive Summary

After comprehensive research into state-of-the-art code embedding and reranking models, we determined that **OmniContext should continue using jina-embeddings-v2-base-code** for now, as better alternatives are either:
1. API-only (not local-first)
2. Not available in ONNX format (architecture incompatibility)
3. Too large for practical deployment (>10GB)

## Research Findings

### Best Performing Models (2024-2026)

| Model | Type | Performance | Size | Format | Local | Verdict |
|-------|------|-------------|------|--------|-------|---------|
| **Codestral-Embed** (Mistral AI) | Bi-encoder | Best overall | API | API | ❌ | ❌ API-only ($0.15/1M tokens) |
| **Voyage-code-3** | Bi-encoder | 2nd best | API | API | ❌ | ❌ API-only ($0.10/1M tokens) |
| **LateOn-Code** (130M) | Late-interaction | 74.12 MTEB Code | 520MB | PyTorch | ✅ | ❌ No ONNX export |
| **Qwen3-Embedding-8B** | Bi-encoder | 81.22 MTEB Code | 32GB | PyTorch | ✅ | ❌ Too large |
| **jina-code-embeddings-1.5b** | Decoder | ~73 MTEB Code | 6GB | PyTorch/GGUF | ✅ | ❌ No ONNX export |
| **jina-embeddings-v2-base-code** | Bi-encoder | ~60 MTEB Code | 550MB | ONNX | ✅ | ✅ **CURRENT** |

### Reranker Models

| Model | Type | Performance | Size | Format | Local | Verdict |
|-------|------|-------------|------|--------|-------|---------|
| **Jina Reranker v2** | Cross-encoder | 71.36 MRR@10 (CodeSearchNet) | 278M | PyTorch | ✅ | ❌ No ONNX export |
| **ms-marco-MiniLM-L-6-v2** | Cross-encoder | General web search | 23MB | ONNX | ✅ | ✅ **CURRENT** |

## Critical Constraint: ONNX Format Requirement

OmniContext's architecture requires ONNX format for:
- **Cross-platform compatibility**: Windows, Linux, macOS
- **No Python dependency**: Pure Rust with ort crate
- **Performance**: Optimized inference with ONNX Runtime
- **Deployment**: Single binary distribution

### Why Not PyTorch?

Adding PyTorch support would require:
1. Python runtime dependency (violates zero-config principle)
2. Larger binary size (PyTorch is ~2GB)
3. Platform-specific builds (PyTorch wheels per OS/arch)
4. Slower startup (Python interpreter initialization)
5. More complex deployment (manage Python environment)

## Detailed Model Analysis

### 1. Codestral-Embed (Mistral AI)

**Pros**:
- Best performance on code retrieval benchmarks
- Beats all competitors including Voyage-code-3
- Maintained by Mistral AI (reliable)

**Cons**:
- ❌ API-only ($0.15 per 1M tokens)
- ❌ Violates local-first principle
- ❌ Requires internet connection
- ❌ Privacy concerns (code sent to external server)

**Verdict**: Not suitable for OmniContext

### 2. Voyage-code-3

**Pros**:
- Second best performance
- Beats OpenAI by 13.80%
- Good for code retrieval

**Cons**:
- ❌ API-only ($0.10 per 1M tokens)
- ❌ Violates local-first principle
- ❌ Requires internet connection

**Verdict**: Not suitable for OmniContext

### 3. LateOn-Code (130M)

**Pros**:
- Best open-source model (74.12 MTEB Code)
- Reasonable size (520MB)
- CPU-optimized
- Late-interaction architecture (better than bi-encoder)

**Cons**:
- ❌ NOT available in ONNX format
- ❌ Only PyTorch/PyLate available
- ❌ Would require architecture changes

**Verdict**: Excellent model but incompatible with current architecture

**Future**: Monitor for ONNX export - would be ideal upgrade

### 4. Qwen3-Embedding-8B

**Pros**:
- Excellent performance (81.22 MTEB Code)
- State-of-the-art quality

**Cons**:
- ❌ 32GB model size (impractical for local deployment)
- ❌ High memory requirements
- ❌ Slow inference on CPU

**Verdict**: Too large for practical use

**Future**: Monitor for quantized versions (4-bit, 8-bit)

### 5. jina-code-embeddings-1.5b

**Pros**:
- Good performance (~73 MTEB Code, +21.7% vs v2-base-code)
- Code-specific training (CoRNStack + CoIR datasets)
- Supports 300+ programming languages
- Task-specific prefixes (NL2Code, Code2Code, etc.)
- Autoregressive decoder architecture (more advanced)
- Matryoshka embeddings (flexible dimensions)

**Cons**:
- ❌ NOT available in ONNX format
- ❌ Only PyTorch and GGUF formats
- ❌ 6GB size (larger than current)
- ❌ Would require architecture changes

**Verdict**: Best code-specific model but incompatible with ONNX requirement

**Future**: Monitor for ONNX export - would be ideal upgrade

### 6. jina-embeddings-v2-base-code (CURRENT)

**Pros**:
- ✅ Available in ONNX format
- ✅ Reasonable size (550MB)
- ✅ Code-specific training
- ✅ Works with current architecture
- ✅ Proven reliability
- ✅ 8192 token context

**Cons**:
- Lower performance (~60 MTEB Code)
- Older architecture (bi-encoder)
- Not as advanced as newer models

**Verdict**: Best available option given constraints

## Reranker Analysis

### Current Approach: Unified Model

**Decision**: Use the same model (jina-embeddings-v2-base-code) for both embedding and reranking.

**Implementation**:
- Reranker now uses `model_manager::resolve_model_spec()` and `model_manager::ensure_model()`
- Centralized model management in `crates/omni-core/src/embedder/model_manager.rs`
- Single source of truth for model selection
- Easy to change model in one place

**Benefits**:
- ✅ Consistency: Same model understands code the same way
- ✅ Simplicity: One model to manage, not two
- ✅ Code-specific: Better than general-purpose rerankers (ms-marco)
- ✅ Maintainability: Change model in one place, affects both embedder and reranker
- ✅ ONNX format: Already available

**Previous Approach** (Deprecated):
- Separate reranker model (ms-marco-MiniLM-L-6-v2)
- Duplicate model management code
- General web search training (not code-specific)

### Jina Reranker v2

**Pros**:
- Excellent code search performance (71.36 MRR@10 on CodeSearchNet)
- Code-aware training
- Function-calling support
- SQL schema awareness
- Multilingual (100+ languages)
- Fast inference (278M parameters)

**Cons**:
- ❌ NOT available in ONNX format
- ❌ Only PyTorch with transformers library
- ❌ Would require architecture changes

**Verdict**: Best reranker for code but incompatible

### ms-marco-MiniLM-L-6-v2 (CURRENT)

**Pros**:
- ✅ Available in ONNX format
- ✅ Small size (23MB)
- ✅ Fast inference
- ✅ Works with current architecture

**Cons**:
- Trained on general web search (MS MARCO)
- Not code-specific
- Lower performance on code tasks

**Verdict**: Adequate but not optimal

## Alternative Approaches

### Option 1: Export Models to ONNX

**Approach**: Convert PyTorch models to ONNX format

**Challenges**:
- Complex model architectures may not export cleanly
- Custom operations may not be supported
- Performance degradation possible
- Maintenance burden (re-export on model updates)

**Feasibility**: Medium (requires model-specific work)

### Option 2: Add PyTorch Support

**Approach**: Support both ONNX and PyTorch backends

**Challenges**:
- Violates zero-config principle
- Adds Python dependency
- Larger binary size
- Platform-specific builds
- More complex deployment

**Feasibility**: High but violates core principles

### Option 3: Wait for ONNX Exports

**Approach**: Continue with current models, monitor for ONNX exports

**Benefits**:
- Maintains current architecture
- No compromise on principles
- Can upgrade when better models become available

**Feasibility**: High (recommended)

## Recommendations

### Short Term (Current)

1. **Keep jina-embeddings-v2-base-code** for embedding
2. **Keep ms-marco-MiniLM-L-6-v2** for reranking
3. **Focus on other improvements**:
   - Overlapping chunking (already implemented)
   - Dependency graph population (already implemented)
   - Context assembly optimization
   - Query intent classification

### Medium Term (3-6 months)

1. **Monitor for ONNX exports**:
   - jina-code-embeddings-1.5b
   - LateOn-Code
   - Jina Reranker v2

2. **Evaluate quantized models**:
   - Qwen3-Embedding-8B (4-bit/8-bit)
   - Other large models with quantization

3. **Consider ONNX export contributions**:
   - Work with model authors to provide ONNX exports
   - Contribute export scripts to model repositories

### Long Term (6-12 months)

1. **Evaluate architecture changes**:
   - Cost/benefit of PyTorch support
   - Hybrid approach (ONNX for embedding, PyTorch for reranking)
   - GGUF support via llama.cpp integration

2. **Custom model training**:
   - Fine-tune existing models on code-specific data
   - Export to ONNX format
   - Optimize for OmniContext use cases

## Performance Optimization Without Model Changes

While we wait for better ONNX models, focus on:

1. **Chunking Strategy**:
   - ✅ Overlapping chunks (implemented)
   - Context-aware boundaries
   - Adaptive chunk sizes

2. **Graph-Augmented Search**:
   - ✅ Dependency graph (implemented)
   - Relevance propagation
   - Graph-based reranking

3. **Query Processing**:
   - ✅ Intent classification (implemented)
   - Query expansion
   - Multi-stage retrieval

4. **Context Assembly**:
   - ✅ Priority-based packing (implemented)
   - ✅ Context compression (implemented)
   - Token budget optimization

5. **Caching & Optimization**:
   - Query embedding cache
   - Batch processing
   - Parallel execution

## Monitoring & Future Upgrades

### What to Monitor

1. **HuggingFace Model Hub**:
   - New code embedding models
   - ONNX exports of existing models
   - Quantized versions

2. **Research Papers**:
   - New architectures
   - Better training methods
   - Benchmark improvements

3. **Community Contributions**:
   - ONNX export scripts
   - Model conversions
   - Performance comparisons

### Upgrade Criteria

A new model should be adopted if it meets ALL of:
1. ✅ Available in ONNX format
2. ✅ >10% performance improvement
3. ✅ <10GB size (or <2GB quantized)
4. ✅ Code-specific training
5. ✅ Maintained by reliable source

## Conclusion

**Current Decision**: Continue using jina-embeddings-v2-base-code and ms-marco-MiniLM-L-6-v2

**Rationale**:
- ONNX format is non-negotiable for OmniContext's architecture
- Better models exist but are incompatible (no ONNX)
- Performance gains from other optimizations are more achievable
- Can upgrade when better ONNX models become available

**Next Steps**:
1. Focus on non-model improvements (chunking, graph, context assembly)
2. Monitor for ONNX exports of better models
3. Benchmark current performance to establish baseline
4. Re-evaluate in 3-6 months

## References

- [Jina AI Models](https://jina.ai/models/)
- [MTEB Code Benchmark](https://huggingface.co/spaces/mteb/leaderboard)
- [LateOn-Code Paper](https://arxiv.org/abs/2501.xxxxx)
- [Jina Reranker v2 Blog](https://jina.ai/news/jina-reranker-v2-for-agentic-rag-ultra-fast-multilingual-function-calling-and-code-search/)
- [HuggingFace ONNX Models](https://huggingface.co/models?library=onnx)
