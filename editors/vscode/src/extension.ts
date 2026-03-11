import * as vscode from "vscode";
import * as cp from "child_process";
import * as path from "path";
import * as net from "net";
import * as fs from "fs";
import { EventTracker } from "./eventTracker";
import { SymbolExtractor } from "./symbolExtractor";
import { OmniSidebarProvider } from "./sidebarProvider";
import { CacheStatsManager } from "./cacheStats";
import { PreflightResponse } from "./types";
import {
  derivePipeName as computePipeName,
  assembleCliContext,
  buildJsonRpcRequest,
  parseJsonRpcResponse,
  calculateBackoffDelay,
  deriveMcpBinaryPath,
  deriveMcpEntryKey,
  formatPreflightContext,
  getKnownMcpClients,
  buildMcpServerEntry,
  mergeMcpConfig,
} from "./extensionUtils";
import {
  bootstrap,
  resolveBinaries,
  BootstrapResult,
} from "./bootstrapService";
import { registerRepo } from "./repoRegistry";

let statusBarItem: vscode.StatusBarItem;
let outputChannel: vscode.OutputChannel;
let daemonProcess: cp.ChildProcess | null = null;
let ipcClient: net.Socket | null = null;
let contextInjectionEnabled: boolean = true;
let requestCounter = 0;
const pendingRequests = new Map<
  number,
  {
    method: string;
    startedAt: number;
    resolve: (v: any) => void;
    reject: (e: Error) => void;
  }
>();

// IPC reconnection state
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 10;
let reconnectTimer: NodeJS.Timeout | null = null;
let currentRepoRoot: string | null = null;

// Event tracking
let eventTracker: EventTracker | null = null;
let symbolExtractor: SymbolExtractor | null = null;
let cacheStatsManager: CacheStatsManager | null = null;
let sidebarRefreshInterval: NodeJS.Timeout | null = null;
let healthCheckInterval: NodeJS.Timeout | null = null;
const HEALTH_CHECK_INTERVAL_MS = 60_000;
let consecutiveHealthFailures = 0;
const MAX_HEALTH_FAILURES = 3;

// ---------------------------------------------------------------------------
// Circuit breaker: prevents sidebar and event tracker from hammering a
// non-existent daemon with IPC requests (the root cause of sidebar freezes).
// ---------------------------------------------------------------------------
let isDaemonConnected = false;

/** Export for use in sidebar and event tracker as a lightweight check. */
export function getDaemonConnected(): boolean {
  return isDaemonConnected;
}

// Bootstrap: resolved binary paths after zero-friction setup
let bootstrapResult: BootstrapResult | null = null;

// VS Code extension context (stored for binary resolution after bootstrap)
let extensionContext: vscode.ExtensionContext | null = null;

// ---------------------------------------------------------------------------
// Extension lifecycle
// ---------------------------------------------------------------------------

export function activate(context: vscode.ExtensionContext) {
  extensionContext = context;
  outputChannel = vscode.window.createOutputChannel("OmniContext");

  // Status bar
  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    100,
  );
  statusBarItem.text = "$(sync~spin) OmniContext: Starting...";
  statusBarItem.tooltip = "OmniContext: Initializing engine";
  statusBarItem.command = "omnicontext.search";
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  // Initialize event tracking and cache stats.
  // Event tracker checks isDaemonConnected before enqueuing any events.
  symbolExtractor = new SymbolExtractor();
  cacheStatsManager = new CacheStatsManager(sendIpcRequest);
  eventTracker = new EventTracker(sendIpcRequest, symbolExtractor);
  eventTracker.setConnectionGate(getDaemonConnected);

  eventTracker.registerListeners(context);

  // Sidebar Provider
  const sidebarProvider = new OmniSidebarProvider(
    context.extensionUri,
    cacheStatsManager,
    eventTracker,
    sendIpcRequest,
    getDaemonConnected,
  );
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      "omnicontext.mainView",
      sidebarProvider,
    ),
  );

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand("omnicontext.index", () => runIndex()),
    vscode.commands.registerCommand("omnicontext.search", () => runSearch()),
    vscode.commands.registerCommand("omnicontext.status", () => runStatus()),
    vscode.commands.registerCommand("omnicontext.startMcp", () => startMcp()),
    vscode.commands.registerCommand("omnicontext.startDaemon", () =>
      startDaemon(),
    ),
    vscode.commands.registerCommand("omnicontext.stopDaemon", () =>
      stopDaemon(),
    ),
    vscode.commands.registerCommand("omnicontext.toggleInjection", () =>
      toggleContextInjection(),
    ),
    vscode.commands.registerCommand("omnicontext.preflight", () =>
      runPreflight(),
    ),
    vscode.commands.registerCommand("omnicontext.moduleMap", () =>
      runModuleMap(),
    ),
    vscode.commands.registerCommand("omnicontext.syncMcp", () => runSyncMcp()),
    vscode.commands.registerCommand("omnicontext.repair", () =>
      runRepairEnvironment(),
    ),
    vscode.commands.registerCommand("omnicontext.refreshSidebar", () => {
      sidebarProvider.refresh();
    }),
    vscode.commands.registerCommand("omnicontext.cleanupIndexes", () =>
      runCleanupIndexes(),
    ),
    vscode.commands.registerCommand("omnicontext.updateBinary", () =>
      runUpdateBinary(),
    ),
  );

  // Register the chat participant for context injection
  registerChatParticipant(context);

  // Register configuration change listener
  registerConfigurationWatcher(context);

  // ---------------------------------------------------------------------------
  // Bootstrap: resolve or auto-download binaries, then start daemon.
  // This runs async so activation returns immediately (VS Code requirement).
  // The sidebar shows bootstrap progress via the status callback.
  // ---------------------------------------------------------------------------
  vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: "OmniContext",
      cancellable: false,
    },
    async (progress) => {
      try {
        bootstrapResult = await bootstrap(context, (status) => {
          outputChannel.appendLine(
            `[bootstrap] ${status.phase}: ${status.message}`,
          );
          sidebarProvider.sendBootstrapStatus(status);

          if (status.progressPercent !== undefined) {
            progress.report({
              message: status.message,
              increment: status.progressPercent,
            });
          } else {
            progress.report({ message: status.message });
          }

          if (status.phase === "failed") {
            statusBarItem.text = "$(error) OmniContext: Setup Failed";
            statusBarItem.tooltip = status.message;
          }
        });

        outputChannel.appendLine(
          `[bootstrap] engine ready at: ${bootstrapResult.cliBinary}`,
        );
        statusBarItem.text = "$(search) OmniContext";
        statusBarItem.tooltip = "OmniContext: Click to search";

        // Now that binaries are confirmed present, start the daemon
        const config = vscode.workspace.getConfiguration("omnicontext");
        if (config.get<boolean>("autoStartDaemon", true)) {
          await startDaemon(true);
        } else if (config.get<boolean>("autoIndex", true)) {
          // Auto-index: start indexing for any workspace with a valid root.
          // Previously-indexed repos re-index silently; new repos are registered and indexed.
          const root = getWorkspaceRoot();
          if (root) {
            const { isRepoIndexed, registerRepo } =
              await import("./repoRegistry");
            if (!isRepoIndexed(root)) {
              // New workspace: register it so subsequent activations skip the first-time path.
              registerRepo(root, 0, 0);
            }
            await runIndex(true);
          }
        }

        sidebarProvider.refresh();
      } catch (err: any) {
        outputChannel.appendLine(`[bootstrap] fatal: ${err.message}`);
        statusBarItem.text = "$(error) OmniContext";
        statusBarItem.tooltip =
          "OmniContext setup failed. Check OmniContext output for details.";
        sidebarProvider.refresh();
      }
    },
  );

  // Poll for status updates for sidebar.
  sidebarRefreshInterval = setInterval(() => {
    sidebarProvider.refresh();
  }, 30000);

  // Auto-sync MCP on workspace/editor changes (throttled).
  // This automatically "auto-corrects" the MCP path when switching repositories.
  let syncTimer: NodeJS.Timeout | null = null;
  const scheduleSync = () => {
    if (syncTimer) return;
    syncTimer = setTimeout(async () => {
      syncTimer = null;
      await syncMcpSilent();
    }, 5000); // 5s debounce for quiet disk I/O
  };

  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(scheduleSync),
    vscode.workspace.onDidChangeWorkspaceFolders(async () => {
      scheduleSync();

      // Restart daemon for the new workspace root so we don't talk to a
      // stale pipe connected to the old project.
      const newRoot = getWorkspaceRoot();
      if (newRoot && newRoot !== currentRepoRoot) {
        outputChannel.appendLine(
          `[workspace] root changed: ${currentRepoRoot} → ${newRoot}`,
        );
        stopDaemon();
        const config = vscode.workspace.getConfiguration("omnicontext");
        if (config.get<boolean>("autoStartDaemon", true)) {
          await startDaemon(true);
        }
        sidebarProvider.refresh();
      }
    }),
  );
}

