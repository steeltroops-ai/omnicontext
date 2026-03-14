//! C-ABI Foreign Function Interface for `OmniContext`.
//!
//! Produces `omnicontext.dll` (Windows) / `libomnicontext.so` (Linux) /
//! `libomnicontext.dylib` (macOS) for Python, Node.js, and other language
//! agents to call at native speed without IPC overhead.
//!
//! ## Pattern
//!
//! All functions accept/return C-compatible types:
//! - Strings in: `*const c_char` (caller-owned, null-terminated UTF-8)
//! - Strings out: `*mut c_char` (callee-allocated via `CString`, caller frees with `omni_free`)
//! - Engine: opaque `*mut c_void` pointer to a boxed `EngineWrapper`
//!
//! ## Usage (Python)
//!
//! ```python
//! import ctypes
//! lib = ctypes.CDLL("./omnicontext.dll")
//! engine = lib.omni_engine_new(b".")
//! result = lib.omni_search(engine, b"embedding pipeline", 5)
//! # ... use result ...
//! lib.omni_free(result)
//! lib.omni_engine_free(engine)
//! ```

// FFI crate requires unsafe for C-ABI interop — this is intentional and expected.
#![allow(unsafe_code)]
#![allow(missing_docs, clippy::missing_errors_doc, clippy::missing_panics_doc)]

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::Path;

/// Wrapper around Engine + Tokio runtime for async method dispatch.
struct EngineWrapper {
    engine: omni_core::Engine,
    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,
}

/// Helper: convert a `*const c_char` to a Rust `&str`.
///
/// # Safety
/// Caller must ensure `ptr` is a valid null-terminated UTF-8 string.
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

/// Helper: convert a Rust `String` to a heap-allocated `*mut c_char`.
fn string_to_cstring(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create a new `OmniContext` engine for the given repository path.
///
/// Returns an opaque engine pointer, or null on failure.
///
/// # Safety
/// `repo_path` must be a valid null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn omni_engine_new(repo_path: *const c_char) -> *mut c_void {
    let Some(path_str) = cstr_to_str(repo_path) else {
        return std::ptr::null_mut();
    };

    let Ok(runtime) = tokio::runtime::Runtime::new() else {
        return std::ptr::null_mut();
    };

    let Ok(engine) = omni_core::Engine::new(Path::new(path_str)) else {
        return std::ptr::null_mut();
    };

    let wrapper = Box::new(EngineWrapper { engine, runtime });
    Box::into_raw(wrapper).cast::<c_void>()
}

/// Free an engine created by `omni_engine_new`.
///
/// # Safety
/// `engine` must be a pointer returned by `omni_engine_new` and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn omni_engine_free(engine: *mut c_void) {
    if !engine.is_null() {
        drop(Box::from_raw(engine.cast::<EngineWrapper>()));
    }
}

/// Free a string returned by any `omni_*` function.
///
/// # Safety
/// `ptr` must be a pointer returned by an `omni_*` function and not yet freed.
#[no_mangle]
pub unsafe extern "C" fn omni_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

