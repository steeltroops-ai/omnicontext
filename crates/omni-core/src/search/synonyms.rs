//! Code vocabulary synonym expansion for BM25 query enrichment.
//!
//! When a user searches for "cache", they likely also want results containing
//! "memoize", "lru", or "ttl". This module provides a curated synonym map
//! for common code concepts, improving keyword search recall without
//! sacrificing precision (synonyms are added as OR terms, not replacements).
//!
//! ## Design decisions
//!
//! 1. Static map via `LazyLock` -- zero allocation after first access
//! 2. Asymmetric: "cache" -> ["lru", "memoize"] but NOT "lru" -> ["cache"]
//!    This prevents runaway expansion chains
//! 3. Only applied to `NaturalLanguage` and `Mixed` query types
//! 4. Capped at 3 synonyms per term to avoid query dilution

use std::collections::HashMap;
use std::sync::LazyLock;

/// Maximum number of synonyms to add per query term.
const MAX_SYNONYMS_PER_TERM: usize = 3;

/// Maximum number of total synonym terms to add to a query.
const MAX_TOTAL_SYNONYMS: usize = 6;

/// The curated code synonym map.
///
/// Each entry maps a common code concept to its most relevant synonyms.
/// Entries are ordered by frequency of occurrence in real developer queries.
static SYNONYM_MAP: LazyLock<HashMap<&'static str, Vec<&'static str>>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Data structures & patterns
    m.insert("cache", vec!["lru", "memoize", "ttl", "expire"]);
    m.insert("queue", vec!["fifo", "dequeue", "enqueue", "buffer"]);
    m.insert("stack", vec!["push", "pop", "lifo"]);
    m.insert("tree", vec!["node", "leaf", "parent", "child", "traversal"]);
    m.insert("graph", vec!["edge", "vertex", "adjacency", "dag"]);
    m.insert("list", vec!["array", "vec", "slice", "collection"]);
    m.insert("map", vec!["hashmap", "dictionary", "hashtable", "lookup"]);
    m.insert("set", vec!["hashset", "unique", "dedup"]);

    // Auth & security
    m.insert(
        "auth",
        vec!["authenticate", "authorization", "login", "jwt", "token"],
    );
    m.insert("login", vec!["authenticate", "sign_in", "session"]);
    m.insert("permission", vec!["rbac", "acl", "authorize", "role"]);
    m.insert("encrypt", vec!["decrypt", "cipher", "aes", "hash"]);
    m.insert("password", vec!["hash", "bcrypt", "argon2", "credential"]);

    // Error handling
    m.insert("error", vec!["exception", "failure", "fault", "panic"]);
    m.insert(
        "retry",
        vec!["backoff", "exponential", "attempt", "resilience"],
    );
    m.insert("timeout", vec!["deadline", "expire", "ttl"]);
    m.insert("validate", vec!["check", "verify", "assert", "constraint"]);

    // Database & storage
    m.insert("database", vec!["db", "sql", "query", "schema"]);
    m.insert("query", vec!["sql", "select", "filter", "search"]);
    m.insert("migrate", vec!["migration", "schema", "alter", "upgrade"]);
    m.insert("transaction", vec!["commit", "rollback", "atomic"]);
    m.insert("index", vec!["btree", "search", "lookup", "key"]);

    // API & networking
    m.insert("api", vec!["endpoint", "route", "handler", "rest"]);
    m.insert("request", vec!["http", "fetch", "call", "invoke"]);
    m.insert("response", vec!["status", "body", "header"]);
    m.insert("middleware", vec!["interceptor", "filter", "hook"]);
    m.insert("route", vec!["endpoint", "path", "handler", "url"]);
    m.insert("webhook", vec!["callback", "hook", "event", "notify"]);
    m.insert("socket", vec!["websocket", "tcp", "connection", "stream"]);

    // Async & concurrency
    m.insert("async", vec!["await", "future", "promise", "concurrent"]);
    m.insert("thread", vec!["spawn", "mutex", "lock", "concurrent"]);
    m.insert("mutex", vec!["lock", "guard", "rwlock", "sync"]);
    m.insert("channel", vec!["mpsc", "sender", "receiver", "message"]);
    m.insert("parallel", vec!["concurrent", "thread", "rayon", "fork"]);

    // Testing
    m.insert("test", vec!["assert", "expect", "mock", "fixture"]);
    m.insert("mock", vec!["stub", "fake", "spy", "double"]);
    m.insert("benchmark", vec!["perf", "measure", "profile", "timing"]);

    // Configuration
    m.insert(
        "config",
        vec!["configuration", "settings", "options", "env"],
    );
    m.insert("env", vec!["environment", "variable", "dotenv"]);
    m.insert("flag", vec!["feature", "toggle", "switch"]);

    // Logging & monitoring
    m.insert("log", vec!["logging", "trace", "debug", "info"]);
    m.insert("metric", vec!["counter", "gauge", "histogram", "measure"]);
    m.insert("trace", vec!["span", "telemetry", "opentelemetry"]);

    // Serialization
    m.insert(
        "serialize",
        vec!["deserialize", "encode", "decode", "marshal"],
    );
    m.insert("json", vec!["serde", "parse", "serialize", "deserialize"]);
    m.insert("parse", vec!["tokenize", "lex", "ast", "grammar"]);

    // Deployment & infra
    m.insert("deploy", vec!["release", "publish", "ci", "cd"]);
    m.insert("container", vec!["docker", "kubernetes", "pod"]);

    // Code patterns
    m.insert("trait", vec!["interface", "protocol", "abstract"]);
    m.insert(
        "interface",
        vec!["trait", "protocol", "abstract", "contract"],
    );
    m.insert("generic", vec!["template", "parametric", "type_param"]);
    m.insert("enum", vec!["variant", "union", "algebraic"]);
    m.insert("struct", vec!["class", "record", "dataclass"]);
    m.insert("closure", vec!["lambda", "anonymous", "callback"]);
    m.insert("iterator", vec!["iter", "next", "yield", "generator"]);
    m.insert("stream", vec!["reader", "writer", "buffer", "io"]);
    m.insert("event", vec!["emit", "listener", "handler", "subscribe"]);
    m.insert("plugin", vec!["extension", "addon", "module", "hook"]);
    m.insert("handler", vec!["callback", "listener", "processor"]);
    m.insert("factory", vec!["builder", "constructor", "create"]);
    m.insert("singleton", vec!["global", "static", "instance"]);

    m
});