export function deactivate() {
  if (sidebarRefreshInterval) {
    clearInterval(sidebarRefreshInterval);
    sidebarRefreshInterval = null;
  }
  stopHealthCheck();
  stopDaemon();
  eventTracker?.dispose();
  eventTracker = null;
  symbolExtractor = null;
  statusBarItem?.dispose();
  outputChannel?.dispose();
}

async function runSyncMcp() {
  const result = await syncMcpToClients();

  if (result.synced > 0) {
    const names = result.syncedClients.join(", ");
    vscode.window.showInformationMessage(
      `OmniContext synced to ${result.synced} AI client(s): ${names}. Restart your AI chat to see it!`,
    );
  } else {
    vscode.window.showWarningMessage(
      "No supported AI clients found for auto-sync. Configure manually.",
    );
  }
}

/**
 * Silent MCP sync -- called automatically on daemon start.
 * Does not show warnings, only logs results.
 */
async function syncMcpSilent(): Promise<void> {
  const config = vscode.workspace.getConfiguration("omnicontext");
  if (!config.get<boolean>("autoSyncMcp", true)) return;

  const result = await syncMcpToClients();
  if (result.synced > 0) {
    outputChannel.appendLine(
      `[mcp-sync] auto-synced to ${result.synced} client(s): ${result.syncedClients.join(", ")}`,
    );
  }
}

/**
 * Core MCP sync logic. Discovers all installed AI clients and writes
 * OmniContext MCP server config to each.
 *
 * It syncs:
 *   1. The currently active workspace to the "omnicontext" primary key (auto-correction).
 *   2. ALL indexed repositories to their unique "omnicontext-<hash>" keys.
 *
 * Path safety: validates all paths are absolute before writing. Relative
 * paths like "." silently resolve to the AI launcher's install directory.
 */
