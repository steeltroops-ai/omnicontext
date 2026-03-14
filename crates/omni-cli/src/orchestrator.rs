//! Universal IDE Orchestrator for `OmniContext`.
//!
//! Auto-discovers every AI IDE and Agent present on the host system and
//! injects a single, universal `omnicontext` MCP server entry using the
//! `--repo .` flag for dynamic context awareness.
//!
//! ## Design Principles
//!
//! - **Zero-Config Installation**: run `omnicontext setup --all` after install;
//!   every detected IDE immediately has the `omnicontext` tool available.
//! - **Universal entry**: always keyed `"omnicontext"` (never project-hash variants).
//! - **Atomic JSON patching**: existing IDE config is never overwritten wholesale;
//!   only the `omnicontext` key inside `mcpServers` is inserted/updated.
//! - **Legacy purge**: any `omnicontext-<hex>` duplicate entries are removed.
//! - **Fail-proof**: read-only configs are skipped with a warning; malformed JSON
//!   is silently treated as an empty object; missing parent dirs are created.
//! - **Self-repair**: calling `orchestrate()` again is idempotent — it re-injects
//!   the entry if an IDE update overwrote it.

#![allow(clippy::too_many_lines)]

use std::path::{Path, PathBuf};

use anyhow::Result;

// ---------------------------------------------------------------------------
// Public result types
// ---------------------------------------------------------------------------

/// Status of configuring a single IDE.
#[derive(Debug, Clone)]
pub enum IdeStatus {
    /// Successfully written.
    Configured,
    /// No change needed — entry was already current.
    AlreadyCurrent,
    /// IDE config directory exists but the file is read-only / permission denied.
    PermissionDenied(String),
    /// Config directory not found on this machine — IDE not installed.
    NotInstalled,
    /// Any other error (malformed config we couldn't recover, I/O failure, …).
    Error(String),
}

impl IdeStatus {
    /// Returns `true` if the IDE was detected (installed) regardless of outcome.
    #[must_use]
    pub fn is_installed(&self) -> bool {
        !matches!(self, IdeStatus::NotInstalled)
    }

    /// Returns `true` if the final config was written or already up-to-date.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        matches!(self, IdeStatus::Configured | IdeStatus::AlreadyCurrent)
    }
}

/// The result for a single IDE target.
#[derive(Debug)]
pub struct IdeResult {
    pub name: &'static str,
    pub status: IdeStatus,
    /// Legacy duplicate keys that were purged (empty if none).
    pub purged: Vec<String>,
}

/// Overall orchestration summary.
#[derive(Debug)]
pub struct OrchestrationResult {
    pub results: Vec<IdeResult>,
    pub mcp_binary: PathBuf,
}

impl OrchestrationResult {
    /// Number of IDEs detected (installed).
    #[must_use]
    pub fn detected(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status.is_installed())
            .count()
    }

    /// Number of IDEs successfully configured.
    #[must_use]
    pub fn configured(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status.is_ok() && r.status.is_installed())
            .count()
    }

    /// Number of legacy entries purged across all IDEs.
    #[must_use]
    pub fn total_purged(&self) -> usize {
        self.results.iter().map(|r| r.purged.len()).sum()
    }
}

// ---------------------------------------------------------------------------
// IDE target definitions
// ---------------------------------------------------------------------------

struct IdeTarget {
    name: &'static str,
    config_path: PathBuf,
    /// The JSON key that holds the `mcpServers` map, supports "a.b" nesting.
    server_key: &'static str,
}