/// Expand a query with code-specific synonyms.
///
/// Returns a list of synonym terms to add (NOT the original query terms).
/// The caller is responsible for deduplication and combining with the original.
pub fn expand_with_synonyms(query: &str) -> Vec<String> {
    let mut expansions = Vec::new();
    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower.split_whitespace().collect();

    for word in &query_words {
        // Strip common suffixes for matching: "caching" -> "cache"
        let base = strip_suffix(word);

        if let Some(synonyms) = SYNONYM_MAP.get(base.as_str()) {
            for syn in synonyms.iter().take(MAX_SYNONYMS_PER_TERM) {
                // Don't add synonyms that are already in the query
                if !query_words.contains(syn) && !expansions.contains(&syn.to_string()) {
                    expansions.push(syn.to_string());
                }
            }
        }

        if expansions.len() >= MAX_TOTAL_SYNONYMS {
            break;
        }
    }

    expansions.truncate(MAX_TOTAL_SYNONYMS);
    expansions
}

/// Strip common English suffixes to find the base form for synonym lookup.
fn strip_suffix(word: &str) -> String {
    let suffixes = [
        "ing", "tion", "ment", "ness", "able", "ible", "ize", "ise", "ed", "er", "es", "ly", "ful",
        "less", "ous",
    ];

    for suffix in &suffixes {
        if word.len() > suffix.len() + 2 {
            if let Some(base) = word.strip_suffix(suffix) {
                // Check if the stripped base is in our map
                if SYNONYM_MAP.contains_key(base) {
                    return base.to_string();
                }
                // Try with trailing 'e' added back (e.g., "caching" -> "cach" -> "cache")
                let with_e = format!("{base}e");
                if SYNONYM_MAP.contains_key(with_e.as_str()) {
                    return with_e;
                }
            }
        }
    }

    word.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synonym_expansion_cache() {
        let syns = expand_with_synonyms("cache");
        assert!(
            syns.contains(&"lru".to_string()),
            "cache should expand to lru: {syns:?}"
        );
        assert!(
            syns.contains(&"memoize".to_string()),
            "cache should expand to memoize: {syns:?}"
        );
    }

    #[test]
    fn test_synonym_expansion_auth() {
        let syns = expand_with_synonyms("auth");
        assert!(
            syns.contains(&"authenticate".to_string()),
            "auth should expand to authenticate: {syns:?}"
        );
        assert!(
            syns.contains(&"authorization".to_string()) || syns.contains(&"login".to_string()),
            "auth should expand to authorization or login: {syns:?}"
        );
    }

    #[test]
    fn test_synonym_expansion_database() {
        let syns = expand_with_synonyms("database connection");
        assert!(
            syns.contains(&"db".to_string()) || syns.contains(&"sql".to_string()),
            "database should expand to db or sql: {syns:?}"
        );
    }

    #[test]
    fn test_no_self_expansion() {
        let syns = expand_with_synonyms("cache lru");
        assert!(
            !syns.contains(&"cache".to_string()),
            "should not include query terms in expansion"
        );
        assert!(
            !syns.contains(&"lru".to_string()),
            "should not include query terms in expansion"
        );
    }

    #[test]
    fn test_no_duplicates() {
        let syns = expand_with_synonyms("auth login");
        let unique: std::collections::HashSet<_> = syns.iter().collect();
        assert_eq!(
            syns.len(),
            unique.len(),
            "should have no duplicates: {syns:?}"
        );
    }

    #[test]
    fn test_max_synonyms_cap() {
        let syns = expand_with_synonyms("cache queue stack tree graph list map set");
        assert!(
            syns.len() <= MAX_TOTAL_SYNONYMS,
            "should not exceed max: {} > {}",
            syns.len(),
            MAX_TOTAL_SYNONYMS
        );
    }

    #[test]
    fn test_unknown_word_no_expansion() {
        let syns = expand_with_synonyms("xyzzy");
        assert!(syns.is_empty(), "unknown word should have no synonyms");
    }

    #[test]
    fn test_suffix_stripping() {
        let syns = expand_with_synonyms("caching");
        assert!(
            syns.contains(&"lru".to_string()) || syns.contains(&"memoize".to_string()),
            "caching should match cache synonyms via suffix stripping: {syns:?}"
        );
    }

    #[test]
    fn test_synonym_map_size() {
        assert!(
            SYNONYM_MAP.len() >= 50,
            "synonym map should have at least 50 entries, got {}",
            SYNONYM_MAP.len()
        );
    }
}