async function syncMcpToClients(): Promise<{
  synced: number;
  syncedClients: string[];
}> {
  const binary = getBinaryPath();
  if (!binary) return { synced: 0, syncedClients: [] };

  const mcpBinary = deriveMcpBinaryPath(binary);
  const activeRoot = getWorkspaceRoot();
  const clients = getKnownMcpClients();

  // Load indexed repos from registry
  const { getIndexedRepos } = await import("./repoRegistry");
  const indexedRepos = getIndexedRepos();

  // Validate active workspace root: must be absolute and exist on disk.
  // Relative paths like "." are the root cause of the "wrong repo" bug.
  const isValidRoot =
    activeRoot && path.isAbsolute(activeRoot) && fs.existsSync(activeRoot);

  // Also validate indexed repo paths
  const validIndexedRepos = indexedRepos.filter((repo) => {
    if (!repo.repoPath || !path.isAbsolute(repo.repoPath)) {
      outputChannel.appendLine(
        `[mcp-sync] skipping repo with non-absolute path: ${repo.repoPath}`,
      );
      return false;
    }
    return true;
  });

  let synced = 0;
  const syncedClients: string[] = [];

  for (const client of clients) {
    const configDir = path.dirname(client.configPath);

    // Skip clients that aren't installed (config dir doesn't exist)
    if (!fs.existsSync(configDir)) continue;

    try {
      const existingJson = fs.existsSync(client.configPath)
        ? fs.readFileSync(client.configPath, "utf-8")
        : null;

      let merged: any = null;

      // 1. Always sync the active workspace to the "omnicontext" primary key.
      // This is the "auto-correction" path: when a user opens a new project,
      // the primary entry is updated to point to that project.
      if (isValidRoot) {
        const primaryEntry = buildMcpServerEntry(mcpBinary, activeRoot);
        merged = mergeMcpConfig(
          existingJson,
          client,
          primaryEntry,
          "omnicontext",
        );
      }

      // 2. Sync ALL indexed repositories using unique workspace keys.
      // This ensures all repos are available simultaneously in agents that support it.
      for (const repo of validIndexedRepos) {
        const entry = buildMcpServerEntry(mcpBinary, repo.repoPath);
        const entryKey = deriveMcpEntryKey(repo.repoPath);
        merged = mergeMcpConfig(
          merged ? JSON.stringify(merged) : existingJson,
          client,
          entry,
          entryKey,
        );
      }

      if (merged) {
        const configStr = JSON.stringify(merged, null, 2);
        fs.writeFileSync(client.configPath, configStr, "utf-8");

        // Verify the write succeeded by reading back
        try {
          const readBack = fs.readFileSync(client.configPath, "utf-8");
          JSON.parse(readBack); // validate JSON
        } catch (verifyErr: any) {
          outputChannel.appendLine(
            `[mcp-sync] WARNING: ${client.name} config verification failed: ${verifyErr.message}`,
          );
          // Retry write once
          fs.writeFileSync(client.configPath, configStr, "utf-8");
        }
      }

      synced++;
      syncedClients.push(client.name);
      outputChannel.appendLine(
        `[mcp-sync] configured ${client.name}: ${client.configPath}`,
      );
    } catch (err: any) {
      outputChannel.appendLine(
        `[mcp-sync] ${client.name} error: ${err.message}`,
      );
    }
  }

  return { synced, syncedClients };
}

async function runRepairEnvironment() {
  const binary = getBinaryPath();
  if (!binary) return;

  const scriptPath = path.join(
    path.dirname(binary),
    "..",
    "..",
    "scripts",
    "fix-onnx-runtime.ps1",
  );

  const terminal = vscode.window.createTerminal("OmniContext Repair");
  terminal.show();

  if (process.platform === "win32") {
    terminal.sendText(`pwsh -File "${scriptPath}"`);
  } else {
    vscode.window.showInformationMessage("Repair is only needed on Windows.");
  }
}

async function runCleanupIndexes() {
  const { purgeOrphanedIndexes, getOmniReposDir } =
    await import("./repoRegistry");
  const reposDir = getOmniReposDir();

  const answer = await vscode.window.showWarningMessage(
    `This will remove orphaned index folders from:\n${reposDir}\n\nOnly indexes tracked in the registry will be kept. Continue?`,
    "Clean Up",
    "Cancel",
  );

  if (answer !== "Clean Up") return;

  const result = purgeOrphanedIndexes();
  const msg = `Cleanup complete: ${result.removed.length} orphaned indexes removed, ${result.kept.length} kept.`;
  outputChannel.appendLine(`[cleanup] ${msg}`);

  if (result.errors.length > 0) {
    outputChannel.appendLine(`[cleanup] errors: ${result.errors.join(", ")}`);
  }

  vscode.window.showInformationMessage(msg);
}

async function runUpdateBinary() {
  const ctx = extensionContext;
  if (!ctx) {
    vscode.window.showErrorMessage(
      "OmniContext: Extension context unavailable for update.",
    );
    return;
  }

  try {
    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: "OmniContext Update",
        cancellable: false,
      },
      async (progress) => {
        bootstrapResult = await bootstrap(ctx, (status) => {
          progress.report({
            message: status.message,
            increment: status.progressPercent,
          });
          outputChannel.appendLine(
            `[update] ${status.phase}: ${status.message}`,
          );
        });
      },
    );

    // Restart daemon to pick up any updated binaries.
    stopDaemon();
    const config = vscode.workspace.getConfiguration("omnicontext");
    if (config.get<boolean>("autoStartDaemon", true)) {
      await startDaemon(true);
    }

    // Ensure MCP clients are aligned to updated binary path.
    await syncMcpSilent();

    vscode.window.showInformationMessage("OmniContext engine update complete.");
  } catch (err: any) {
    outputChannel.appendLine(`[update] failed: ${err.message}`);
    vscode.window.showErrorMessage(`OmniContext update failed: ${err.message}`);
  }
}

// ... (existing helper functions) ...

// ---------------------------------------------------------------------------
// Configuration watcher
// ---------------------------------------------------------------------------

function registerConfigurationWatcher(context: vscode.ExtensionContext): void {
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("omnicontext.prefetch")) {
        handlePrefetchConfigChange();
      }
    }),
  );
}

