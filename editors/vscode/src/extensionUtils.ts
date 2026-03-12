/**
 * Pure-function utilities extracted from extension.ts for testability.
 * No VS Code API dependencies -- these functions are testable in any environment.
 */

import * as crypto from "crypto";

/**
 * Derive the IPC named pipe / Unix socket path from a repository root.
 *
 * Uses a SHA-256 hash of the normalized path to create a deterministic,
 * filesystem-safe pipe name. Consistent across extension restarts.
 *
 * Normalization must match the daemon's `default_pipe_name()`:
 *   1. Strip `\\?\` prefix
 *   2. Backslash -> forward slash
 *   3. Lowercase
 *   4. Strip trailing separator(s)
 */
export function derivePipeName(repoRoot: string): string {
  let normalized = repoRoot
    .replace("\\\\?\\", "")
    .replace(/\\/g, "/")
    .toLowerCase();

  // Strip trailing separator(s) for consistency
  while (normalized.endsWith("/")) {
    normalized = normalized.slice(0, -1);
  }

  const hash = crypto
    .createHash("sha256")
    .update(normalized)
    .digest("hex")
    .substring(0, 12);

  if (process.platform === "win32") {
    return `\\\\.\\pipe\\omnicontext-${hash}`;
  }
  const runtimeDir = process.env.XDG_RUNTIME_DIR || "/tmp";
  return `${runtimeDir}/omnicontext-${hash}.sock`;
}

/**
 * Assemble a context document from search results.
 * Used as the CLI fallback when the daemon is not available.
 *
 * Returns null if no results are available.
 */
export function assembleCliContext(
  results: CliSearchResult[],
  tokenBudget: number,
  elapsedMs: number,
): AssembledContext | null {
  if (!results || results.length === 0) return null;

  let context = "<context_engine>\n";
  context +=
    "OmniContext has analyzed the codebase and identified the following relevant code.\n\n";
  context += "## Relevant Code\n\n";

  let tokensUsed = 0;
  let entriesCount = 0;

  for (const r of results) {
    const chunkTokens = Math.ceil(r.content.length / 4);
    if (tokensUsed + chunkTokens > tokenBudget) break;

    context += `### ${sanitizeForDisplay(r.symbol)} (${sanitizeForDisplay(r.kind)}, score: ${r.score.toFixed(4)})\n`;
    context += `File: ${sanitizeForDisplay(r.file)}\n`;
    context += `\`\`\`\n${r.content}\n\`\`\`\n\n`;

    tokensUsed += chunkTokens;
    entriesCount++;
  }

  context += "</context_engine>\n";

  return {
    system_context: context,
    entries_count: entriesCount,
    tokens_used: tokensUsed,
    token_budget: tokenBudget,
    elapsed_ms: elapsedMs,
  };
}

/**
 * Sanitize a string for safe display in markdown.
 * Strips characters that could break markdown formatting or inject directives.
 */
