//! Git commit lineage engine.
//!
//! Indexes git history to provide commit-level context: which commits
//! touched which files, authorship patterns, and change summaries.
#![allow(clippy::doc_markdown, clippy::missing_errors_doc)]

use std::path::Path;

use crate::error::{OmniError, OmniResult};
use crate::index::MetadataIndex;

/// A parsed commit record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitInfo {
    /// Git commit hash (full SHA).
    pub hash: String,
    /// Commit message (first line).
    pub message: String,
    /// Author name.
    pub author: String,
    /// Commit timestamp (ISO 8601).
    pub timestamp: String,
    /// Optional AI-generated summary of changes.
    pub summary: Option<String>,
    /// Files changed in this commit.
    pub files_changed: Vec<String>,
}

/// Commit lineage engine that indexes git history.
pub struct CommitEngine {
    /// Maximum number of commits to index.
    max_commits: usize,
}

impl CommitEngine {
    /// Create a new commit engine.
    #[must_use]
    pub fn new(max_commits: usize) -> Self {
        Self { max_commits }
    }

    /// Index recent git history from the repository.
    pub fn index_history(&self, repo_path: &Path, index: &MetadataIndex) -> OmniResult<usize> {
        let output = std::process::Command::new("git")
            .args([
                "log",
                "--format=%H%n%s%n%an%n%aI",
                "--name-only",
                &format!("-{}", self.max_commits),
            ])
            .current_dir(repo_path)
            .output()
            .map_err(|e| OmniError::Internal(format!("git log failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OmniError::Internal(format!("git log error: {stderr}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commits = Self::parse_git_log(&stdout);

        let mut stored = 0;
        for commit in &commits {
            if let Err(e) = Self::store_commit(index, commit) {
                tracing::warn!(hash = %commit.hash, error = %e, "failed to store commit");
            } else {
                stored += 1;
            }
        }

        tracing::info!(
            commits = stored,
            total = commits.len(),
            "indexed git history"
        );

        Ok(stored)
    }

    /// Parse `git log` output into CommitInfo records.
    fn parse_git_log(output: &str) -> Vec<CommitInfo> {
        let mut commits = Vec::new();
        let mut lines = output.lines().peekable();

        while lines.peek().is_some() {
            // Each commit block: hash, message, author, timestamp, blank, files...
            let hash = match lines.next() {
                Some(h) if !h.is_empty() => h.to_string(),
                _ => break,
            };
            let message = lines.next().unwrap_or("").to_string();
            let author = lines.next().unwrap_or("").to_string();
            let timestamp = lines.next().unwrap_or("").to_string();

            // Skip blank line after timestamp
            if let Some(line) = lines.peek() {
                if line.is_empty() {
                    lines.next();
                }
            }

            // Collect files until next blank line or EOF
            let mut files = Vec::new();
            while let Some(line) = lines.peek() {
                if line.is_empty() {
                    lines.next();
                    break;
                }
                files.push((*line).to_string());
                lines.next();
            }

            commits.push(CommitInfo {
                hash,
                message,
                author,
                timestamp,
                summary: None,
                files_changed: files,
            });
        }

        commits
    }

    /// Store a commit in the SQLite index.
    fn store_commit(index: &MetadataIndex, commit: &CommitInfo) -> OmniResult<()> {
        let conn = index.connection();
        conn.execute(
            "INSERT OR REPLACE INTO commits (hash, message, author, timestamp, summary, files_changed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                commit.hash,
                commit.message,
                commit.author,
                commit.timestamp,
                commit.summary,
                serde_json::to_string(&commit.files_changed).unwrap_or_default(),
            ],
        )?;
        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    /// Get recent commits that touched a specific file.
    pub fn commits_for_file(
        index: &MetadataIndex,
        file_path: &str,
        limit: usize,
    ) -> OmniResult<Vec<CommitInfo>> {
        let conn = index.connection();
        let mut stmt = conn.prepare(
            "SELECT hash, message, author, timestamp, summary, files_changed
             FROM commits
             WHERE files_changed LIKE ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let pattern = format!("%\"{file_path}\"%");
        let commits = stmt
            .query_map(rusqlite::params![pattern, limit], |row| {
                let files_json: String = row.get(5)?;
                let files: Vec<String> = serde_json::from_str(&files_json).unwrap_or_default();
                Ok(CommitInfo {
                    hash: row.get(0)?,
                    message: row.get(1)?,
                    author: row.get(2)?,
                    timestamp: row.get(3)?,
                    summary: row.get(4)?,
                    files_changed: files,
                })
            })?
            .filter_map(std::result::Result::ok)
            .collect();

        Ok(commits)
    }

    #[allow(clippy::missing_errors_doc)]
    /// Get the most active authors for a file.
    pub fn top_authors(
        index: &MetadataIndex,
        file_path: &str,
        limit: usize,
    ) -> OmniResult<Vec<(String, usize)>> {
        let commits = Self::commits_for_file(index, file_path, 100)?;
        let mut author_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for commit in &commits {
            *author_counts.entry(commit.author.clone()).or_default() += 1;
        }

        let mut sorted: Vec<(String, usize)> = author_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(limit);

        Ok(sorted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_log() {
        let _engine = CommitEngine::new(100);
        let log = "abc123\nfeat: add login\nJohn Doe\n2024-01-15T10:30:00+00:00\n\nsrc/auth.rs\nsrc/main.rs\n\ndef456\nfix: typo\nJane Smith\n2024-01-14T09:00:00+00:00\n\nREADME.md\n";

        let commits = CommitEngine::parse_git_log(log);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc123");
        assert_eq!(commits[0].message, "feat: add login");
        assert_eq!(commits[0].author, "John Doe");
        assert_eq!(commits[0].files_changed, vec!["src/auth.rs", "src/main.rs"]);
        assert_eq!(commits[1].hash, "def456");
        assert_eq!(commits[1].files_changed, vec!["README.md"]);
    }

    #[test]
    fn test_parse_empty_log() {
        let commits = CommitEngine::parse_git_log("");
        assert!(commits.is_empty());
    }
}