async function handlePrefetchConfigChange(): Promise<void> {
  const config = vscode.workspace.getConfiguration("omnicontext.prefetch");

  // Get configuration values
  const enabled = config.get<boolean>("enabled", true);
  const cacheSize = config.get<number>("cacheSize", 100);
  const cacheTtlSeconds = config.get<number>("cacheTtlSeconds", 300);
  const debounceMs = config.get<number>("debounceMs", 200);

  // Validate settings
  if (cacheSize < 10 || cacheSize > 1000) {
    vscode.window.showErrorMessage(
      `OmniContext: Invalid cache size ${cacheSize}. Must be between 10 and 1000.`,
    );
    return;
  }

  if (cacheTtlSeconds < 60 || cacheTtlSeconds > 3600) {
    vscode.window.showErrorMessage(
      `OmniContext: Invalid cache TTL ${cacheTtlSeconds}. Must be between 60 and 3600 seconds.`,
    );
    return;
  }

  if (debounceMs < 50 || debounceMs > 1000) {
    vscode.window.showErrorMessage(
      `OmniContext: Invalid debounce delay ${debounceMs}. Must be between 50 and 1000 milliseconds.`,
    );
    return;
  }

  // Update EventTracker
  if (eventTracker) {
    eventTracker.setEnabled(enabled);
    eventTracker.setDebounceMs(debounceMs);
    outputChannel.appendLine(
      `[config] Pre-fetch ${enabled ? "enabled" : "disabled"}, debounce: ${debounceMs}ms`,
    );
  }

  // Send config updates to daemon via IPC
  if (ipcClient) {
    try {
      await sendIpcRequest("update_config", {
        cache_size: cacheSize,
        cache_ttl_seconds: cacheTtlSeconds,
      });
      outputChannel.appendLine(
        `[config] Updated daemon cache: size=${cacheSize}, ttl=${cacheTtlSeconds}s`,
      );
      vscode.window.showInformationMessage(
        `OmniContext: Configuration updated successfully`,
      );
    } catch (err: any) {
      outputChannel.appendLine(
        `[config] Failed to update daemon: ${err.message}`,
      );
      vscode.window.showWarningMessage(
        `OmniContext: Failed to update daemon configuration. Changes will apply on next daemon restart.`,
      );
    }
  } else {
    outputChannel.appendLine(
      `[config] Daemon not connected, configuration will apply on next daemon start`,
    );
  }
}

// ---------------------------------------------------------------------------
// Binary resolution
// ---------------------------------------------------------------------------

function getBinaryPath(): string {
  // 1. User-configured explicit path always wins
  const config = vscode.workspace.getConfiguration("omnicontext");
  const configured = config.get<string>("binaryPath", "");
  if (configured && fs.existsSync(configured)) {
    return configured;
  }

  // 2. Bootstrap result (auto-downloaded or bundled) - highest confidence
  if (bootstrapResult && fs.existsSync(bootstrapResult.cliBinary)) {
    return bootstrapResult.cliBinary;
  }

  const home = process.env.HOME || process.env.USERPROFILE || "";
  const ext = process.platform === "win32" ? ".exe" : "";
  const binName = `omnicontext${ext}`;

  // 3. Ordered candidate list — checked with fs.existsSync (non-blocking)
  const candidates = [
    // Standalone install.ps1 / install.sh location
    path.join(home, ".omnicontext", "bin", binName),
    // Linux ~/.local/bin
    path.join(home, ".local", "bin", binName),
    // Developer cargo install
    path.join(home, ".cargo", "bin", binName),
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  // 4. Last resort: system PATH probe (synchronous but fast on PATH hits)
  try {
    cp.execSync(`omnicontext --version`, {
      stdio: "ignore",
      timeout: 2000,
    });
    return "omnicontext";
  } catch {
    // Not in PATH
  }

  outputChannel.appendLine(
    "[binary] omnicontext not found. Bootstrap should have resolved this.",
  );
  return "";
}

function getDaemonBinaryPath(): string {
  const binary = getBinaryPath();
  if (!binary) return "";

  // Try dedicated daemon binary first
  const daemonBinary = binary.replace(
    /omnicontext(\.exe)?$/,
    "omnicontext-daemon$1",
  );
  try {
    cp.execSync(`"${daemonBinary}" --help`, { stdio: "ignore" });
    return daemonBinary;
  } catch {
    // Daemon binary not available
    return "";
  }
}

function getWorkspaceRoot(): string {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) {
    vscode.window.showWarningMessage("No workspace folder open");
    return "";
  }

  // In multi-root workspaces, prefer the folder containing the active editor
  const activeEditor = vscode.window.activeTextEditor;
  if (activeEditor && folders.length > 1) {
    const activeFolder = vscode.workspace.getWorkspaceFolder(
      activeEditor.document.uri,
    );
    if (activeFolder) {
      return activeFolder.uri.fsPath;
    }
  }

  return folders[0].uri.fsPath;
}

// ---------------------------------------------------------------------------
// Daemon management
// ---------------------------------------------------------------------------

async function startDaemon(silent: boolean = false) {
  const daemonBinary = getDaemonBinaryPath();
  const root = getWorkspaceRoot();

  if (!root) return;

  // If no daemon binary, fall back to auto-index
  if (!daemonBinary) {
    if (!silent) {
      outputChannel.appendLine(
        "Daemon binary not found, falling back to CLI indexing",
      );
    }
    // Try CLI indexing — if that also fails, show an explicit error
    const binary = getBinaryPath();
    if (!binary) {
      statusBarItem.text = "$(error) OmniContext: Not Installed";
      statusBarItem.tooltip = "OmniContext binary not found. Click to search.";
      vscode.window
        .showErrorMessage(
          "OmniContext binary not found. Install it or configure omnicontext.binaryPath.",
          "Open Settings",
        )
        .then((choice) => {
          if (choice === "Open Settings") {
            vscode.commands.executeCommand(
              "workbench.action.openSettings",
              "omnicontext.binaryPath",
            );
          }
        });
      return;
    }
    runIndex(silent);
    return;
  }

  if (daemonProcess) {
    if (!silent) {
      vscode.window.showInformationMessage(
        "OmniContext daemon already running",
      );
    }
    return;
  }

  statusBarItem.text = "$(sync~spin) Starting daemon...";

  try {
    daemonProcess = cp.spawn(daemonBinary, ["--repo", root], {
      cwd: root,
      stdio: ["ignore", "pipe", "pipe"],
    });

    daemonProcess.stderr?.on("data", (data: Buffer) => {
      outputChannel.appendLine(`[daemon] ${data.toString().trim()}`);
    });

    let daemonExitedEarly = false;
    daemonProcess.on("exit", (code) => {
      outputChannel.appendLine(`[daemon] exited with code ${code}`);
      daemonExitedEarly = true;
      rejectAllPendingRequests("Daemon process exited");
      daemonProcess = null;
      ipcClient = null;
      isDaemonConnected = false;
      statusBarItem.text = "$(search) OmniContext";
    });

    // Poll for daemon readiness instead of a fixed delay.
    // The daemon needs time to create the named pipe before we can connect.
    const maxWaitMs = 10_000;
    const pollIntervalMs = 250;
    let connected = false;
    for (let waited = 0; waited < maxWaitMs; waited += pollIntervalMs) {
      if (daemonExitedEarly) {
        throw new Error(
          "Daemon exited before IPC connection could be established",
        );
      }
      try {
        await connectIpc(root);
        connected = true;
        break;
      } catch {
        // Pipe not ready yet — wait and retry
        await new Promise((r) => setTimeout(r, pollIntervalMs));
      }
    }
    if (!connected) {
      throw new Error("Daemon failed to become ready within 10 seconds");
    }

    statusBarItem.text = "$(zap) OmniContext";
    statusBarItem.tooltip = "OmniContext: Daemon active, context injection ON";

    // Auto-sync MCP to all detected AI clients
    syncMcpSilent();

    if (!silent) {
      vscode.window.showInformationMessage("OmniContext daemon started");
    }
  } catch (err: any) {
    statusBarItem.text = "$(error) OmniContext";
    if (!silent) {
      outputChannel.appendLine(`Daemon start error: ${err.message}`);
      vscode.window.showErrorMessage(
        `OmniContext daemon failed: ${err.message}`,
      );
    }
  }
}