export function sanitizeForDisplay(input: string): string {
  if (!input) return "";
  return input
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F]/g, "") // strip control chars (keep \n, \r, \t)
    .replace(/`{3,}/g, "``") // prevent code fence escaping
    .substring(0, 500); // hard length limit
}

/**
 * Build the JSON-RPC request payload for IPC communication.
 */
export function buildJsonRpcRequest(
  id: number,
  method: string,
  params: any,
): string {
  const request = {
    jsonrpc: "2.0",
    id,
    method,
    params,
  };
  return JSON.stringify(request) + "\n";
}

/**
 * Parse a JSON-RPC response from the daemon.
 * Returns the parsed response or null if invalid.
 */
export function parseJsonRpcResponse(line: string): JsonRpcResponse | null {
  if (!line.trim()) return null;
  try {
    const parsed = JSON.parse(line);
    if (typeof parsed.id !== "number") return null;
    return parsed as JsonRpcResponse;
  } catch {
    return null;
  }
}

/**
 * Calculate exponential backoff delay for reconnection.
 * Capped at 30 seconds.
 */
export function calculateBackoffDelay(attempt: number): number {
  return Math.min(1000 * Math.pow(2, attempt), 30000);
}

/**
 * Derive the MCP binary path from the main binary path.
 */
export function deriveMcpBinaryPath(binaryPath: string): string {
  return binaryPath.replace(/omnicontext(\.exe)?$/, "omnicontext-mcp$1");
}

/**
 * Derive the daemon binary path from the main binary path.
 */
export function deriveDaemonBinaryPath(binaryPath: string): string {
  return binaryPath.replace(/omnicontext(\.exe)?$/, "omnicontext-daemon$1");
}

/**
 * Format preflight context with cache indicators.
 */
export function formatPreflightContext(
  systemContext: string,
  elapsedMs: number,
  fromCache: boolean,
): string {
  if (fromCache) {
    return `[Cached context, ${elapsedMs}ms]\n\n${systemContext}`;
  }
  return `[Fresh search, ${elapsedMs}ms]\n\n${systemContext}`;
}

// ---------------------------------------------------------------------------
// MCP Client Discovery & Auto-Configuration
// ---------------------------------------------------------------------------

/**
 * Known AI client MCP configuration targets.
 * Each entry defines where the client stores its MCP server config
 * across Windows, macOS, and Linux.
 */
export interface McpClientTarget {
  /** Human-readable client name */
  name: string;
  /** Config file path (platform-aware) */
  configPath: string;
  /** JSON path to the mcpServers object within the config */
  serversKey: string;
  /** Whether this client wraps servers in a "powers" namespace */
  usesPowersNamespace: boolean;
}

/**
 * Get all known AI client config paths for the current platform.
 */
export function getKnownMcpClients(): McpClientTarget[] {
  const home = process.env.HOME || process.env.USERPROFILE || "";
  const appData = process.env.APPDATA || "";
  const isWin = process.platform === "win32";
  const isMac = process.platform === "darwin";

  const targets: McpClientTarget[] = [];

  // Claude Desktop
  if (isWin) {
    targets.push({
      name: "Claude Desktop",
      configPath: joinPath(appData, "Claude", "claude_desktop_config.json"),
      serversKey: "mcpServers",
      usesPowersNamespace: false,
    });
  } else if (isMac) {
    targets.push({
      name: "Claude Desktop",
      configPath: joinPath(
        home,
        "Library",
        "Application Support",
        "Claude",
        "claude_desktop_config.json",
      ),
      serversKey: "mcpServers",
      usesPowersNamespace: false,
    });
  } else {
    targets.push({
      name: "Claude Desktop",
      configPath: joinPath(
        home,
        ".config",
        "claude",
        "claude_desktop_config.json",
      ),
      serversKey: "mcpServers",
      usesPowersNamespace: false,
    });
  }

  // Cursor — global user config (same as VS Code fork pattern)
  if (isWin) {
    targets.push({
      name: "Cursor",
      configPath: joinPath(appData, "Cursor", "User", "mcp.json"),
      serversKey: "mcpServers",
      usesPowersNamespace: false,
    });
  } else if (isMac) {
    targets.push({
      name: "Cursor",
      configPath: joinPath(home, "Library", "Application Support", "Cursor", "User", "mcp.json"),
      serversKey: "mcpServers",
      usesPowersNamespace: false,
    });
  } else {
    targets.push({
      name: "Cursor",
      configPath: joinPath(home, ".cursor", "mcp.json"),
      serversKey: "mcpServers",
      usesPowersNamespace: false,
    });
  }

  // VS Code-family MCP user configs (for IDEs that support user-level mcp.json)
  if (isWin) {
    targets.push(
      {
        name: "VS Code",
        configPath: joinPath(appData, "Code", "User", "mcp.json"),
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      },
      {
        name: "VS Code Insiders",
        configPath: joinPath(appData, "Code - Insiders", "User", "mcp.json"),
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      },
      {
        name: "VSCodium",
        configPath: joinPath(appData, "VSCodium", "User", "mcp.json"),
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      },
    );
  } else if (isMac) {
    targets.push(
      {
        name: "VS Code",
        configPath: joinPath(
          home,
          "Library",
          "Application Support",
          "Code",
          "User",
          "mcp.json",
        ),
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      },
      {
        name: "VS Code Insiders",
        configPath: joinPath(
          home,
          "Library",
          "Application Support",
          "Code - Insiders",
          "User",
          "mcp.json",
        ),
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      },
      {
        name: "VSCodium",
        configPath: joinPath(
          home,
          "Library",
          "Application Support",
          "VSCodium",
          "User",
          "mcp.json",
        ),
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      },
    );
  } else {
    targets.push(
      {
        name: "VS Code",
        configPath: joinPath(home, ".config", "Code", "User", "mcp.json"),
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      },
      {
        name: "VS Code Insiders",
        configPath: joinPath(
          home,
          ".config",
          "Code - Insiders",
          "User",
          "mcp.json",
        ),
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      },
      {
        name: "VSCodium",
        configPath: joinPath(home, ".config", "VSCodium", "User", "mcp.json"),
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      },
    );
  }

  // Continue.dev
  targets.push({
    name: "Continue.dev",
    configPath: joinPath(home, ".continue", "config.json"),
    serversKey: "mcpServers",
    usesPowersNamespace: false,
  });

  // Kiro
  targets.push({
    name: "Kiro",
    configPath: joinPath(home, ".kiro", "settings", "mcp.json"),
    serversKey: "mcpServers",
    usesPowersNamespace: false,
  });

  // Windsurf (Codeium) — global config at ~/.codeium/windsurf/
  // Verified: https://docs.windsurf.com/windsurf/cascade/mcp
  targets.push({
    name: "Windsurf",
    configPath: joinPath(home, ".codeium", "windsurf", "mcp_config.json"),
    serversKey: "mcpServers",
    usesPowersNamespace: false,
  });

  // Cline (VS Code extension — saoudrizwan.claude-dev)
  if (isWin) {
    targets.push({
      name: "Cline",
      configPath: joinPath(
        appData, "Code", "User", "globalStorage",
        "saoudrizwan.claude-dev", "settings", "cline_mcp_settings.json",
      ),
      serversKey: "mcpServers",
      usesPowersNamespace: false,
    });
  } else if (isMac) {
    targets.push({
      name: "Cline",
      configPath: joinPath(
        home, "Library", "Application Support", "Code", "User",
        "globalStorage", "saoudrizwan.claude-dev", "settings", "cline_mcp_settings.json",
      ),
      serversKey: "mcpServers",
      usesPowersNamespace: false,
    });
  } else {
    targets.push({
      name: "Cline",
      configPath: joinPath(
        home, ".config", "Code", "User", "globalStorage",
        "saoudrizwan.claude-dev", "settings", "cline_mcp_settings.json",
      ),
      serversKey: "mcpServers",
      usesPowersNamespace: false,
    });
  }

  // Gemini CLI (google-gemini/gemini-cli)
  // Verified: https://github.com/google-gemini/gemini-cli
  targets.push({
    name: "Gemini CLI",
    configPath: joinPath(home, ".gemini", "settings.json"),
    serversKey: "mcpServers",
    usesPowersNamespace: false,
  });

  // Amazon Q CLI
  targets.push({
    name: "Amazon Q CLI",
    configPath: joinPath(home, ".aws", "amazonq", "mcp.json"),
    serversKey: "mcpServers",
    usesPowersNamespace: false,
  });

  return targets;
}

/**
 * Build the MCP server entry for OmniContext.
 * Uses a workspace-specific key derived from the repo root hash to
 * prevent multiple VS Code windows from overwriting each other.
 *
 * Defense-in-depth: passes the repo path via three independent channels
 * so external AI agents (Antigravity, Claude Desktop, Cursor, etc.)
 * always resolve the correct workspace even if one mechanism is stripped:
 *   1. --repo <path>           (primary)
 *   2. --cwd  <path>           (fallback if --repo is swallowed)
 *   3. OMNICONTEXT_REPO env    (fallback if args are not forwarded)
 *
 * @throws if repoRoot is relative, empty, or a placeholder -- this prevents
 * writing broken MCP configs that silently resolve to the wrong directory.
 */
export function buildMcpServerEntry(
  mcpBinaryPath: string,
  repoRoot: string,
): McpServerEntry {
  // Guard: relative paths like "." silently resolve to the AI launcher's
  // install directory, which is the root cause of the wrong-repo bug.
  if (
    !repoRoot ||
    repoRoot === "." ||
    repoRoot === "REPLACE_WITH_YOUR_REPO_PATH"
  ) {
    throw new Error(
      `buildMcpServerEntry: repoRoot must be an absolute path, got "${repoRoot}"`,
    );
  }
  // On Windows, absolute paths start with a drive letter (e.g., C:\).
  // On Unix, they start with /.
  const looksAbsolute =
    repoRoot.startsWith("/") || /^[a-zA-Z]:[\\/]/.test(repoRoot);
  if (!looksAbsolute) {
    throw new Error(
      `buildMcpServerEntry: repoRoot must be an absolute path, got "${repoRoot}"`,
    );
  }

  return {
    command: mcpBinaryPath,
    args: ["--repo", repoRoot, "--cwd", repoRoot],
    env: {
      OMNICONTEXT_REPO: repoRoot,
    },
    disabled: false,
  };
}

/**
 * Derive a short workspace key for MCP server config entries.
 * Uses a 6-char hash to create unique keys like "omnicontext-a1b2c3".
 */
export function deriveMcpEntryKey(repoRoot: string): string {
  const hash = crypto
    .createHash("sha256")
    .update(repoRoot.replace(/\\/g, "/").toLowerCase())
    .digest("hex")
    .substring(0, 6);
  return `omnicontext-${hash}`;
}

/**
 * Merge an OmniContext MCP entry into a client's config JSON.
 * Returns the updated config object. Does NOT write to disk.
 * Uses workspace-specific keys to avoid overwriting configs from other workspaces.
 *
 * Also cleans up any legacy bare "omnicontext" entries that use `--repo "."` without
 * a `--cwd` fallback, as these resolve to the AI launcher's install directory.
 */
export function mergeMcpConfig(
  existingJson: string | null,
  target: McpClientTarget,
  entry: McpServerEntry,
  entryKey: string = "omnicontext",
): any {
  let config: any = {};

  if (existingJson) {
    try {
      config = JSON.parse(existingJson);
    } catch {
      config = {};
    }
  }

  const getServers = (): any => {
    if (target.usesPowersNamespace) {
      return config.powers?.[target.serversKey];
    }
    return config[target.serversKey];
  };

  // Cleanup: remove legacy bare "omnicontext" entries that use --repo "." without --cwd.
  // These entries cause the MCP server to resolve "." against the agent launcher's install
  // directory (e.g., Antigravity's Program Files dir) instead of the user's project.
  const servers = getServers();
  if (servers?.["omnicontext"]) {
    const bareEntry = servers["omnicontext"];
    const args: string[] = bareEntry.args || [];
    const repoIdx = args.indexOf("--repo");
    const cwdIdx = args.indexOf("--cwd");
    const hasEnv = bareEntry.env?.OMNICONTEXT_REPO;
    // If it has --repo "." but no --cwd and no OMNICONTEXT_REPO env, it's a broken legacy entry
    if (repoIdx >= 0 && args[repoIdx + 1] === "." && cwdIdx === -1 && !hasEnv) {
      delete servers["omnicontext"];
    }
  }

  if (target.usesPowersNamespace) {
    if (!config.powers) config.powers = {};
    if (!config.powers[target.serversKey])
      config.powers[target.serversKey] = {};
    config.powers[target.serversKey][entryKey] = entry;
  } else {
    if (!config[target.serversKey]) config[target.serversKey] = {};
    config[target.serversKey][entryKey] = entry;
  }

  return config;
}

export interface McpServerEntry {
  command: string;
  args: string[];
  env?: Record<string, string>;
  disabled?: boolean;
  autoApprove?: string[];
}

/** Simple path joiner that doesn't require the `path` module. */
function joinPath(...parts: string[]): string {
  return parts.join(process.platform === "win32" ? "\\" : "/");
}

// ----------- Types -----------

export interface CliSearchResult {
  symbol: string;
  kind: string;
  score: number;
  file: string;
  content: string;
  line_start: number;
  line_end: number;
}

export interface AssembledContext {
  system_context: string;
  entries_count: number;
  tokens_used: number;
  token_budget: number;
  elapsed_ms: number;
}

export interface JsonRpcResponse {
  jsonrpc: string;
  id: number;
  result?: any;
  error?: { code: number; message: string };
}
