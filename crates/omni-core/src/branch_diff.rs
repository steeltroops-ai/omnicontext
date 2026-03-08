//! Per-branch diff indexing for branch-aware context.
//!
//! Tracks the current git branch and uncommitted/unpushed changes,
//! enabling the daemon to serve branch-specific context overlays
//! on top of the main index. This achieves the SOTA per-developer
//! indexing capability described in Section 7.B of the architecture.
//!
//! Design principles:
//! - Non-destructive: operates as an overlay on the main index
//! - Lightweight: only tracks diff hunks, not full file contents
//! - Self-healing: gracefully degrades if git is unavailable

#![allow(clippy::doc_markdown, clippy::missing_errors_doc)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{OmniError, OmniResult};

/// A changed hunk in a file diff.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiffHunk {
    /// File path relative to repo root.
    pub file_path: String,
    /// Start line of the hunk (1-based).
    pub start_line: usize,
    /// Number of lines in the hunk.
    pub line_count: usize,
    /// The actual changed content.
    pub content: String,
    /// Whether this is an addition or modification.
    pub change_type: ChangeType,
}

/// Type of change in a diff hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChangeType {
    /// New lines added.
    Added,
    /// Existing lines modified.
    Modified,
    /// Lines deleted.
    Deleted,
}

/// Branch-level diff state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BranchDiff {
    /// Current branch name.
    pub branch: String,
    /// Base branch (what we're diffing against, typically main/master).
    pub base_branch: String,
    /// Files with uncommitted changes (working tree).
    pub uncommitted_files: Vec<String>,
    /// Files with unpushed changes (committed but not on remote).
    pub unpushed_files: Vec<String>,
    /// Diff hunks for uncommitted changes.
    pub uncommitted_hunks: Vec<DiffHunk>,
    /// Total lines changed across all hunks.
    pub total_lines_changed: usize,
}

/// Per-branch diff tracker.
///
/// Queries git to determine the current branch state and extracts
/// structured diff data for context-aware search overlays.
pub struct BranchTracker {
    /// Repository root path.
    repo_root: PathBuf,
    /// Cached branch state.
    cached_state: Option<BranchDiff>,
    /// Timestamp of last cache refresh.
    last_refresh: Option<std::time::Instant>,
    /// Cache TTL (how often to re-query git).
    cache_ttl: std::time::Duration,
}