function stopDaemon() {
  // Clear reconnection timer
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  reconnectAttempts = 0;
  currentRepoRoot = null;
  isDaemonConnected = false;
  stopHealthCheck();

  if (ipcClient) {
    try {
      sendIpcRequest("shutdown", {}).catch(() => {});
    } catch {
      // Ignore errors during shutdown
    }
    rejectAllPendingRequests("Daemon stopped");
    ipcClient.destroy();
    ipcClient = null;
  }

  if (daemonProcess) {
    daemonProcess.kill();
    daemonProcess = null;
  }

  statusBarItem.text = "$(search) OmniContext";
  statusBarItem.tooltip = "OmniContext: Daemon stopped";
}

// ---------------------------------------------------------------------------
// IPC client (named pipe / Unix socket)
// ---------------------------------------------------------------------------

async function connectIpc(repoRoot: string): Promise<void> {
  const pipeName = derivePipeName(repoRoot);
  outputChannel.appendLine(`[ipc] connecting to: ${pipeName}`);

  // Store repo root for reconnection
  currentRepoRoot = repoRoot;

  return new Promise((resolve, reject) => {
    const client = net.createConnection(pipeName);
    let buffer = "";
    let settled = false;
    const connectTimeout = setTimeout(() => {
      if (settled) return;
      settled = true;
      client.destroy();
      reject(new Error("IPC connection timeout"));
    }, 5000);

    client.on("connect", () => {
      if (settled) return;
      settled = true;
      clearTimeout(connectTimeout);
      outputChannel.appendLine("[ipc] connected");
      ipcClient = client;
      isDaemonConnected = true;

      // Reset reconnection attempts on successful connection
      reconnectAttempts = 0;
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }

      // Start periodic health-check pings
      startHealthCheck();

      resolve();
    });

    client.on("data", (data: Buffer) => {
      buffer += data.toString();
      const lines = buffer.split("\n");
      buffer = lines.pop() || "";

      for (const line of lines) {
        if (!line.trim()) continue;
        try {
          const response = JSON.parse(line);
          const pending = pendingRequests.get(response.id);
          if (pending) {
            pendingRequests.delete(response.id);
            if (response.error) {
              pending.reject(new Error(response.error.message));
            } else {
              pending.resolve(response.result);
            }
          }
        } catch (e: any) {
          outputChannel.appendLine(`[ipc] parse error: ${e.message}`);
        }
      }
    });

    client.on("error", (err) => {
      const shouldReject = !settled;
      if (shouldReject) {
        settled = true;
        clearTimeout(connectTimeout);
      }
      outputChannel.appendLine(`[ipc] error: ${err.message}`);
      rejectAllPendingRequests(`IPC error: ${err.message}`);
      ipcClient = null;
      isDaemonConnected = false;

      // Trigger reconnection on error
      scheduleReconnect();

      if (shouldReject) {
        reject(err);
      }
    });

    client.on("close", () => {
      const shouldReject = !settled;
      if (shouldReject) {
        settled = true;
      }
      clearTimeout(connectTimeout);
      outputChannel.appendLine("[ipc] disconnected");
      rejectAllPendingRequests("IPC connection closed");
      ipcClient = null;
      isDaemonConnected = false;

      // Trigger reconnection on close
      scheduleReconnect();

      if (shouldReject) {
        reject(new Error("IPC connection closed"));
      }
    });
  });
}

function scheduleReconnect(): void {
  // Don't schedule if already scheduled
  if (reconnectTimer) {
    return;
  }

  // Don't schedule if no repo root (never connected)
  if (!currentRepoRoot) {
    return;
  }

  // Give up after max attempts
  if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
    outputChannel.appendLine(
      `[ipc] max reconnection attempts (${MAX_RECONNECT_ATTEMPTS}) reached, giving up`,
    );
    statusBarItem.text = "$(error) OmniContext";
    statusBarItem.tooltip = "OmniContext: Connection failed";
    return;
  }

  // Exponential backoff: 1s, 2s, 4s, 8s, 16s, 32s (capped at 30s)
  const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
  reconnectAttempts++;

  outputChannel.appendLine(
    `[ipc] scheduling reconnection attempt ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS} in ${delay}ms`,
  );

  reconnectTimer = setTimeout(async () => {
    reconnectTimer = null;

    outputChannel.appendLine(
      `[ipc] attempting reconnection (attempt ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})`,
    );

    try {
      await connectIpc(currentRepoRoot!);
      outputChannel.appendLine("[ipc] reconnection successful");
      statusBarItem.text = "$(zap) OmniContext";
      statusBarItem.tooltip =
        "OmniContext: Daemon active, context injection ON";
    } catch (err: any) {
      outputChannel.appendLine(`[ipc] reconnection failed: ${err.message}`);
      // scheduleReconnect will be called by connectIpc's error handler
    }
  }, delay);
}