/// Search the codebase. Returns a JSON array of results, or null on error.
///
/// # Safety
/// - `engine` must be a valid engine pointer from `omni_engine_new`.
/// - `query` must be a valid null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn omni_search(
    engine: *const c_void,
    query: *const c_char,
    limit: u32,
) -> *mut c_char {
    let Some(wrapper) = engine.cast::<EngineWrapper>().as_ref() else {
        return std::ptr::null_mut();
    };
    let Some(query_str) = cstr_to_str(query) else {
        return std::ptr::null_mut();
    };

    match wrapper.engine.search(query_str, limit as usize) {
        Ok(results) => {
            let json_results: Vec<serde_json::Value> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "file": r.file_path.display().to_string(),
                        "score": r.score,
                        "symbol": r.chunk.symbol_path,
                        "kind": r.chunk.kind.as_str(),
                        "line_start": r.chunk.line_start,
                        "line_end": r.chunk.line_end,
                        "content": r.chunk.content,
                    })
                })
                .collect();
            let json = serde_json::to_string(&json_results).unwrap_or_default();
            string_to_cstring(json)
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get engine status as JSON. Returns null on error.
///
/// # Safety
/// `engine` must be a valid engine pointer from `omni_engine_new`.
#[no_mangle]
pub unsafe extern "C" fn omni_status(engine: *const c_void) -> *mut c_char {
    let Some(wrapper) = engine.cast::<EngineWrapper>().as_ref() else {
        return std::ptr::null_mut();
    };

    match wrapper.engine.status() {
        Ok(status) => {
            let json = serde_json::to_string(&status).unwrap_or_default();
            string_to_cstring(json)
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Assemble a token-budget-aware context window as JSON. Returns null on error.
///
/// # Safety
/// - `engine` must be a valid engine pointer.
/// - `query` must be a valid null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn omni_context_window(
    engine: *const c_void,
    query: *const c_char,
    limit: u32,
    token_budget: u32,
) -> *mut c_char {
    let Some(wrapper) = engine.cast::<EngineWrapper>().as_ref() else {
        return std::ptr::null_mut();
    };
    let Some(query_str) = cstr_to_str(query) else {
        return std::ptr::null_mut();
    };

    let budget = if token_budget == 0 {
        None
    } else {
        Some(token_budget)
    };

    match wrapper
        .engine
        .search_context_window(query_str, limit as usize, budget)
    {
        Ok(ctx) => {
            let json = serde_json::json!({
                "total_tokens": ctx.total_tokens,
                "token_budget": ctx.token_budget,
                "entries_count": ctx.entries.len(),
                "rendered": ctx.render(),
            });
            string_to_cstring(json.to_string())
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Audit a plan and return JSON critique. Returns null on error.
///
/// # Safety
/// - `engine` must be a valid engine pointer.
/// - `plan` must be a valid null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn omni_audit_plan(
    engine: *const c_void,
    plan: *const c_char,
) -> *mut c_char {
    let Some(wrapper) = engine.cast::<EngineWrapper>().as_ref() else {
        return std::ptr::null_mut();
    };
    let Some(plan_str) = cstr_to_str(plan) else {
        return std::ptr::null_mut();
    };

    let auditor = omni_core::plan_auditor::PlanAuditor::new(&wrapper.engine);
    match auditor.audit(plan_str, 3) {
        Ok(critique) => {
            let json = serde_json::to_string(&critique).unwrap_or_default();
            string_to_cstring(json)
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Compute blast radius for a symbol. Returns JSON array, or null on error.
///
/// Handles all of the following gracefully (returns null, no panic):
/// - null `engine` pointer
/// - null `symbol` pointer
/// - `symbol` containing invalid UTF-8 bytes
/// - symbol not found in index
/// - any internal engine error
///
/// # Safety
/// - `engine` must be a valid engine pointer returned by `omni_engine_new`, or null.
/// - `symbol` must be a valid null-terminated C string, or null.
#[no_mangle]
pub unsafe extern "C" fn omni_blast_radius(
    engine: *const c_void,
    symbol: *const c_char,
    max_depth: u32,
) -> *mut c_char {
    // Guard: null engine
    let Some(wrapper) = engine.cast::<EngineWrapper>().as_ref() else {
        return std::ptr::null_mut();
    };

    // Guard: null symbol pointer
    if symbol.is_null() {
        return std::ptr::null_mut();
    }

    // Guard: invalid UTF-8 in symbol string
    let Ok(sym_str) = CStr::from_ptr(symbol).to_str() else {
        return std::ptr::null_mut();
    };

    // Guard: empty symbol
    if sym_str.trim().is_empty() {
        return std::ptr::null_mut();
    }

    let index = wrapper.engine.metadata_index();
    let graph = wrapper.engine.dep_graph();

    // Look up symbol — try FQN first, fall back to name search
    let sym = match index.get_symbol_by_fqn(sym_str) {
        Ok(Some(s)) => s,
        _ => match index.search_symbols_by_name(sym_str, 1) {
            Ok(syms) => match syms.into_iter().next() {
                Some(s) => s,
                None => return std::ptr::null_mut(),
            },
            Err(_) => return std::ptr::null_mut(),
        },
    };

    match graph.blast_radius(sym.id, max_depth as usize) {
        Ok(radius) => {
            let results: Vec<serde_json::Value> = radius
                .iter()
                .filter_map(|(id, dist)| {
                    index.get_symbol_by_id(*id).ok().flatten().map(|s| {
                        serde_json::json!({
                            "symbol": s.fqn,
                            "distance": dist,
                            "kind": s.kind.as_str(),
                        })
                    })
                })
                .collect();
            let json = serde_json::to_string(&results).unwrap_or_default();
            string_to_cstring(json)
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Verify that the `OmniContext` engine is healthy for the given repo path.
///
/// Specifically, this function:
/// 1. Attempts to open (and immediately release) the `SQLite` database to
///    confirm no other process holds an exclusive lock.
/// 2. Returns `1` (healthy) or `0` (unhealthy / locked).
///
/// Call this before `omni_engine_new` when you need to guarantee the database
/// is available — for example, from an installer or process-guard logic that
/// wants to detect zombie `omnicontext-mcp` processes holding the DB lock.
///
/// # Safety
/// `repo_path` must be a valid null-terminated UTF-8 C string, or null.
/// Returns `0` (unhealthy) on null input.
#[no_mangle]
pub unsafe extern "C" fn omni_ensure_health(repo_path: *const c_char) -> i32 {
    // Guard: null path
    if repo_path.is_null() {
        return 0;
    }

    // Guard: invalid UTF-8
    let path_str = match CStr::from_ptr(repo_path).to_str() {
        Ok(s) if !s.trim().is_empty() => s,
        _ => return 0,
    };

    let path = std::path::Path::new(path_str);

    // Derive the data directory the same way omni-core does:
    // Uses Config::defaults to get the same hash-derived path.
    let data_dir = omni_core::Config::defaults(path).data_dir();
    let db_path = data_dir.join("omnicontext.db");

    // If DB doesn't exist yet there's nothing to lock — healthy.
    if !db_path.exists() {
        return 1;
    }

    // Attempt to open with WAL and an immediate EXCLUSIVE lock probe.
    // rusqlite is bundled — always available.
    match rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(conn) => {
            // Try a BEGIN IMMEDIATE to test for exclusive lock contention.
            let locked = conn.execute_batch("BEGIN IMMEDIATE; ROLLBACK;").is_err();
            i32::from(!locked)
        }
        // Can't open → locked or permissions issue
        Err(_) => 0,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    //! Unit tests for the omni-ffi C ABI layer.
    //!
    //! These tests exercise the public FFI surface directly — same functions a
    //! Python / Node.js caller would invoke — so they cover null-safety, memory
    //! management, and round-trip JSON correctness.

    use super::*;
    use std::ffi::CString;

    // ------------------------------------------------------------------ helpers

    /// Build a `CString` from a literal; panics on interior nul (never in tests).
    fn cs(s: &str) -> CString {
        CString::new(s).expect("test string must not contain interior nul bytes")
    }

    /// Read a callee-allocated `*mut c_char` back into a Rust `String` without
    /// consuming the pointer (so the caller can still pass it to `omni_free`).
    ///
    /// # Safety
    /// `ptr` must be a valid, non-null pointer returned by an `omni_*` function.
    unsafe fn read_cstr(ptr: *mut c_char) -> String {
        CStr::from_ptr(ptr)
            .to_str()
            .expect("FFI returned invalid UTF-8")
            .to_owned()
    }

    /// Create a temporary directory containing a single Rust source file, then
    /// construct an engine pointing at it.  Returns `(TempDir, engine_ptr)`.
    ///
    /// The `TempDir` **must** be kept alive for the duration of the test so that
    /// the underlying path remains valid.  Drop it (and free the engine) after
    /// calling `omni_engine_free`.
    fn make_engine_with_tempdir() -> (tempfile::TempDir, *mut c_void) {
        let dir = tempfile::tempdir().expect("failed to create temp dir");

        // Write a tiny Rust file so the indexer has something to ingest.
        let src = dir.path().join("hello.rs");
        std::fs::write(
            &src,
            b"/// Says hello.\npub fn hello() -> &'static str { \"hello\" }\n",
        )
        .expect("failed to write test source file");

        let path_cstr = cs(dir.path().to_str().expect("temp dir path is not UTF-8"));
        // SAFETY: path_cstr is valid and lives until after this call.
        let engine = unsafe { omni_engine_new(path_cstr.as_ptr()) };

        (dir, engine)
    }

    // -------------------------------------------------------- null-pointer safety

    /// Passing null to `omni_search` must return null without panicking.
    #[test]
    fn test_null_pointer_handling_search_null_engine() {
        let query = cs("anything");
        // SAFETY: intentionally passing null engine — the function must guard.
        let result = unsafe { omni_search(std::ptr::null(), query.as_ptr(), 5) };
        assert!(
            result.is_null(),
            "omni_search(null engine) should return null"
        );
    }

    /// Passing a null query to `omni_search` (valid engine) must return null.
    #[test]
    #[ignore = "requires ONNX model download (~550 MB)"]
    fn test_null_pointer_handling_search_null_query() {
        let (_dir, engine) = make_engine_with_tempdir();
        // SAFETY: engine is valid; null query must be handled gracefully.
        let result = unsafe { omni_search(engine.cast_const(), std::ptr::null(), 5) };
        assert!(
            result.is_null(),
            "omni_search(null query) should return null"
        );
        // SAFETY: engine was created by omni_engine_new and has not been freed.
        unsafe { omni_engine_free(engine) };
    }

    /// Passing null to `omni_status` must return null without panicking.
    #[test]
    fn test_null_pointer_handling_status() {
        // SAFETY: intentionally passing null — the function must guard.
        let result = unsafe { omni_status(std::ptr::null()) };
        assert!(result.is_null(), "omni_status(null) should return null");
    }

    /// Passing null to `omni_context_window` must return null without panicking.
    #[test]
    fn test_null_pointer_handling_context_window() {
        let query = cs("embedding");
        // SAFETY: intentionally passing null engine — the function must guard.
        let result = unsafe { omni_context_window(std::ptr::null(), query.as_ptr(), 5, 2048) };
        assert!(
            result.is_null(),
            "omni_context_window(null engine) should return null"
        );
    }

    // ---------------------------------------------------------- omni_free(null)

    /// Calling `omni_free` with a null pointer must be a safe no-op.
    #[test]
    fn test_omni_free_null() {
        // SAFETY: the function contract explicitly allows null — must not panic.
        unsafe { omni_free(std::ptr::null_mut()) };
        // If we reach here the test passes — no panic, no UB.
    }

    // --------------------------------------------------------- engine lifecycle

    /// Create an engine pointing at a real (temporary) directory, verify the
    /// returned pointer is non-null, and then cleanly free it.
    #[test]
    #[ignore = "requires ONNX model download (~550 MB)"]
    fn test_engine_lifecycle() {
        let (_dir, engine) = make_engine_with_tempdir();
        assert!(
            !engine.is_null(),
            "omni_engine_new should return a non-null pointer for a valid path"
        );
        // SAFETY: engine is non-null and was returned by omni_engine_new.
        unsafe { omni_engine_free(engine) };
        // Double-free must NOT happen: omni_engine_free already consumed the box.
        // We simply verify no panic/abort occurred above.
    }

    /// `omni_engine_new` with a null path must return null without panicking.
    #[test]
    fn test_engine_new_null_path() {
        // SAFETY: intentionally passing null — must be handled gracefully.
        let engine = unsafe { omni_engine_new(std::ptr::null()) };
        assert!(engine.is_null(), "omni_engine_new(null) should return null");
    }

    // ------------------------------------------------- status returns valid JSON

    /// Create an engine, call `omni_status`, and verify the result is valid JSON
    /// that contains the mandatory top-level fields defined by `EngineStatus`.
    #[test]
    #[ignore = "requires ONNX model download (~550 MB)"]
    fn test_status_returns_json() {
        let (_dir, engine) = make_engine_with_tempdir();
        assert!(!engine.is_null(), "engine creation failed");

        // SAFETY: engine is valid and non-null.
        let status_ptr = unsafe { omni_status(engine.cast_const()) };
        assert!(
            !status_ptr.is_null(),
            "omni_status returned null for a valid engine"
        );

        // SAFETY: status_ptr is non-null and was returned by omni_status.
        let json_str = unsafe { read_cstr(status_ptr) };

        // Must be parseable JSON.
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("omni_status did not return valid JSON");

        // EngineStatus has these mandatory fields (see pipeline::EngineStatus).
        let obj = parsed.as_object().expect("status JSON must be an object");
        assert!(
            obj.contains_key("repo_path"),
            "status JSON missing 'repo_path'"
        );
        assert!(
            obj.contains_key("files_indexed"),
            "status JSON missing 'files_indexed'"
        );
        assert!(
            obj.contains_key("chunks_indexed"),
            "status JSON missing 'chunks_indexed'"
        );

        // Free the callee-allocated string.
        // SAFETY: status_ptr was returned by omni_status and has not been freed.
        unsafe { omni_free(status_ptr) };
        // SAFETY: engine was returned by omni_engine_new and has not been freed.
        unsafe { omni_engine_free(engine) };
    }

    // ------------------------------------------------- search returns valid JSON

    /// Create an engine, run a search query, and verify the result is a valid
    /// JSON array (possibly empty if the tiny test file isn't semantically
    /// similar enough, but the structure must be correct).
    #[test]
    #[ignore = "requires ONNX model download (~550 MB)"]
    fn test_search_returns_json() {
        let (_dir, engine) = make_engine_with_tempdir();
        assert!(!engine.is_null(), "engine creation failed");

        let query = cs("hello function");

        // SAFETY: engine is valid; query is a valid null-terminated UTF-8 string.
        let result_ptr = unsafe { omni_search(engine.cast_const(), query.as_ptr(), 5) };
        assert!(
            !result_ptr.is_null(),
            "omni_search returned null for a valid engine and query"
        );

        // SAFETY: result_ptr is non-null and was returned by omni_search.
        let json_str = unsafe { read_cstr(result_ptr) };

        // Must parse as JSON.
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("omni_search did not return valid JSON");

        // Must be a JSON array.
        let arr = parsed
            .as_array()
            .expect("omni_search result must be a JSON array");

        // If there are results, each entry must have the expected keys.
        for entry in arr {
            let obj = entry
                .as_object()
                .expect("search result entry must be an object");
            assert!(obj.contains_key("file"), "search result missing 'file'");
            assert!(obj.contains_key("score"), "search result missing 'score'");
            assert!(obj.contains_key("kind"), "search result missing 'kind'");
        }

        // Free the callee-allocated string.
        // SAFETY: result_ptr was returned by omni_search and has not been freed.
        unsafe { omni_free(result_ptr) };
        // SAFETY: engine was returned by omni_engine_new and has not been freed.
        unsafe { omni_engine_free(engine) };
    }
}
