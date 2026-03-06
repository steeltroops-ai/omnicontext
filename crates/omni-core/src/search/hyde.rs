//! Template-based Hypothetical Document Embeddings (HyDE).
//!
//! For natural language queries, HyDE generates a hypothetical code snippet
//! that the user might be looking for. The embedding of this hypothetical
//! snippet is often closer in vector space to relevant code than the
//! embedding of the original NL query.
//!
//! ## Strategy
//!
//! We use template-based generation (zero LLM cost) rather than LLM-generated
//! hypothetical documents. Templates cover ~80% of query intents. When no
//! template matches, we fall back to the original query.
//!
//! ## References
//!
//! Gao et al., "Precise Zero-Shot Dense Retrieval without Relevance Labels" (2022)
//! https://arxiv.org/abs/2212.10496

use super::intent::QueryIntent;

/// Generate a hypothetical code snippet for the given query and intent.
///
/// Returns `None` if no template matches, meaning the original query
/// should be used for embedding instead.
pub fn generate_hypothetical_document(query: &str, intent: QueryIntent) -> Option<String> {
    let entity = extract_primary_entity(query);
    if entity.is_empty() {
        return None;
    }

    let snippet = match intent {
        QueryIntent::Explain => {
            format!(
                "/// {entity} - core implementation\n\
                 pub fn {sanitized}(input: &str) -> Result<Output, Error> {{\n\
                 \x20   // Main logic for {entity}\n\
                 \x20   let result = process(input)?;\n\
                 \x20   Ok(result)\n\
                 }}",
                sanitized = sanitize_identifier(&entity),
            )
        }
        QueryIntent::Debug => {
            format!(
                "fn {sanitized}() -> Result<(), Error> {{\n\
                 \x20   // Error handling for {entity}\n\
                 \x20   match operation() {{\n\
                 \x20       Ok(val) => Ok(val),\n\
                 \x20       Err(e) => {{\n\
                 \x20           tracing::error!(error = %e, \"{entity} failed\");\n\
                 \x20           Err(e)\n\
                 \x20       }}\n\
                 \x20   }}\n\
                 }}",
                sanitized = sanitize_identifier(&entity),
            )
        }
        QueryIntent::Edit | QueryIntent::Refactor => {
            format!(
                "impl {capitalized} {{\n\
                 \x20   pub fn {sanitized}(&mut self) {{\n\
                 \x20       // Implementation of {entity}\n\
                 \x20       self.update();\n\
                 \x20   }}\n\
                 }}",
                capitalized = capitalize_first(&entity),
                sanitized = sanitize_identifier(&entity),
            )
        }
        QueryIntent::Generate => {
            format!(
                "/// Create a new {entity}.\n\
                 pub struct {capitalized} {{\n\
                 \x20   // Fields for {entity}\n\
                 }}\n\n\
                 impl {capitalized} {{\n\
                 \x20   pub fn new() -> Self {{\n\
                 \x20       Self {{ }}\n\
                 \x20   }}\n\
                 }}",
                capitalized = capitalize_first(&entity),
            )
        }
        QueryIntent::DataFlow => {
            format!(
                "fn process_{sanitized}(input: Input) -> Output {{\n\
                 \x20   // Data flow: parse -> transform -> emit\n\
                 \x20   let parsed = parse(input)?;\n\
                 \x20   let transformed = transform_{sanitized}(parsed)?;\n\
                 \x20   emit(transformed)\n\
                 }}",
                sanitized = sanitize_identifier(&entity),
            )
        }
        QueryIntent::Dependency => {
            format!(
                "use crate::{sanitized};\n\
                 use crate::{sanitized}::*;\n\n\
                 // Dependencies of {entity}\n\
                 fn {sanitized}_deps() -> Vec<Dependency> {{\n\
                 \x20   vec![]\n\
                 }}",
                sanitized = sanitize_identifier(&entity),
            )
        }
        QueryIntent::TestCoverage => {
            format!(
                "#[cfg(test)]\n\
                 mod tests {{\n\
                 \x20   use super::*;\n\n\
                 \x20   #[test]\n\
                 \x20   fn test_{sanitized}() {{\n\
                 \x20       // Test coverage for {entity}\n\
                 \x20       let result = {sanitized}();\n\
                 \x20       assert!(result.is_ok());\n\
                 \x20   }}\n\
                 }}",
                sanitized = sanitize_identifier(&entity),
            )
        }
        QueryIntent::Unknown => {
            // For unknown intent, generate a generic function stub
            format!(
                "fn {sanitized}() {{\n\
                 \x20   // {entity}\n\
                 }}",
                sanitized = sanitize_identifier(&entity),
            )
        }
    };

    Some(snippet)
}