// ---------------------------------------------------------------------------
// Health check: periodic ping to detect a silently-dead daemon.
// ---------------------------------------------------------------------------
function startHealthCheck(): void {
  stopHealthCheck();
  consecutiveHealthFailures = 0;

  healthCheckInterval = setInterval(async () => {
    if (!isDaemonConnected) return;

    // Don't treat long-running index/history calls as daemon health failures.
    if (hasLongRunningRequestInFlight()) {
      consecutiveHealthFailures = 0;
      return;
    }

    try {
      await sendIpcRequest("system_status", {});
      consecutiveHealthFailures = 0;
    } catch {
      consecutiveHealthFailures++;
      outputChannel.appendLine(
        `[health] ping failed (${consecutiveHealthFailures}/${MAX_HEALTH_FAILURES})`,
      );

      if (consecutiveHealthFailures >= MAX_HEALTH_FAILURES) {
        outputChannel.appendLine("[health] daemon unresponsive, restarting...");
        stopHealthCheck();
        await stopDaemon();
        await startDaemon(true);
      }
    }
  }, HEALTH_CHECK_INTERVAL_MS);
}

function stopHealthCheck(): void {
  if (healthCheckInterval) {
    clearInterval(healthCheckInterval);
    healthCheckInterval = null;
  }
  consecutiveHealthFailures = 0;
}

function getIpcTimeoutMs(method: string): number {
  switch (method) {
    case "index":
    case "history/index_commits":
      // Full indexing and commit-history indexing can take minutes on large repos.
      return 10 * 60 * 1000;
    case "clear_index":
      return 60 * 1000;
    case "preflight":
      return 15 * 1000;
    case "search":
    case "context_window":
      return 20 * 1000;
    default:
      return 5 * 1000;
  }
}

function hasLongRunningRequestInFlight(): boolean {
  for (const pending of pendingRequests.values()) {
    if (
      pending.method === "index" ||
      pending.method === "history/index_commits"
    ) {
      return true;
    }
  }
  return false;
}

function rejectAllPendingRequests(reason: string): void {
  for (const [id, pending] of pendingRequests.entries()) {
    pending.reject(new Error(reason));
    pendingRequests.delete(id);
  }
}

function sendIpcRequest(method: string, params: any): Promise<any> {
  return new Promise((resolve, reject) => {
    if (!ipcClient) {
      reject(new Error("IPC not connected"));
      return;
    }

    const id = ++requestCounter;
    const request = {
      jsonrpc: "2.0",
      id,
      method,
      params,
    };

    pendingRequests.set(id, {
      method,
      startedAt: Date.now(),
      resolve,
      reject,
    });

    const payload = JSON.stringify(request) + "\n";

    // Handle write errors
    const writeSuccess = ipcClient.write(payload, (err) => {
      if (err) {
        outputChannel.appendLine(`[ipc] write error: ${err.message}`);
        pendingRequests.delete(id);
        ipcClient = null;
        scheduleReconnect();
        reject(err);
      }
    });

    if (!writeSuccess) {
      outputChannel.appendLine("[ipc] write buffer full");
      pendingRequests.delete(id);
      reject(new Error("IPC write buffer full"));
    }

    const timeoutMs = getIpcTimeoutMs(method);
    setTimeout(() => {
      if (pendingRequests.has(id)) {
        pendingRequests.delete(id);
        reject(new Error(`IPC request timeout: ${method} (${timeoutMs}ms)`));
      }
    }, timeoutMs);
  });
}

function derivePipeName(repoRoot: string): string {
  return computePipeName(repoRoot);
}

// ---------------------------------------------------------------------------
// Chat participant -- pre-flight context injection
// ---------------------------------------------------------------------------

function registerChatParticipant(context: vscode.ExtensionContext) {
  // Register a chat participant that silently injects context
  // This works with VS Code's built-in Copilot chat
  try {
    const participant = vscode.chat.createChatParticipant(
      "omnicontext.context",
      async (
        request: vscode.ChatRequest,
        _chatContext: vscode.ChatContext,
        stream: vscode.ChatResponseStream,
        token: vscode.CancellationToken,
      ) => {
        if (token.isCancellationRequested) return;

        const contextResult = await getPreflightContext(request.prompt);

        if (contextResult) {
          const cacheIndicator = contextResult.from_cache
            ? "[cached]"
            : "[fresh search]";
          stream.markdown(
            `*OmniContext injected ${contextResult.entries_count} code chunks ` +
              `(${contextResult.tokens_used}/${contextResult.token_budget} tokens, ` +
              `${contextResult.elapsed_ms}ms ${cacheIndicator})*\n\n`,
          );
          stream.markdown(contextResult.system_context);
        } else {
          stream.markdown(
            "*OmniContext: could not retrieve context (daemon not running)*\n\n",
          );
        }
      },
    );

    participant.iconPath = new vscode.ThemeIcon("search");
    context.subscriptions.push(participant);
  } catch {
    // Chat API might not be available in all VS Code versions
    outputChannel.appendLine(
      "[info] Chat participant API not available, skipping registration",
    );
  }
}

async function getPreflightContext(
  prompt: string,
): Promise<PreflightResponse | null> {
  // Try IPC first (daemon), then fall back to CLI
  if (ipcClient && contextInjectionEnabled) {
    try {
      const activeEditor = vscode.window.activeTextEditor;
      const activeFile = activeEditor?.document.uri.fsPath;
      const cursorLine = activeEditor?.selection.active.line;
      const openFiles = vscode.window.visibleTextEditors.map(
        (e) => e.document.uri.fsPath,
      );

      const config = vscode.workspace.getConfiguration("omnicontext");
      const tokenBudget = config.get<number>("tokenBudget", 8192);

      const result = await sendIpcRequest("preflight", {
        prompt,
        active_file: activeFile,
        cursor_line: cursorLine,
        open_files: openFiles,
        token_budget: tokenBudget,
      });

      const response = result as PreflightResponse;

      // Add cache indicators and logging
      if (response.from_cache) {
        const estimatedTimeSaved = Math.max(0, 300 - response.elapsed_ms);
        outputChannel.appendLine(
          `[preflight] Cache HIT: ${response.elapsed_ms}ms (saved ~${estimatedTimeSaved}ms)`,
        );
      } else {
        outputChannel.appendLine(
          `[preflight] Cache MISS: ${response.elapsed_ms}ms (fresh search)`,
        );
      }

      response.system_context = formatPreflightContext(
        response.system_context,
        response.elapsed_ms,
        !!response.from_cache,
      );

      return response;
    } catch (err: any) {
      outputChannel.appendLine(`[preflight] IPC error: ${err.message}`);
    }
  }

  // Fallback: use CLI for context_window
  return getCliContext(prompt);
}