fn build_ide_targets() -> Vec<IdeTarget> {
    let home = dirs::home_dir().unwrap_or_default();

    #[cfg(windows)]
    let appdata = std::env::var("APPDATA")
        .map_or_else(|_| home.join("AppData").join("Roaming"), PathBuf::from);

    // macOS Application Support dir (used for many macOS apps)
    #[cfg(target_os = "macos")]
    let app_support = home.join("Library").join("Application Support");

    // Linux: ~/.config
    #[cfg(all(not(windows), not(target_os = "macos")))]
    let app_support = home.join(".config");

    vec![
        // ----------------------------------------------------------------
        // Claude Desktop
        // Verified: https://modelcontextprotocol.io/quickstart/user
        // ----------------------------------------------------------------
        #[cfg(windows)]
        IdeTarget {
            name: "Claude Desktop",
            config_path: appdata.join("Claude").join("claude_desktop_config.json"),
            server_key: "mcpServers",
        },
        #[cfg(target_os = "macos")]
        IdeTarget {
            name: "Claude Desktop",
            config_path: app_support
                .join("Claude")
                .join("claude_desktop_config.json"),
            server_key: "mcpServers",
        },
        #[cfg(all(not(windows), not(target_os = "macos")))]
        IdeTarget {
            name: "Claude Desktop",
            config_path: app_support
                .join("Claude")
                .join("claude_desktop_config.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // Claude Code (CLI) — ~/.claude.json (user-scoped MCP servers)
        // Verified: https://code.claude.com/docs/en/mcp
        // Note: ~/.claude/settings.json holds permissions/env, NOT mcpServers.
        //       ~/.claude.json is the correct file for user-scoped MCP.
        // ----------------------------------------------------------------
        IdeTarget {
            name: "Claude Code",
            config_path: home.join(".claude.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // Cursor
        // Global user config (macOS: ~/Library/Application Support/Cursor/User/mcp.json)
        // Verified: Cursor docs + VS Code fork pattern
        // ----------------------------------------------------------------
        #[cfg(windows)]
        IdeTarget {
            name: "Cursor",
            config_path: appdata.join("Cursor").join("User").join("mcp.json"),
            server_key: "mcpServers",
        },
        #[cfg(target_os = "macos")]
        IdeTarget {
            name: "Cursor",
            config_path: app_support.join("Cursor").join("User").join("mcp.json"),
            server_key: "mcpServers",
        },
        #[cfg(all(not(windows), not(target_os = "macos")))]
        IdeTarget {
            name: "Cursor",
            config_path: home.join(".cursor").join("mcp.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // Windsurf (Codeium)
        // Verified: https://docs.windsurf.com/windsurf/cascade/mcp
        // Global only: ~/.codeium/windsurf/mcp_config.json
        // ----------------------------------------------------------------
        IdeTarget {
            name: "Windsurf",
            config_path: home
                .join(".codeium")
                .join("windsurf")
                .join("mcp_config.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // VS Code (GitHub Copilot + native MCP, v1.99+)
        // Verified: https://code.visualstudio.com/docs/copilot/chat/mcp-servers
        // Note: VS Code uses "servers" (NOT "mcpServers") in its mcp.json
        // ----------------------------------------------------------------
        #[cfg(windows)]
        IdeTarget {
            name: "VS Code",
            config_path: appdata.join("Code").join("User").join("mcp.json"),
            server_key: "servers",
        },
        #[cfg(target_os = "macos")]
        IdeTarget {
            name: "VS Code",
            config_path: app_support.join("Code").join("User").join("mcp.json"),
            server_key: "servers",
        },
        #[cfg(all(not(windows), not(target_os = "macos")))]
        IdeTarget {
            name: "VS Code",
            config_path: app_support.join("Code").join("User").join("mcp.json"),
            server_key: "servers",
        },
        // ----------------------------------------------------------------
        // VS Code Insiders (same layout, different app data dir)
        // ----------------------------------------------------------------
        #[cfg(windows)]
        IdeTarget {
            name: "VS Code Insiders",
            config_path: appdata
                .join("Code - Insiders")
                .join("User")
                .join("mcp.json"),
            server_key: "servers",
        },
        #[cfg(target_os = "macos")]
        IdeTarget {
            name: "VS Code Insiders",
            config_path: app_support
                .join("Code - Insiders")
                .join("User")
                .join("mcp.json"),
            server_key: "servers",
        },
        #[cfg(all(not(windows), not(target_os = "macos")))]
        IdeTarget {
            name: "VS Code Insiders",
            config_path: app_support
                .join("Code - Insiders")
                .join("User")
                .join("mcp.json"),
            server_key: "servers",
        },
        // ----------------------------------------------------------------
        // Cline (VS Code extension — saoudrizwan.claude-dev)
        // Verified: VS Code globalStorage pattern
        // ----------------------------------------------------------------
        #[cfg(windows)]
        IdeTarget {
            name: "Cline",
            config_path: appdata
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("saoudrizwan.claude-dev")
                .join("settings")
                .join("cline_mcp_settings.json"),
            server_key: "mcpServers",
        },
        #[cfg(target_os = "macos")]
        IdeTarget {
            name: "Cline",
            config_path: app_support
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("saoudrizwan.claude-dev")
                .join("settings")
                .join("cline_mcp_settings.json"),
            server_key: "mcpServers",
        },
        #[cfg(all(not(windows), not(target_os = "macos")))]
        IdeTarget {
            name: "Cline",
            config_path: app_support
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("saoudrizwan.claude-dev")
                .join("settings")
                .join("cline_mcp_settings.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // RooCode (VS Code extension — rooveterinaryinc.roo-cline)
        // Verified: RooCode source McpHub.ts
        // ----------------------------------------------------------------
        #[cfg(windows)]
        IdeTarget {
            name: "RooCode",
            config_path: appdata
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("rooveterinaryinc.roo-cline")
                .join("settings")
                .join("mcp_settings.json"),
            server_key: "mcpServers",
        },
        #[cfg(target_os = "macos")]
        IdeTarget {
            name: "RooCode",
            config_path: app_support
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("rooveterinaryinc.roo-cline")
                .join("settings")
                .join("mcp_settings.json"),
            server_key: "mcpServers",
        },
        #[cfg(all(not(windows), not(target_os = "macos")))]
        IdeTarget {
            name: "RooCode",
            config_path: app_support
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("rooveterinaryinc.roo-cline")
                .join("settings")
                .join("mcp_settings.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // Continue.dev (VS Code / JetBrains extension)
        // Verified: Continue source paths.ts
        // config.json (legacy) — config.yaml uses array syntax, not suitable
        // for our JSON patch approach, so we target the JSON file.
        // ----------------------------------------------------------------
        IdeTarget {
            name: "Continue.dev",
            config_path: home.join(".continue").join("config.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // Zed Editor
        // Uses "context_servers" key, not "mcpServers"
        // ----------------------------------------------------------------
        IdeTarget {
            name: "Zed",
            config_path: home.join(".config").join("zed").join("settings.json"),
            server_key: "context_servers",
        },
        // ----------------------------------------------------------------
        // Kiro (AWS/Amazon IDE)
        // Uses nested "powers.mcpServers" key
        // ----------------------------------------------------------------
        IdeTarget {
            name: "Kiro",
            config_path: home.join(".kiro").join("settings").join("mcp.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // PearAI
        // ----------------------------------------------------------------
        #[cfg(windows)]
        IdeTarget {
            name: "PearAI",
            config_path: appdata.join("PearAI").join("User").join("mcp.json"),
            server_key: "mcpServers",
        },
        #[cfg(target_os = "macos")]
        IdeTarget {
            name: "PearAI",
            config_path: app_support.join("PearAI").join("User").join("mcp.json"),
            server_key: "mcpServers",
        },
        #[cfg(all(not(windows), not(target_os = "macos")))]
        IdeTarget {
            name: "PearAI",
            config_path: app_support.join("PearAI").join("User").join("mcp.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // Trae IDE (ByteDance — VS Code fork)
        // ----------------------------------------------------------------
        #[cfg(windows)]
        IdeTarget {
            name: "Trae",
            config_path: appdata
                .join("Trae")
                .join("User")
                .join("globalStorage")
                .join("trae-ide.trae-ai")
                .join("mcp_settings.json"),
            server_key: "mcpServers",
        },
        #[cfg(target_os = "macos")]
        IdeTarget {
            name: "Trae",
            config_path: app_support.join("Trae").join("mcp_config.json"),
            server_key: "mcpServers",
        },
        #[cfg(all(not(windows), not(target_os = "macos")))]
        IdeTarget {
            name: "Trae",
            config_path: app_support.join("Trae").join("mcp_config.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // Antigravity (VS Code fork)
        // Uses %APPDATA%/Antigravity/User/mcp.json with "servers" key
        // (same convention as VS Code native MCP)
        // ----------------------------------------------------------------
        #[cfg(windows)]
        IdeTarget {
            name: "Antigravity",
            config_path: appdata.join("Antigravity").join("User").join("mcp.json"),
            server_key: "servers",
        },
        #[cfg(target_os = "macos")]
        IdeTarget {
            name: "Antigravity",
            config_path: app_support
                .join("Antigravity")
                .join("User")
                .join("mcp.json"),
            server_key: "servers",
        },
        #[cfg(all(not(windows), not(target_os = "macos")))]
        IdeTarget {
            name: "Antigravity",
            config_path: app_support
                .join("Antigravity")
                .join("User")
                .join("mcp.json"),
            server_key: "servers",
        },
        // ----------------------------------------------------------------
        // Gemini CLI (google-gemini/gemini-cli)
        // Verified: https://github.com/google-gemini/gemini-cli
        // ----------------------------------------------------------------
        IdeTarget {
            name: "Gemini CLI",
            config_path: home.join(".gemini").join("settings.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // AWS Amazon Q Developer CLI
        // Uses ~/.aws/amazonq/mcp.json
        // ----------------------------------------------------------------
        IdeTarget {
            name: "Amazon Q CLI",
            config_path: home.join(".aws").join("amazonq").join("mcp.json"),
            server_key: "mcpServers",
        },
        // ----------------------------------------------------------------
        // Augment Code (VS Code extension — augment.vscode-augment)
        // ----------------------------------------------------------------
        #[cfg(windows)]
        IdeTarget {
            name: "Augment Code",
            config_path: appdata
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("augment.vscode-augment")
                .join("mcp_settings.json"),
            server_key: "mcpServers",
        },
        #[cfg(target_os = "macos")]
        IdeTarget {
            name: "Augment Code",
            config_path: app_support
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("augment.vscode-augment")
                .join("mcp_settings.json"),
            server_key: "mcpServers",
        },
        #[cfg(all(not(windows), not(target_os = "macos")))]
        IdeTarget {
            name: "Augment Code",
            config_path: app_support
                .join("Code")
                .join("User")
                .join("globalStorage")
                .join("augment.vscode-augment")
                .join("mcp_settings.json"),
            server_key: "mcpServers",
        },
    ]
}

// ---------------------------------------------------------------------------
// MCP binary discovery
// ---------------------------------------------------------------------------

/// Find the `omnicontext-mcp` binary — searches next to the running exe first.
pub fn find_mcp_binary() -> Result<PathBuf> {
    let current_exe = std::env::current_exe()?;
    let dir = current_exe.parent().unwrap_or_else(|| Path::new("."));

    // Preferred: sibling binary without extension (Unix) or with .exe (Windows)
    let candidates: &[&str] = &[
        #[cfg(windows)]
        "omnicontext-mcp.exe",
        "omnicontext-mcp",
    ];

    for name in candidates {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // Fall back to current exe (CLI delegates to MCP via subprocess anyway)
    Ok(current_exe)
}

// ---------------------------------------------------------------------------
// Core entry builder
// ---------------------------------------------------------------------------

/// Build the universal MCP server entry JSON.
///
/// Command is the **absolute path** to `omnicontext-mcp(.exe)`.
/// Args are `["--repo", "."]` so the server resolves context dynamically.
fn build_universal_entry(mcp_binary: &Path) -> serde_json::Value {
    // On Windows, convert forward slashes to backslashes for JSON safety
    #[cfg(windows)]
    let command_str = mcp_binary.display().to_string().replace('/', "\\");
    #[cfg(not(windows))]
    let command_str = mcp_binary.display().to_string();

    serde_json::json!({
        "command": command_str,
        "args": ["--repo", "."],
        "autoApprove": [
            "search_code",
            "get_symbol",
            "get_file_summary",
            "get_dependencies",
            "find_patterns",
            "get_architecture",
            "context_window",
            "search_by_intent",
            "get_module_map",
            "get_status"
        ],
        "disabled": false
    })
}

// ---------------------------------------------------------------------------
// JSON patcher
// ---------------------------------------------------------------------------

/// Regex-free check: does a key look like a legacy `omnicontext-<hex>` entry?
fn is_legacy_omnicontext_key(key: &str) -> bool {
    if !key.starts_with("omnicontext-") {
        return false;
    }
    let suffix = &key["omnicontext-".len()..];
    // Legacy keys have a 6–8 char lowercase hex suffix and nothing else
    !suffix.is_empty() && suffix.len() <= 12 && suffix.chars().all(|c| c.is_ascii_hexdigit())
}

/// Patch a config file with the universal entry and return the list of purged keys.
///
/// Steps:
/// 1. Read existing JSON (or start with `{}`).
/// 2. Navigate to `server_key` (create if absent).
/// 3. Purge all `omnicontext-<hex>` legacy keys.
/// 4. Insert / update the `"omnicontext"` universal key.
/// 5. Write back atomically (write to `.tmp` then rename).
fn patch_config(
    config_path: &Path,
    server_key: &str,
    entry: &serde_json::Value,
) -> Result<(bool, Vec<String>)> {
    // --- read ---------------------------------------------------------------
    let mut root: serde_json::Value = if config_path.exists() {
        let raw = std::fs::read_to_string(config_path)?;
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // --- navigate to server key (supports dotted nesting) -------------------
    let parts: Vec<&str> = server_key.split('.').collect();
    let mut current = &mut root;
    for part in &parts {
        if !current.is_object() {
            *current = serde_json::json!({});
        }
        if current.get(*part).is_none() {
            current[*part] = serde_json::json!({});
        }
        #[allow(clippy::expect_used)]
        {
            current = current.get_mut(*part).expect("just inserted");
        }
    }

    // --- purge legacy keys --------------------------------------------------
    let mut purged = Vec::new();
    if let Some(map) = current.as_object_mut() {
        let legacy_keys: Vec<String> = map
            .keys()
            .filter(|k| is_legacy_omnicontext_key(k))
            .cloned()
            .collect();
        for k in &legacy_keys {
            map.remove(k);
            purged.push(k.clone());
        }
    }

    // --- check if already current -------------------------------------------
    let already_current = current.get("omnicontext").is_some_and(|v| v == entry);

    if already_current && purged.is_empty() {
        return Ok((false, purged));
    }

    // --- insert universal entry ---------------------------------------------
    current["omnicontext"] = entry.clone();

    // --- atomic write --------------------------------------------------------
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json_output = serde_json::to_string_pretty(&root)?;

    // Write to a temp file then rename for atomicity
    let tmp_path = config_path.with_extension("omni-tmp");
    std::fs::write(&tmp_path, &json_output).inspect_err(|_| {
        // Clean up tmp if write failed
        let _ = std::fs::remove_file(&tmp_path);
    })?;
    std::fs::rename(&tmp_path, config_path)?;

    Ok((true, purged))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run the universal orchestration pass.
///
/// Discovers all installed IDEs, injects the universal `omnicontext` MCP entry,
/// purges legacy duplicates, and returns a rich result for display.
///
/// Pass `dry_run = true` to simulate without writing any files.
pub fn orchestrate(dry_run: bool) -> Result<OrchestrationResult> {
    let mcp_binary = find_mcp_binary()?;
    let entry = build_universal_entry(&mcp_binary);
    let targets = build_ide_targets();

    let mut results = Vec::with_capacity(targets.len());

    for target in targets {
        // Detect: does the config directory exist?
        let dir_exists = target
            .config_path
            .parent()
            .is_some_and(std::path::Path::exists);

        if !dir_exists && !target.config_path.exists() {
            results.push(IdeResult {
                name: target.name,
                status: IdeStatus::NotInstalled,
                purged: vec![],
            });
            continue;
        }

        if dry_run {
            results.push(IdeResult {
                name: target.name,
                status: IdeStatus::Configured, // would configure
                purged: vec![],
            });
            continue;
        }

        match patch_config(&target.config_path, target.server_key, &entry) {
            Ok((changed, purged)) => {
                let status = if changed {
                    IdeStatus::Configured
                } else {
                    IdeStatus::AlreadyCurrent
                };
                results.push(IdeResult {
                    name: target.name,
                    status,
                    purged,
                });
            }
            Err(e) => {
                // Distinguish permission errors from generic I/O errors
                let status = if e.to_string().to_lowercase().contains("permission denied")
                    || e.to_string().to_lowercase().contains("access is denied")
                {
                    IdeStatus::PermissionDenied(e.to_string())
                } else {
                    IdeStatus::Error(e.to_string())
                };
                results.push(IdeResult {
                    name: target.name,
                    status,
                    purged: vec![],
                });
            }
        }
    }

    Ok(OrchestrationResult {
        results,
        mcp_binary,
    })
}

/// Print a high-fidelity ASCII success matrix to stdout.
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │  OmniContext — Universal IDE Configuration                   │
/// │  MCP Binary: C:\Users\…\omnicontext-mcp.exe                 │
/// ├───────────────────┬────────────┬──────────────────────────  │
/// │  IDE              │  Status    │  Notes                      │
/// ├───────────────────┼────────────┼──────────────────────────  │
/// │  Claude Desktop   │  ✓ wired   │                            │
/// │  Cursor           │  ✓ wired   │  2 legacy entries purged    │
/// │  VS Code          │  — skip    │  not installed              │
/// └───────────────────┴────────────┴──────────────────────────  ┘
/// ```
pub fn print_orchestration_matrix(result: &OrchestrationResult, dry_run: bool) {
    let title = if dry_run {
        "OmniContext — Universal IDE Configuration [DRY RUN]"
    } else {
        "OmniContext — Universal IDE Configuration"
    };

    let mcp_display = result.mcp_binary.display().to_string();

    // Column widths
    let name_w = 20usize;
    let status_w = 14usize;
    let notes_w = 32usize;
    let total_w = name_w + status_w + notes_w + 6; // separators

    let rule = "─".repeat(total_w);
    let top = format!("┌{rule}┐");
    let mid = format!("├{rule}┤");
    let bot = format!("└{rule}┘");
    let sep = format!("│{:─<name_w$}┼{:─<status_w$}┼{:─<notes_w$}│", "", "", "");

    println!();
    println!("{top}");
    println!("│ {:<width$}│", title, width = total_w - 1);
    println!(
        "│ {:<width$}│",
        format!(
            "MCP: {}",
            &mcp_display[..mcp_display.len().min(total_w - 7)]
        ),
        width = total_w - 1
    );
    println!("{mid}");
    println!(
        "│ {:<name_w$}│ {:<status_w$}│ {:<notes_w$}│",
        "IDE", "Status", "Notes"
    );
    println!("{sep}");

    let installed_count = result
        .results
        .iter()
        .filter(|r| r.status.is_installed())
        .count();
    let ok_count = result
        .results
        .iter()
        .filter(|r| r.status.is_ok() && r.status.is_installed())
        .count();

    for r in &result.results {
        let (status_str, notes_str) = match &r.status {
            IdeStatus::Configured => ("✓ wired".to_string(), {
                if r.purged.is_empty() {
                    String::new()
                } else {
                    format!(
                        "{} legacy entr{} purged",
                        r.purged.len(),
                        if r.purged.len() == 1 { "y" } else { "ies" }
                    )
                }
            }),
            IdeStatus::AlreadyCurrent => ("✓ current".to_string(), "no change needed".to_string()),
            IdeStatus::NotInstalled => ("— skip".to_string(), "not installed".to_string()),
            IdeStatus::PermissionDenied(_) => {
                ("✗ denied".to_string(), "read-only config".to_string())
            }
            IdeStatus::Error(e) => ("✗ error".to_string(), e.chars().take(notes_w).collect()),
        };

        // Truncate to column widths
        let name_col = format!("{:<name_w$}", truncate(r.name, name_w));
        let stat_col = format!("{:<status_w$}", truncate(&status_str, status_w));
        let note_col = format!("{:<notes_w$}", truncate(&notes_str, notes_w));

        println!("│ {name_col}│ {stat_col}│ {note_col}│");
    }

    println!("{bot}");
    println!();

    if dry_run {
        println!("  [DRY RUN] No files written. Run without --dry-run to apply.");
    } else {
        println!("  Detected: {installed_count} IDE(s)   Configured: {ok_count}   Purged: {} legacy entr{}",
            result.total_purged(),
            if result.total_purged() == 1 { "y" } else { "ies" });
    }

    if ok_count > 0 && !dry_run {
        println!();
        println!("  Restart your IDE(s) to activate OmniContext.");
        println!("  The `omnicontext` tool will be visible in every configured agent.");
    }

    println!();
}

fn truncate(s: &str, max: usize) -> &str {
    let mut end = s.len().min(max);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

// ---------------------------------------------------------------------------
// Health check (self-repair)
// ---------------------------------------------------------------------------

/// Silently re-inject the universal entry for any IDE that has lost it.
///
/// Intended to be called at the start of every `omnicontext` invocation so
/// that IDE updates that overwrite configs are auto-healed.
#[allow(dead_code)]
pub fn health_check_silent() {
    if let Ok(result) = orchestrate(false) {
        let configured = result
            .results
            .iter()
            .filter(|r| matches!(r.status, IdeStatus::Configured))
            .count();
        if configured > 0 {
            tracing::debug!(
                healed = configured,
                "orchestrator re-injected omnicontext entry for {configured} IDE(s)"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_key_detection() {
        assert!(is_legacy_omnicontext_key("omnicontext-2fd8bc"));
        assert!(is_legacy_omnicontext_key("omnicontext-15ba29"));
        assert!(is_legacy_omnicontext_key("omnicontext-aabbcc"));
        assert!(!is_legacy_omnicontext_key("omnicontext"));
        assert!(!is_legacy_omnicontext_key("omnicontext-"));
        assert!(!is_legacy_omnicontext_key("cursor"));
        // Long hex string should still match (up to 12 chars)
        assert!(is_legacy_omnicontext_key("omnicontext-abc123"));
        // Non-hex suffix should not match
        assert!(!is_legacy_omnicontext_key("omnicontext-myrepo"));
    }

    #[test]
    fn test_patch_config_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("mcp.json");
        let entry = serde_json::json!({ "command": "omnicontext-mcp", "args": ["--repo", "."] });

        let (changed, purged) = patch_config(&config, "mcpServers", &entry).unwrap();
        assert!(changed);
        assert!(purged.is_empty());
        assert!(config.exists());

        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        assert_eq!(written["mcpServers"]["omnicontext"], entry);
    }

    #[test]
    fn test_patch_config_purges_legacy() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.json");

        // Pre-populate with legacy entries
        let initial = serde_json::json!({
            "mcpServers": {
                "omnicontext-2fd8bc": { "command": "old-binary", "args": ["--repo", "/some/path"] },
                "omnicontext-15ba29": { "command": "old-binary2", "args": [] },
                "other-tool": { "command": "other" }
            }
        });
        std::fs::write(&config, serde_json::to_string_pretty(&initial).unwrap()).unwrap();

        let entry = serde_json::json!({ "command": "omnicontext-mcp", "args": ["--repo", "."] });
        let (changed, purged) = patch_config(&config, "mcpServers", &entry).unwrap();

        assert!(changed);
        assert_eq!(purged.len(), 2);

        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();

        // Universal entry present
        assert_eq!(written["mcpServers"]["omnicontext"], entry);
        // Legacy entries gone
        assert!(written["mcpServers"].get("omnicontext-2fd8bc").is_none());
        assert!(written["mcpServers"].get("omnicontext-15ba29").is_none());
        // Other tools preserved
        assert!(written["mcpServers"].get("other-tool").is_some());
    }

    #[test]
    fn test_patch_config_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.json");

        let entry = serde_json::json!({ "command": "omnicontext-mcp", "args": ["--repo", "."] });
        patch_config(&config, "mcpServers", &entry).unwrap();

        // Second call should report no change
        let (changed, purged) = patch_config(&config, "mcpServers", &entry).unwrap();
        assert!(!changed);
        assert!(purged.is_empty());
    }

    #[test]
    fn test_patch_config_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.json");

        // Write invalid JSON — should be treated as empty object, not error
        std::fs::write(&config, b"{ this is not valid JSON!!!").unwrap();

        let entry = serde_json::json!({ "command": "mcp", "args": [] });
        let result = patch_config(&config, "mcpServers", &entry);
        assert!(result.is_ok());

        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        assert!(written["mcpServers"]["omnicontext"].is_object());
    }

    #[test]
    fn test_patch_config_nested_server_key() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("settings.json");

        let entry = serde_json::json!({ "command": "mcp", "args": [] });
        patch_config(&config, "powers.mcpServers", &entry).unwrap();

        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
        assert!(written["powers"]["mcpServers"]["omnicontext"].is_object());
    }

    #[test]
    fn test_build_universal_entry_has_repo_dot_arg() {
        use std::path::PathBuf;
        let entry = build_universal_entry(&PathBuf::from("omnicontext-mcp"));
        let args = entry["args"].as_array().unwrap();
        assert_eq!(args[0].as_str().unwrap(), "--repo");
        assert_eq!(args[1].as_str().unwrap(), ".");
    }
}