/// Extract the primary entity/subject from a query string.
///
/// Strips common question words, prepositions, and articles to isolate
/// the main code concept the user is asking about.
fn extract_primary_entity(query: &str) -> String {
    let stop_words = [
        "how", "does", "the", "a", "an", "is", "are", "was", "were", "do", "what", "why", "when",
        "where", "which", "who", "can", "could", "should", "would", "will", "to", "from", "in",
        "of", "for", "with", "at", "by", "on", "about", "it", "this", "that", "these", "those",
        "work", "works", "working", "implement", "implemented", "implementation", "fix", "find",
        "show", "me", "please", "i", "want", "need", "help",
    ];

    let words: Vec<&str> = query
        .split_whitespace()
        .filter(|w| {
            let lower = w.to_lowercase();
            !stop_words.contains(&lower.as_str()) && lower.len() > 1
        })
        .collect();

    if words.is_empty() {
        return String::new();
    }

    // Take up to 3 significant words as the entity
    words.iter().take(3).copied().collect::<Vec<_>>().join("_")
}

/// Sanitize a string into a valid Rust/Python identifier.
fn sanitize_identifier(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

/// Capitalize the first character of a string.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => {
            let upper = c.to_uppercase().to_string();
            let rest: String = chars.collect();
            format!("{upper}{rest}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyde_explain_intent() {
        let result =
            generate_hypothetical_document("how does authentication work", QueryIntent::Explain);
        assert!(result.is_some());
        let doc = result.unwrap();
        assert!(
            doc.contains("authentication"),
            "should contain entity: {doc}"
        );
        assert!(doc.contains("fn "), "should contain function signature");
    }

    #[test]
    fn test_hyde_debug_intent() {
        let result =
            generate_hypothetical_document("fix the database connection error", QueryIntent::Debug);
        assert!(result.is_some());
        let doc = result.unwrap();
        assert!(doc.contains("Error"), "should contain error handling");
        assert!(doc.contains("database"), "should contain entity");
    }

    #[test]
    fn test_hyde_generate_intent() {
        let result = generate_hypothetical_document("create a cache layer", QueryIntent::Generate);
        assert!(result.is_some());
        let doc = result.unwrap();
        assert!(doc.contains("struct"), "should contain struct definition");
        assert!(doc.contains("new()"), "should contain constructor");
    }

    #[test]
    fn test_hyde_unknown_fallback() {
        let result = generate_hypothetical_document("cache", QueryIntent::Unknown);
        assert!(result.is_some());
        let doc = result.unwrap();
        assert!(doc.contains("cache"), "should contain entity");
    }

    #[test]
    fn test_hyde_empty_query() {
        let result = generate_hypothetical_document("", QueryIntent::Explain);
        assert!(result.is_none(), "empty query should return None");
    }

    #[test]
    fn test_hyde_all_stop_words() {
        let result = generate_hypothetical_document("how does the it work", QueryIntent::Explain);
        assert!(result.is_none(), "all-stop-word query should return None");
    }

    #[test]
    fn test_extract_entity() {
        assert_eq!(
            extract_primary_entity("how does authentication work"),
            "authentication"
        );
        assert_eq!(
            extract_primary_entity("fix the database connection error"),
            "database_connection_error"
        );
        assert_eq!(extract_primary_entity("create a cache"), "create_cache");
    }

    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(sanitize_identifier("hello-world"), "hello_world");
        assert_eq!(sanitize_identifier("CamelCase"), "camelcase");
        assert_eq!(sanitize_identifier("with spaces"), "with_spaces");
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("a"), "A");
    }
}
