/**
 * Shared TypeScript interfaces for OmniContext VS Code extension.
 */

/**
 * IDE event sent to daemon for pre-fetch.
 */
export interface IdeEvent {
    event_type: 'file_opened' | 'cursor_moved' | 'text_edited';
    file_path: string;
    cursor_line?: number;
    symbol?: string;
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
}
