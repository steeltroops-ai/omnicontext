/**
 * Shared TypeScript interfaces for OmniContext VS Code extension.
 */

/**
 * IDE event sent to daemon for pre-fetch.
 * Enhanced with LSP-resolved symbol metadata for precise context pre-loading.
 */
export interface IdeEvent {
  event_type: "file_opened" | "cursor_moved" | "text_edited";
  file_path: string;
  cursor_line?: number;
  /** Symbol name at cursor */
  symbol?: string;
  /** Fully qualified name from LSP DocumentSymbol tree */
  symbol_fqn?: string;
  /** Symbol kind (Function, Class, Method, etc.) */
  symbol_kind?: string;
  /** Type signature from LSP hover (e.g., "fn foo(x: i32) -> bool") */
  type_signature?: string;
  /** File where symbol is defined (for cross-file pre-fetch) */
  definition_file?: string;
  /** Line where symbol is defined */
  definition_line?: number;
}

/**
 * Cache statistics from daemon.
 */
export interface CacheStats {
  hits: number;
  misses: number;
  size: number;
  capacity: number;
  hit_rate: number;
}

/**
 * Configuration for EventTracker.
 */
export interface EventTrackerConfig {
  enabled: boolean;
  debounceMs: number;
  maxQueueSize: number;
}

/**
 * Pre-flight context response.
 */
export interface PreflightResponse {
  system_context: string;
  entries_count: number;
  tokens_used: number;
  token_budget: number;
  elapsed_ms: number;
  cache_hit?: boolean;
  from_cache?: boolean;
}

/**
 * Engine status from daemon or CLI.
 */
export interface EngineStatus {
  repo_path: string;
  search_mode: string;
  files_indexed: number;
  chunks_indexed: number;
  symbols_indexed: number;
  vectors_indexed: number;
  dep_edges: number;
  graph_nodes: number;
  graph_edges: number;
  has_cycles: boolean;
  language_distribution?: Record<string, number>;

  // Reranking metrics
  reranker_enabled?: boolean;
  reranker_model?: string;
  reranker_latency_ms?: number;

  // Graph metrics
  graph_boosting_enabled?: boolean;
  pagerank_computed?: boolean;
  edge_types?: Record<string, number>;

  // Historical context
  commits_indexed?: number;
  co_change_patterns?: number;
  bug_prone_files?: number;

  // Embedder metrics
  quantization_mode?: 'fp32' | 'fp16' | 'int8';
  memory_usage_mb?: number;
  throughput_chunks_per_sec?: number;
}

/**
 * Resilience metrics from daemon.
 */
export interface ResilienceMetrics {
  circuit_breakers: {
    embedder: 'closed' | 'open' | 'half_open';
    reranker: 'closed' | 'open' | 'half_open';
    index: 'closed' | 'open' | 'half_open';
    vector: 'closed' | 'open' | 'half_open';
  };
  health_status: {
    parser: 'healthy' | 'degraded' | 'critical';
    embedder: 'healthy' | 'degraded' | 'critical';
    index: 'healthy' | 'degraded' | 'critical';
    vector: 'healthy' | 'degraded' | 'critical';
  };
  deduplication: {
    events_processed: number;
    duplicates_skipped: number;
    skip_rate: number;
  };
  backpressure: {
    active_requests: number;
    max_concurrent: number;
    rejected_requests: number;
    load_percent: number;
  };
}

/**
 * Graph metrics from daemon.
 */
export interface GraphMetrics {
  nodes: number;
  edges: number;
  edge_types: Record<string, number>;
  cycles: number; // Number of cycles detected (0 = none)
  pagerank_computed: boolean;
  max_hops: number;
  boosting_enabled: boolean;
}

/**
 * Historical context metrics from daemon.
 */
export interface HistoricalMetrics {
  commits_indexed: number;
  co_change_patterns: number;
  bug_prone_files: number;
  last_commit_indexed: string;
}


/**
 * Reranker metrics from daemon (Phase 1).
 */
export interface RerankerMetrics {
  enabled: boolean;
  model: string;
  latency_ms: number;
  improvement_percent: number;
  batch_size: number;
  max_candidates: number;
  rrf_weight: number;
}

/**
 * Search intent classification result (Phase 1).
 */
export interface SearchIntent {
  query: string;
  intent: 'architectural' | 'implementation' | 'debugging';
  confidence: number;
  hyde_applicable: boolean;
  synonyms_applicable: boolean;
}

// Phase 2: Resilience Monitoring Types
export interface ResilienceStatus {
  circuit_breakers: Record<string, CircuitBreakerState>;
  health_status: Record<string, HealthStatus>;
  deduplication: DeduplicationMetrics;
  backpressure: BackpressureMetrics;
}

export interface CircuitBreakerState {
  state: 'closed' | 'open' | 'half_open';
  failure_count: number;
  last_failure_time: number | null;
  next_attempt_time: number | null;
}

export interface HealthStatus {
  status: 'healthy' | 'degraded' | 'unhealthy';
  last_check_time: number;
  error_message: string | null;
}

export interface DeduplicationMetrics {
  events_processed: number;
  duplicates_skipped: number;
  in_flight_count: number;
  deduplication_rate: number;
}

export interface BackpressureMetrics {
  active_requests: number;
  load_percent: number;
  requests_rejected: number;
  peak_load_percent: number;
}

// Phase 4: Graph Visualization Types
// ---------------------------------------------------------------------------

export interface NeighborFileInfo {
  path: string;
  distance: number;
  edge_types: string[];
  importance: number;
}

export interface ArchitecturalContextResponse {
  focal_file: string;
  neighbors: NeighborFileInfo[];
  total_files: number;
  max_hops: number;
}

export interface CyclesResponse {
  cycle_count: number;
  cycles: number[][];
}

// Phase 5: Multi-Repository Support Types
// ---------------------------------------------------------------------------

export interface RepositoryInfo {
  path: string;
  name: string;
  priority: number;
  files_indexed: number;
  auto_index: boolean;
}

// Phase 6: Performance Controls Types
// ---------------------------------------------------------------------------

export interface EmbedderMetrics {
  quantization_mode: 'fp32' | 'fp16' | 'int8';
  memory_usage_mb: number;
  memory_savings_percent: number;
  throughput_chunks_per_sec: number;
  batch_fill_rate: number;
  batch_size: number;
  batch_timeout_ms: number;
}

export interface IndexPoolMetrics {
  active_connections: number;
  max_pool_size: number;
  utilization_percent: number;
  total_queries: number;
  avg_query_time_ms: number;
}

export interface CompressionStats {
  bytes_before: number;
  bytes_after: number;
  compression_ratio: number;
  savings_percent: number;
}