impl BranchTracker {
    /// Create a new branch tracker for the given repository.
    pub fn new(repo_root: &Path) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
            cached_state: None,
            last_refresh: None,
            cache_ttl: std::time::Duration::from_secs(5),
        }
    }

    /// Get the current branch diff state.
    ///
    /// Uses a cached result if within TTL, otherwise re-queries git.
    pub fn get_branch_diff(&mut self) -> OmniResult<&BranchDiff> {
        let now = std::time::Instant::now();
        let needs_refresh = match self.last_refresh {
            Some(last) => now.duration_since(last) > self.cache_ttl,
            None => true,
        };

        if needs_refresh {
            let diff = self.compute_branch_diff()?;
            self.cached_state = Some(diff);
            self.last_refresh = Some(now);
        }

        self.cached_state
            .as_ref()
            .ok_or_else(|| OmniError::Internal("branch diff unavailable".into()))
    }

    /// Get the current branch name.
    pub fn current_branch(&self) -> OmniResult<String> {
        git_current_branch(&self.repo_root)
    }

    /// Get the default base branch (main or master).
    pub fn detect_base_branch(&self) -> OmniResult<String> {
        // Check if 'main' exists, otherwise fall back to 'master'
        let output = std::process::Command::new("git")
            .args(["branch", "--list", "main"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| OmniError::Internal(format!("git branch failed: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().contains("main") {
            Ok("main".to_string())
        } else {
            Ok("master".to_string())
        }
    }

    /// Force a cache refresh on next access.
    pub fn invalidate(&mut self) {
        self.last_refresh = None;
    }

    /// Compute the full branch diff state.
    fn compute_branch_diff(&self) -> OmniResult<BranchDiff> {
        let branch = self.current_branch()?;
        let base_branch = self.detect_base_branch()?;

        // Get uncommitted file list (working tree + staged)
        let uncommitted_files = self.get_uncommitted_files()?;

        // Get unpushed file list (commits ahead of remote)
        let unpushed_files = self.get_unpushed_files(&branch)?;

        // Get diff hunks for uncommitted changes
        let uncommitted_hunks = self.get_uncommitted_hunks()?;

        let total_lines_changed: usize = uncommitted_hunks.iter().map(|h| h.line_count).sum();

        Ok(BranchDiff {
            branch,
            base_branch,
            uncommitted_files,
            unpushed_files,
            uncommitted_hunks,
            total_lines_changed,
        })
    }

    /// Get list of files with uncommitted changes.
    fn get_uncommitted_files(&self) -> OmniResult<Vec<String>> {
        let output = std::process::Command::new("git")
            .args(["diff", "--name-only", "HEAD"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| OmniError::Internal(format!("git diff failed: {e}")))?;

        // Also get staged but uncommitted
        let staged_output = std::process::Command::new("git")
            .args(["diff", "--name-only", "--cached"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| OmniError::Internal(format!("git diff --cached failed: {e}")))?;

        // Also get untracked files
        let untracked_output = std::process::Command::new("git")
            .args(["ls-files", "--others", "--exclude-standard"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| OmniError::Internal(format!("git ls-files failed: {e}")))?;

        let mut files: Vec<String> = Vec::new();
        for out in [&output, &staged_output, &untracked_output] {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let trimmed = line.trim().to_string();
                    if !trimmed.is_empty() && !files.contains(&trimmed) {
                        files.push(trimmed);
                    }
                }
            }
        }

        Ok(files)
    }

    /// Get list of files changed in unpushed commits.
    fn get_unpushed_files(&self, branch: &str) -> OmniResult<Vec<String>> {
        let remote_ref = format!("origin/{branch}");
        let output = std::process::Command::new("git")
            .args(["diff", "--name-only", &remote_ref, "HEAD"])
            .current_dir(&self.repo_root)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                Ok(stdout
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect())
            }
            _ => {
                // No remote tracking branch -- not an error
                Ok(Vec::new())
            }
        }
    }

    /// Parse diff hunks from uncommitted changes.
    fn get_uncommitted_hunks(&self) -> OmniResult<Vec<DiffHunk>> {
        let output = std::process::Command::new("git")
            .args(["diff", "--unified=0", "HEAD"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| OmniError::Internal(format!("git diff failed: {e}")))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_diff_hunks(&stdout))
    }

    /// Get files changed between current branch and base branch.
    pub fn get_branch_changed_files(&self) -> OmniResult<Vec<String>> {
        let base = self.detect_base_branch()?;
        let merge_base_output = std::process::Command::new("git")
            .args(["merge-base", &base, "HEAD"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| OmniError::Internal(format!("git merge-base failed: {e}")))?;

        if !merge_base_output.status.success() {
            return Ok(Vec::new());
        }

        let merge_base = String::from_utf8_lossy(&merge_base_output.stdout)
            .trim()
            .to_string();

        let output = std::process::Command::new("git")
            .args(["diff", "--name-only", &merge_base, "HEAD"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| OmniError::Internal(format!("git diff failed: {e}")))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    /// Get a summary of branch activity suitable for context injection.
    pub fn branch_context_summary(&mut self) -> OmniResult<String> {
        let diff = self.compute_branch_diff()?;
        let mut summary = String::new();

        summary.push_str(&format!("Branch: {}\n", diff.branch));
        summary.push_str(&format!("Base: {}\n", diff.base_branch));

        if !diff.uncommitted_files.is_empty() {
            summary.push_str(&format!(
                "Uncommitted changes: {} files\n",
                diff.uncommitted_files.len()
            ));
            for f in &diff.uncommitted_files {
                summary.push_str(&format!("  - {f}\n"));
            }
        }

        if !diff.unpushed_files.is_empty() {
            summary.push_str(&format!(
                "Unpushed changes: {} files\n",
                diff.unpushed_files.len()
            ));
        }

        summary.push_str(&format!(
            "Total lines changed: {}\n",
            diff.total_lines_changed
        ));

        Ok(summary)
    }
}

/// Get current git branch name.
fn git_current_branch(repo_root: &Path) -> OmniResult<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| OmniError::Internal(format!("git rev-parse failed: {e}")))?;

    if !output.status.success() {
        return Err(OmniError::Internal("not a git repository".into()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Parse a unified diff into structured hunks.
fn parse_diff_hunks(diff_output: &str) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    let mut current_file: Option<String> = None;

    for line in diff_output.lines() {
        // Track current file from diff headers
        if let Some(path) = line.strip_prefix("+++ b/") {
            current_file = Some(path.to_string());
            continue;
        }
        if line.starts_with("--- ") || line.starts_with("+++ ") {
            continue;
        }

        // Parse hunk headers: @@ -old_start,old_count +new_start,new_count @@
        if line.starts_with("@@ ") {
            if let Some(ref file) = current_file {
                if let Some(hunk) = parse_hunk_header(line, file) {
                    hunks.push(hunk);
                }
            }
            continue;
        }

        // Accumulate content for the current hunk
        if let Some(last_hunk) = hunks.last_mut() {
            if line.starts_with('+') && !line.starts_with("+++") {
                let content_line = &line[1..]; // Strip the leading '+'
                if !last_hunk.content.is_empty() {
                    last_hunk.content.push('\n');
                }
                last_hunk.content.push_str(content_line);
                last_hunk.line_count += 1;
            }
        }
    }

    // Filter out empty hunks
    hunks.retain(|h| !h.content.is_empty());
    hunks
}

/// Parse a single @@ hunk header line.
fn parse_hunk_header(line: &str, file: &str) -> Option<DiffHunk> {
    // Format: @@ -old_start[,old_count] +new_start[,new_count] @@ [context]
    let parts: Vec<&str> = line.split("@@").collect();
    if parts.len() < 2 {
        return None;
    }

    let range_part = parts[1].trim();
    let new_range = range_part.split(' ').find(|s| s.starts_with('+'))?;
    let new_range = &new_range[1..]; // Strip '+'

    let (start_str, _count_str) = if new_range.contains(',') {
        let mut parts = new_range.splitn(2, ',');
        (parts.next()?, parts.next().unwrap_or("1"))
    } else {
        (new_range, "1")
    };

    let start_line: usize = start_str.parse().ok()?;

    Some(DiffHunk {
        file_path: file.to_string(),
        start_line,
        line_count: 0, // Will be incremented as content lines are added
        content: String::new(),
        change_type: ChangeType::Modified,
    })
}

/// Build a file-to-hunks mapping for efficient lookup.
pub fn group_hunks_by_file(hunks: &[DiffHunk]) -> HashMap<String, Vec<&DiffHunk>> {
    let mut map: HashMap<String, Vec<&DiffHunk>> = HashMap::new();
    for hunk in hunks {
        map.entry(hunk.file_path.clone()).or_default().push(hunk);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_hunks_basic() {
        let diff = "\
diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -10,0 +11,2 @@
+fn new_function() {
+    println!(\"hello\");
";

        let hunks = parse_diff_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].file_path, "src/main.rs");
        assert_eq!(hunks[0].start_line, 11);
        assert_eq!(hunks[0].line_count, 2);
        assert!(hunks[0].content.contains("new_function"));
    }

    #[test]
    fn test_parse_diff_hunks_multiple_files() {
        let diff = "\
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,0 +2,1 @@
+use crate::b;
diff --git a/src/b.rs b/src/b.rs
--- a/src/b.rs
+++ b/src/b.rs
@@ -5,0 +6,1 @@
+pub fn exported() {}
";

        let hunks = parse_diff_hunks(diff);
        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].file_path, "src/a.rs");
        assert_eq!(hunks[1].file_path, "src/b.rs");
    }

    #[test]
    fn test_parse_empty_diff() {
        let hunks = parse_diff_hunks("");
        assert!(hunks.is_empty());
    }

    #[test]
    fn test_group_hunks_by_file() {
        let hunks = vec![
            DiffHunk {
                file_path: "a.rs".to_string(),
                start_line: 1,
                line_count: 2,
                content: "fn a()".to_string(),
                change_type: ChangeType::Added,
            },
            DiffHunk {
                file_path: "b.rs".to_string(),
                start_line: 5,
                line_count: 1,
                content: "fn b()".to_string(),
                change_type: ChangeType::Modified,
            },
            DiffHunk {
                file_path: "a.rs".to_string(),
                start_line: 10,
                line_count: 3,
                content: "fn a2()".to_string(),
                change_type: ChangeType::Added,
            },
        ];

        let grouped = group_hunks_by_file(&hunks);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped["a.rs"].len(), 2);
        assert_eq!(grouped["b.rs"].len(), 1);
    }

    #[test]
    fn test_parse_hunk_header() {
        let hunk = parse_hunk_header("@@ -10,3 +15,5 @@ fn main() {", "src/main.rs");
        assert!(hunk.is_some());
        let h = hunk.unwrap();
        assert_eq!(h.start_line, 15);
        assert_eq!(h.file_path, "src/main.rs");
    }
}