function getCliContext(prompt: string): PreflightResponse | null {
  const binary = getBinaryPath();
  const root = getWorkspaceRoot();
  if (!binary || !root) return null;

  try {
    const config = vscode.workspace.getConfiguration("omnicontext");
    const tokenBudget = config.get<number>("tokenBudget", 8192);

    const result = cp.execFileSync(
      binary,
      ["search", prompt, "--json", "--limit", "20"],
      { encoding: "utf-8", timeout: 10000, cwd: root },
    );

    const data = JSON.parse(result);
    const assembled = assembleCliContext(
      data.results || [],
      tokenBudget,
      data.elapsed_ms || 0,
    );
    if (!assembled) return null;

    return {
      system_context: assembled.system_context,
      entries_count: assembled.entries_count,
      tokens_used: assembled.tokens_used,
      token_budget: assembled.token_budget,
      elapsed_ms: assembled.elapsed_ms,
      from_cache: false,
    };
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

async function runIndex(silent: boolean = false) {
  // Try daemon IPC first
  if (ipcClient) {
    if (!silent) statusBarItem.text = "$(sync~spin) Indexing...";
    try {
      const result = await sendIpcRequest("index", {});
      statusBarItem.text = `$(zap) OmniContext (${result.files_processed} files)`;

      // Record in repo registry for sidebar tracking
      const root = getWorkspaceRoot();
      if (root) {
        registerRepo(root, result.files_processed, result.chunks_created);
      }

      if (!silent) {
        const failed = result.files_failed ?? 0;
        const failedText = failed > 0 ? `, ${failed} failed` : "";
        const embeddingFailures = result.embedding_failures ?? 0;
        const embeddingFailureText =
          embeddingFailures > 0
            ? `, ${embeddingFailures} embedding flush error(s)`
            : "";
        vscode.window.showInformationMessage(
          `OmniContext: Indexed ${result.files_processed} files, ` +
            `${result.chunks_created} chunks, ${result.embeddings_generated ?? 0} embeddings${failedText}${embeddingFailureText} in ${result.elapsed_ms}ms`,
        );
      }
      return;
    } catch (err: any) {
      outputChannel.appendLine(
        `[index] IPC error, falling back to CLI: ${err.message}`,
      );
    }
  }

  // Fallback to CLI
  const binary = getBinaryPath();
  const root = getWorkspaceRoot();
  if (!binary || !root) {
    if (!binary) {
      vscode.window
        .showErrorMessage(
          "OmniContext: Cannot index — daemon binary not found. Check the omnicontext.binaryPath setting.",
          "Open Settings",
        )
        .then((action) => {
          if (action === "Open Settings") {
            vscode.commands.executeCommand(
              "workbench.action.openSettings",
              "omnicontext.binaryPath",
            );
          }
        });
    }
    if (!root) {
      vscode.window.showWarningMessage(
        "OmniContext: No workspace folder open.",
      );
    }
    return;
  }

  if (!silent) {
    statusBarItem.text = "$(sync~spin) Indexing...";
  }

  try {
    const result = cp.execFileSync(binary, ["index", root, "--json"], {
      encoding: "utf-8",
      timeout: 300_000,
      cwd: root,
    });

    const data = JSON.parse(result);
    statusBarItem.text = `$(search) OmniContext (${data.files_processed} files)`;

    // Record in repo registry for sidebar tracking
    registerRepo(root, data.files_processed, data.chunks_created);

    if (!silent) {
      const failed = data.files_failed ?? 0;
      const failedText = failed > 0 ? `, ${failed} failed` : "";
      const embeddingFailures = data.embedding_failures ?? 0;
      const embeddingFailureText =
        embeddingFailures > 0
          ? `, ${embeddingFailures} embedding flush error(s)`
          : "";
      vscode.window.showInformationMessage(
        `OmniContext: Indexed ${data.files_processed} files, ` +
          `${data.chunks_created} chunks, ${data.symbols_extracted} symbols, ` +
          `${data.embeddings_generated ?? 0} embeddings${failedText} ` +
          `${embeddingFailureText} in ${data.elapsed_ms}ms`,
      );
    }
  } catch (err: any) {
    statusBarItem.text = "$(error) OmniContext";
    if (!silent) {
      outputChannel.appendLine(`Index error: ${err.message}`);
      vscode.window.showErrorMessage(
        `OmniContext index failed: ${err.message}`,
      );
    }
  }
}

async function runSearch() {
  const root = getWorkspaceRoot();
  if (!root) return;

  const query = await vscode.window.showInputBox({
    prompt: "Search your codebase",
    placeHolder:
      "e.g. authentication handler, Config::new, how does caching work?",
  });

  if (!query) return;

  try {
    let data: any;

    if (ipcClient) {
      // Use daemon IPC
      data = await sendIpcRequest("search", { query, limit: 20 });
    } else {
      // Fallback to CLI
      const binary = getBinaryPath();
      if (!binary) return;

      const result = cp.execFileSync(
        binary,
        ["search", query, "--json", "--limit", "20"],
        { encoding: "utf-8", timeout: 30_000, cwd: root },
      );
      data = JSON.parse(result);
    }

    if (!data.results || data.results.length === 0) {
      vscode.window.showInformationMessage(`No results for "${query}"`);
      return;
    }

    const items = data.results.map((r: any, i: number) => ({
      label: `${i + 1}. ${r.symbol}`,
      description: `${r.kind} | score: ${r.score.toFixed(4)}`,
      detail: `${r.file}:${r.line_start}-${r.line_end}`,
      file: r.file,
      line: r.line_start,
    }));

    const selected = (await vscode.window.showQuickPick(items, {
      placeHolder: `${data.results.length} results for "${query}"`,
      matchOnDescription: true,
      matchOnDetail: true,
    })) as any;

    if (selected) {
      const uri = vscode.Uri.file(path.join(root, selected.file));
      const doc = await vscode.workspace.openTextDocument(uri);
      const editor = await vscode.window.showTextDocument(doc);
      const line = Math.max(0, selected.line - 1);
      editor.revealRange(
        new vscode.Range(line, 0, line + 5, 0),
        vscode.TextEditorRevealType.InCenter,
      );
      editor.selection = new vscode.Selection(line, 0, line, 0);
    }
  } catch (err: any) {
    outputChannel.appendLine(`Search error: ${err.message}`);
    vscode.window.showErrorMessage(`Search failed: ${err.message}`);
  }
}

async function runStatus() {
  const root = getWorkspaceRoot();
  if (!root) return;

  try {
    let data: any;

    if (ipcClient) {
      data = await sendIpcRequest("status", {});
    } else {
      const binary = getBinaryPath();
      if (!binary) return;

      const result = cp.execFileSync(binary, ["status", root, "--json"], {
        encoding: "utf-8",
        timeout: 10_000,
        cwd: root,
      });
      data = JSON.parse(result);
    }

    outputChannel.clear();
    outputChannel.appendLine("OmniContext Status");
    outputChannel.appendLine("---");
    outputChannel.appendLine(`Repository:      ${data.repo_path}`);
    outputChannel.appendLine(`Search mode:     ${data.search_mode}`);
    outputChannel.appendLine(`Files indexed:   ${data.files_indexed}`);
    outputChannel.appendLine(`Chunks indexed:  ${data.chunks_indexed}`);
    outputChannel.appendLine(`Symbols indexed: ${data.symbols_indexed}`);
    outputChannel.appendLine(`Vectors indexed: ${data.vectors_indexed}`);
    outputChannel.appendLine(`Dep edges:       ${data.dep_edges}`);
    outputChannel.appendLine(`Graph nodes:     ${data.graph_nodes}`);
    outputChannel.appendLine(`Graph edges:     ${data.graph_edges}`);
    outputChannel.appendLine(
      `Daemon:          ${ipcClient ? "CONNECTED" : "NOT CONNECTED"}`,
    );
    outputChannel.appendLine(
      `Injection:       ${contextInjectionEnabled ? "ON" : "OFF"}`,
    );
    if (data.has_cycles) {
      outputChannel.appendLine("[!] Circular dependencies detected");
    }
    outputChannel.show();
  } catch (err: any) {
    vscode.window.showErrorMessage(`Status failed: ${err.message}`);
  }
}

async function startMcp() {
  const binary = getBinaryPath();
  const root = getWorkspaceRoot();
  if (!binary || !root) {
    if (!binary) {
      vscode.window
        .showErrorMessage(
          "OmniContext: Cannot start MCP — daemon binary not found. Check the omnicontext.binaryPath setting.",
          "Open Settings",
        )
        .then((action) => {
          if (action === "Open Settings") {
            vscode.commands.executeCommand(
              "workbench.action.openSettings",
              "omnicontext.binaryPath",
            );
          }
        });
    }
    if (!root) {
      vscode.window.showWarningMessage(
        "OmniContext: No workspace folder open.",
      );
    }
    return;
  }

  const mcpBinary = deriveMcpBinaryPath(binary);

  const terminal = vscode.window.createTerminal({
    name: "OmniContext MCP",
    shellPath: mcpBinary,
    shellArgs: ["--repo", root],
    cwd: root,
  });

  terminal.show();
  vscode.window.showInformationMessage("OmniContext MCP server started");
}

function toggleContextInjection() {
  contextInjectionEnabled = !contextInjectionEnabled;
  const state = contextInjectionEnabled ? "ON" : "OFF";
  statusBarItem.tooltip = `OmniContext: Context injection ${state}`;

  // Enable/disable event tracking along with context injection
  if (eventTracker) {
    eventTracker.setEnabled(contextInjectionEnabled);
  }

  vscode.window.showInformationMessage(
    `OmniContext: Context injection ${state}`,
  );
}

async function runPreflight() {
  const query = await vscode.window.showInputBox({
    prompt: "Enter prompt for pre-flight context",
    placeHolder: "e.g. Fix the authentication middleware",
  });

  if (!query) return;

  const start = Date.now();
  const context = await getPreflightContext(query);

  if (context) {
    outputChannel.clear();
    outputChannel.appendLine("=== Pre-Flight Context ===");
    outputChannel.appendLine(`Entries: ${context.entries_count}`);
    outputChannel.appendLine(
      `Tokens: ${context.tokens_used}/${context.token_budget}`,
    );
    outputChannel.appendLine(`Time: ${Date.now() - start}ms`);
    outputChannel.appendLine("---");
    outputChannel.appendLine(context.system_context);
    outputChannel.show();
  } else {
    vscode.window.showWarningMessage(
      "No context available. Is the daemon running?",
    );
  }
}

async function runModuleMap() {
  if (!ipcClient) {
    vscode.window.showWarningMessage(
      "Module map requires the daemon. Run 'OmniContext: Start Daemon' first.",
    );
    return;
  }

  try {
    const data = await sendIpcRequest("module_map", { max_depth: 3 });

    outputChannel.clear();
    outputChannel.appendLine("=== Module Map ===");
    outputChannel.appendLine(
      `Modules: ${data.module_count} | Files: ${data.file_count}`,
    );
    outputChannel.appendLine("---");

    for (const [modulePath, files] of Object.entries(data.modules)) {
      outputChannel.appendLine(`\n${modulePath}/`);
      for (const file of files as any[]) {
        const symbols = file.symbols.join(", ");
        outputChannel.appendLine(
          `  ${path.basename(file.file)} [${file.language}] ${symbols ? "-- " + symbols : ""}`,
        );
      }
    }

    outputChannel.show();
  } catch (err: any) {
    vscode.window.showErrorMessage(`Module map failed: ${err.message}`);
  }
}
