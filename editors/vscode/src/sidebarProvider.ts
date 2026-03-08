/**
 * Webview provider for OmniContext sidebar.
 * Provides comprehensive system status, metrics, and controls.
 */

import * as vscode from "vscode";
import { CacheStatsManager } from "./cacheStats";
import { EventTracker } from "./eventTracker";
import { BootstrapStatus } from "./bootstrapService";
import {
  getIndexedRepos,
  registerRepo,
  unregisterRepo,
  IndexedRepo,
  hasIndexOnDisk,
  getOmniReposDir,
  discoverReposFromDisk,
} from "./repoRegistry";
import * as fs from "fs";
import * as path from "path";

interface SystemStatus {
  initialization_status: "initializing" | "ready" | "error";
  connection_health: "connected" | "disconnected" | "reconnecting";
  last_index_time?: number;
  daemon_uptime_seconds: number;
  files_indexed: number;
  chunks_indexed: number;
}

interface PerformanceMetrics {
  search_latency_p50_ms: number;
  search_latency_p95_ms: number;
  search_latency_p99_ms: number;
  embedding_coverage_percent: number;
  memory_usage_bytes: number;
  peak_memory_usage_bytes: number;
  total_searches: number;
}

interface ActivityLogEntry {
  type: string;
  status: "success" | "error" | "warning" | "info";
  time: string;
  details: string;
  timestamp: number;
}

export class OmniSidebarProvider implements vscode.WebviewViewProvider {
  private _view?: vscode.WebviewView;
  private activityLog: ActivityLogEntry[] = [];
  private readonly maxActivityEntries = 10;
  /** Guards against rapid-fire refresh() calls (tab switches, events). */
  private _refreshTimer: ReturnType<typeof setTimeout> | null = null;
  private _refreshPending = false;

  constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly cacheStatsManager: CacheStatsManager,
    private readonly eventTracker: EventTracker,
    private readonly sendIpcRequest: (
      method: string,
      params: any,
    ) => Promise<any>,
    private readonly isDaemonConnected: () => boolean = () => false,
  ) {}

  /**
   * Send a bootstrap status update directly to the sidebar webview.
   * Called by extension.ts during the bootstrap phase before daemon starts.
   */
  public sendBootstrapStatus(status: BootstrapStatus): void {
    if (!this._view) {
      return;
    }
    this._view.webview.postMessage({
      type: "bootstrapStatus",
      phase: status.phase,
      message: status.message,
      progressPercent: status.progressPercent,
    });
  }

  /**
   * Resolve the webview view.
   */
  public async resolveWebviewView(
    webviewView: vscode.WebviewView,
    context: vscode.WebviewViewResolveContext,
    token: vscode.CancellationToken,
  ): Promise<void> {
    this._view = webviewView;

    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [this.extensionUri],
    };

    webviewView.webview.html = this.getHtmlForWebview(webviewView.webview);

    // Handle messages from webview
    webviewView.webview.onDidReceiveMessage(async (message) => {
      await this.handleWebviewMessage(message);
    });

    // Automatically update the "Active" repository tracking badge when switching tabs.
    // Throttled to max once per 500ms so rapid tab-cycling doesn't saturate the CPU.
    vscode.window.onDidChangeActiveTextEditor(() => {
      if (this._view && this._view.visible) {
        this.scheduleRefresh();
      }
    });

    // Initial refresh
    await this.refresh();
  }

  /**
   * Schedule a debounced refresh. Prevents back-to-back refresh() calls from
   * saturating the CPU when you rapidly switch tabs or VS Code fires multiple
   * editor-change events.
   */
  private scheduleRefresh(): void {
    if (this._refreshTimer) {
      this._refreshPending = true;
      return; // A refresh is already scheduled
    }
    this._refreshTimer = setTimeout(async () => {
      this._refreshTimer = null;
      await this.refresh();
      // If another refresh was requested while we were running, do one more
      if (this._refreshPending) {
        this._refreshPending = false;
        this.scheduleRefresh();
      }
    }, 500);
  }

  /**
   * Refresh the sidebar with latest data.
   */
  public async refresh(): Promise<void> {
    if (!this._view) {
      return;
    }

    const isConnected = this.isDaemonConnected();

    if (!isConnected) {
      this._view.webview.postMessage({ type: "daemonOffline" });
      // We do NOT return early here, because we still need to send the repo list,
      // version info, and settings to the UI even if the backend is down.
    }

    // Retrieve cache statistics
    const cacheStats = isConnected
      ? await this.cacheStatsManager.getStats()
      : null;

    // Retrieve system status
    const systemStatus = isConnected ? await this.getSystemStatus() : null;

    // Retrieve performance metrics
    const performanceMetrics = isConnected
      ? await this.getPerformanceMetrics()
      : null;

    // Get prefetch enabled state from configuration
    const config = vscode.workspace.getConfiguration("omnicontext.prefetch");
    const prefetchEnabled = config.get<boolean>("enabled", true);

    // Determine cache status
    let cacheStatus: "active" | "disabled" | "offline";
    let cacheStatusText: string;

    if (!cacheStats) {
      cacheStatus = "offline";
      cacheStatusText = "Offline";
    } else if (!prefetchEnabled) {
      cacheStatus = "disabled";
      cacheStatusText = "Disabled";
    } else {
      cacheStatus = "active";
      cacheStatusText = "Active";
    }

    // Format cache statistics
    const hitRate = cacheStats
      ? `${(cacheStats.hit_rate * 100).toFixed(1)}%`
      : "0%";
    const hits = cacheStats ? cacheStats.hits.toString() : "0";
    const misses = cacheStats ? cacheStats.misses.toString() : "0";
    const cacheSize = cacheStats
      ? `${cacheStats.size}/${cacheStats.capacity}`
      : "0/100";

    this._view.webview.postMessage({
      type: "updateCacheStats",
      data: {
        status: cacheStatus,
        statusText: cacheStatusText,
        hitRate,
        hits,
        misses,
        cacheSize,
        prefetchEnabled,
      },
    });

    // Send repository info
    let currentRepoPath = "";

    // 1. Prioritize the folder of the currently focused document
    const activeEditor = vscode.window.activeTextEditor;
    if (activeEditor) {
      const activeFolder = vscode.workspace.getWorkspaceFolder(
        activeEditor.document.uri,
      );
      if (activeFolder) {
        currentRepoPath = activeFolder.uri.fsPath;
      }
    }

    // 2. Fallback to the first workspace folder if no document is focused
    if (
      !currentRepoPath &&
      vscode.workspace.workspaceFolders &&
      vscode.workspace.workspaceFolders.length > 0
    ) {
      currentRepoPath = vscode.workspace.workspaceFolders[0].uri.fsPath;
    }

    if (currentRepoPath) {
      this._view.webview.postMessage({
        type: "updateRepositoryInfo",
        repoPath: currentRepoPath,
      });
    }

    // Send system status update and auto-repair registry properties if connected
    if (systemStatus) {
      this._view.webview.postMessage({
        type: "updateSystemStatus",
        status: systemStatus,
      });

      // If we are connected and indexing has occurred, self-heal the registry
      if (
        systemStatus.connection_health === "connected" &&
        systemStatus.files_indexed > 0
      ) {
        if (currentRepoPath) {
          // registerRepo will update files_indexed, chunks_indexed, and last_indexed_at
          registerRepo(
            currentRepoPath,
            systemStatus.files_indexed,
            systemStatus.chunks_indexed,
          );
        }
      }
    }

    // Send performance metrics update
    if (performanceMetrics) {
      this._view.webview.postMessage({
        type: "updatePerformanceMetrics",
        metrics: performanceMetrics,
      });
    }

    // Auto-discover repos indexed via CLI that aren't in registry.json yet.
    // We pass all known workspace folder paths so hashes can be resolved to names.
    const allWorkspacePaths = (vscode.workspace.workspaceFolders || []).map(
      (f) => f.uri.fsPath,
    );
    discoverReposFromDisk(allWorkspacePaths);

    // Send indexed repos registry
    const indexedRepos = getIndexedRepos();
    this._view.webview.postMessage({
      type: "updateIndexedRepos",
      repos: indexedRepos,
      activeRepoPath: currentRepoPath,
    });

    // Send automation settings
    const omniConfig = vscode.workspace.getConfiguration("omnicontext");
    const automationConfig = vscode.workspace.getConfiguration(
      "omnicontext.automation",
    );
    this._view.webview.postMessage({
      type: "updateAutomationSettings",
      settings: {
        autoIndex: omniConfig.get<boolean>("autoIndex", true),
        autoStartDaemon: omniConfig.get<boolean>("autoStartDaemon", true),
        autoSyncMcp: automationConfig.get<boolean>("autoSyncMcp", false),
      },
    });

    // Send activity log
    this._view.webview.postMessage({
      type: "updateActivityLog",
      activities: this.activityLog.slice(-this.maxActivityEntries),
    });

    // Send system info -- including IDE identity for dynamic sync buttons.
    // Use the extension context to find ourselves, regardless of publisher ID.
    let extensionVersion = "unknown";
    for (const ext of vscode.extensions.all) {
      if (ext.packageJSON?.name === "omnicontext") {
        extensionVersion = ext.packageJSON.version;
        break;
      }
    }
    this._view.webview.postMessage({
      type: "updateSystemInfo",
      info: {
        version: extensionVersion,
        platform: `${process.platform} ${process.arch}`,
        ideName: vscode.env.appName,
        ideVersion: vscode.version,
      },
    });
  }

  /**
   * Get system status from daemon.
   */
  private async getSystemStatus(): Promise<SystemStatus | null> {
    try {
      const result = await this.sendIpcRequest("system_status", {});
      return result as SystemStatus;
    } catch (err) {
      return null;
    }
  }

  /**
   * Get performance metrics from daemon.
   */
  private async getPerformanceMetrics(): Promise<PerformanceMetrics | null> {
    try {
      const result = await this.sendIpcRequest("performance_metrics", {});
      return result as PerformanceMetrics;
    } catch (err) {
      return null;
    }
  }

  /**
   * Handle messages from the webview.
   */
  private async handleWebviewMessage(message: any): Promise<void> {
    switch (message.command) {
      case "refreshStatus":
        await this.refresh();
        break;

      case "clearCache":
        await this.handleClearCache();
        break;

      case "togglePrefetch":
        await this.handleTogglePrefetch(message.enabled);
        break;

      case "reindexRepository":
        await this.handleReindexRepository();
        break;

      case "clearIndex":
        await this.handleClearIndex();
        break;

      case "toggleAutoIndex":
        await this.handleToggleAutoIndex(message.enabled);
        break;

      case "toggleAutoDaemon":
        await this.handleToggleAutoDaemon(message.enabled);
        break;

      case "toggleAutoSync":
        await this.handleToggleAutoSync(message.enabled);
        break;

      case "quickSearch":
        await this.handleQuickSearch();
        break;

      case "clearActivityLog":
        await this.handleClearActivityLog();
        break;

      case "copyDiagnostics":
        await this.handleCopyDiagnostics();
        break;

      case "openLogs":
        await this.handleOpenLogs();
        break;

      case "viewActivityDetails":
        await this.handleViewActivityDetails(message.index);
        break;

      case "removeIndexedRepo":
        await this.handleRemoveIndexedRepo(message.hash);
        break;

      case "cleanupOrphans":
        vscode.commands.executeCommand("omnicontext.cleanupIndexes");
        break;

      case "updateBinary":
        vscode.commands.executeCommand("omnicontext.repair");
        break;

      case "syncMcpConfig":
        vscode.commands.executeCommand("omnicontext.syncMcp");
        break;

      default:
        console.warn("Unknown webview message:", message);
    }
  }

  /**
   * Handle clear cache request.
   */
  private async handleClearCache(): Promise<void> {
    try {
      await this.cacheStatsManager.clearCache();
      vscode.window.showInformationMessage("Cache cleared successfully");
      this.logActivity("Clear Cache", "success", "Pre-fetch cache cleared");
      await this.refresh();
    } catch (err) {
      vscode.window.showErrorMessage(`Failed to clear cache: ${err}`);
      this.logActivity("Clear Cache", "error", `Failed: ${err}`);
    }
  }

  /**
   * Handle toggle prefetch request.
   */
  private async handleTogglePrefetch(enabled: boolean): Promise<void> {
    const config = vscode.workspace.getConfiguration("omnicontext.prefetch");
    await config.update(
      "enabled",
      enabled,
      vscode.ConfigurationTarget.Workspace,
    );
    this.eventTracker.setEnabled(enabled);
    await this.refresh();
  }

  /**
   * Handle re-index repository request.
   */
  private async handleReindexRepository(): Promise<void> {
    try {
      this.logActivity("Re-index", "info", "Starting repository re-index...");

      // Show progress notification
      await vscode.window.withProgress(
        {
          location: vscode.ProgressLocation.Notification,
          title: "Re-indexing repository...",
          cancellable: false,
        },
        async (progress) => {
          progress.report({ increment: 0, message: "Starting indexing..." });

          // Trigger re-index via IPC
          const result = await this.sendIpcRequest("index", {});

          progress.report({ increment: 100, message: "Complete!" });

          // Register in repo registry
          const workspaceFolders = vscode.workspace.workspaceFolders;
          if (workspaceFolders && workspaceFolders.length > 0) {
            registerRepo(
              workspaceFolders[0].uri.fsPath,
              result.files_processed,
              result.chunks_created,
            );
          }

          const message = `Re-indexed ${result.files_processed} files, ${result.chunks_created} chunks in ${result.elapsed_ms}ms`;
          vscode.window.showInformationMessage(message);
          this.logActivity("Re-index", "success", message);
        },
      );

      await this.refresh();
    } catch (err: any) {
      vscode.window.showErrorMessage(`Failed to re-index: ${err.message}`);
      this.logActivity("Re-index", "error", `Failed: ${err.message}`);
    }
  }

  /**
   * Handle clear index request.
   */
  private async handleClearIndex(): Promise<void> {
    try {
      // Clear the index by sending a clear_index IPC request
      await this.sendIpcRequest("clear_index", {});
      vscode.window.showInformationMessage(
        "Index cleared successfully. Re-indexing recommended.",
      );
      this.logActivity(
        "Clear Index",
        "warning",
        "Index cleared - re-indexing recommended",
      );
      await this.refresh();
    } catch (err: any) {
      vscode.window.showErrorMessage(`Failed to clear index: ${err.message}`);
      this.logActivity("Clear Index", "error", `Failed: ${err.message}`);
    }
  }

  /**
   * Handle toggle auto-index request.
   */
  private async handleToggleAutoIndex(enabled: boolean): Promise<void> {
    const config = vscode.workspace.getConfiguration("omnicontext");
    await config.update(
      "autoIndex",
      enabled,
      vscode.ConfigurationTarget.Global,
    );
    vscode.window.showInformationMessage(
      `Auto-index ${enabled ? "enabled" : "disabled"}`,
    );
  }

  /**
   * Handle toggle auto-daemon request.
   */
  private async handleToggleAutoDaemon(enabled: boolean): Promise<void> {
    const config = vscode.workspace.getConfiguration("omnicontext");
    await config.update(
      "autoStartDaemon",
      enabled,
      vscode.ConfigurationTarget.Global,
    );
    vscode.window.showInformationMessage(
      `Auto-start daemon ${enabled ? "enabled" : "disabled"}`,
    );
  }

  /**
   * Handle toggle auto-sync request.
   */
  private async handleToggleAutoSync(enabled: boolean): Promise<void> {
    const config = vscode.workspace.getConfiguration("omnicontext.automation");
    await config.update(
      "autoSyncMcp",
      enabled,
      vscode.ConfigurationTarget.Global,
    );
    vscode.window.showInformationMessage(
      `Auto-sync MCP ${enabled ? "enabled" : "disabled"}`,
    );
  }

  /**
   * Handle quick search request.
   */
  private async handleQuickSearch(): Promise<void> {
    // Trigger the search command
    vscode.commands.executeCommand("omnicontext.search");
  }

  /**
   * Log an activity to the activity log.
   */
  public logActivity(
    type: string,
    status: "success" | "error" | "warning" | "info",
    details: string,
  ): void {
    const now = Date.now();
    const timeAgo = this.formatTimeAgo(now);

    this.activityLog.push({
      type,
      status,
      time: timeAgo,
      details,
      timestamp: now,
    });

    // Keep only last 100 entries
    if (this.activityLog.length > 100) {
      this.activityLog = this.activityLog.slice(-100);
    }

    // Update webview if visible
    if (this._view) {
      this._view.webview.postMessage({
        type: "updateActivityLog",
        activities: this.activityLog.slice(-this.maxActivityEntries),
      });
    }
  }

  /**
   * Handle remove indexed repo request.
   */
  private async handleRemoveIndexedRepo(hash: string): Promise<void> {
    if (!hash) return;

    const answer = await vscode.window.showWarningMessage(
      "Remove this repository from the OmniContext registry?",
      {
        modal: true,
        detail:
          "Do you also want to delete all generated index data from disk to free up space?",
      },
      "Yes, delete data",
      "No, keep data",
      "Cancel",
    );

    if (answer === "Cancel" || !answer) {
      return;
    }

    if (answer === "Yes, delete data") {
      try {
        const repoDir = path.join(getOmniReposDir(), hash);
        if (fs.existsSync(repoDir)) {
          fs.rmSync(repoDir, { recursive: true, force: true });
          this.logActivity(
            "Remove Repo",
            "success",
            `Deleted index data for ${hash}`,
          );
        } else {
          this.logActivity(
            "Remove Repo",
            "info",
            `Index data not found for ${hash}`,
          );
        }
      } catch (err: any) {
        vscode.window.showErrorMessage(
          `Failed to delete index data: ${err.message}`,
        );
        this.logActivity(
          "Remove Repo",
          "error",
          `Cleanup failed: ${err.message}`,
        );
      }
    }

    unregisterRepo(hash);

    if (answer === "No, keep data") {
      this.logActivity(
        "Remove Repo",
        "warning",
        `Unregistered indexed repo: ${hash}`,
      );
    }

    await this.refresh();
  }

  /**
   * Format timestamp as relative time.
   */
  private formatTimeAgo(timestamp: number): string {
    const seconds = Math.floor((Date.now() - timestamp) / 1000);

    if (seconds < 60) return `${seconds}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    return `${Math.floor(seconds / 86400)}d ago`;
  }

  /**
   * Handle clear activity log request.
   */
  private async handleClearActivityLog(): Promise<void> {
    this.activityLog = [];
    await this.refresh();
  }

  /**
   * Handle copy diagnostics request.
   */
  private async handleCopyDiagnostics(): Promise<void> {
    const diagnostics = await this.collectDiagnostics();
    await vscode.env.clipboard.writeText(diagnostics);
    vscode.window.showInformationMessage("Diagnostics copied to clipboard");
    this.logActivity(
      "Copy Diagnostics",
      "success",
      "System diagnostics copied to clipboard",
    );
  }

  /**
   * Collect system diagnostics.
   */
  private async collectDiagnostics(): Promise<string> {
    let extensionVersion = "unknown";
    for (const ext of vscode.extensions.all) {
      if (ext.packageJSON?.name === "omnicontext") {
        extensionVersion = ext.packageJSON.version;
        break;
      }
    }
    const workspaceFolders = vscode.workspace.workspaceFolders;

    let diagnostics = "# OmniContext Diagnostics\n\n";
    diagnostics += `Extension Version: ${extensionVersion}\n`;
    diagnostics += `IDE: ${vscode.env.appName} ${vscode.version}\n`;
    diagnostics += `Platform: ${process.platform} ${process.arch}\n`;
    diagnostics += `Node Version: ${process.version}\n\n`;

    if (workspaceFolders && workspaceFolders.length > 0) {
      diagnostics += `Workspace: ${workspaceFolders[0].uri.fsPath}\n`;
    }

    // Get system status if available
    try {
      const status = await this.getSystemStatus();
      if (status) {
        diagnostics += `\n## System Status\n`;
        diagnostics += `Initialization: ${status.initialization_status}\n`;
        diagnostics += `Connection: ${status.connection_health}\n`;
        diagnostics += `Files Indexed: ${status.files_indexed}\n`;
        diagnostics += `Chunks Indexed: ${status.chunks_indexed}\n`;
        diagnostics += `Daemon Uptime: ${status.daemon_uptime_seconds}s\n`;
      }
    } catch (err) {
      diagnostics += `\nDaemon Status: Not connected\n`;
    }

    return diagnostics;
  }

  /**
   * Handle open logs request.
   */
  private async handleOpenLogs(): Promise<void> {
    vscode.commands.executeCommand("workbench.action.output.show");
    this.logActivity("Open Logs", "info", "Output channel opened");
  }

  /**
   * Handle view activity details request.
   */
  private async handleViewActivityDetails(index: number): Promise<void> {
    if (index >= 0 && index < this.activityLog.length) {
      const activity =
        this.activityLog[
          this.activityLog.length - this.maxActivityEntries + index
        ];
      if (activity) {
        vscode.window
          .showInformationMessage(
            `${activity.type}: ${activity.details}`,
            "View Logs",
          )
          .then((selection) => {
            if (selection === "View Logs") {
              vscode.commands.executeCommand("workbench.action.output.show");
            }
          });
      }
    }
  }

  /**
   * Generate HTML for the webview.
   */
  private getHtmlForWebview(webview: vscode.Webview): string {
    // Get codicon URI
    const codiconUri = webview.asWebviewUri(
      vscode.Uri.joinPath(
        this.extensionUri,
        "node_modules",
        "@vscode/codicons",
        "dist",
        "codicon.css",
      ),
    );

    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OmniContext</title>
    <link href="${codiconUri}" rel="stylesheet" />
    <style>
        body {
            font-family: var(--vscode-font-family);
            color: var(--vscode-foreground);
            padding: 12px;
            background: var(--vscode-sideBar-background);
        }
        
        /* VS Code Codicon Support */
        .codicon {
            font-family: codicon;
            font-size: 16px;
        }
        
        /* Section Styles */
        .section {
            margin-bottom: 20px;
            padding: 12px;
            background: var(--vscode-editor-background);
            border-radius: 6px;
            border: 1px solid var(--vscode-panel-border);
        }
        
        .section-title {
            font-size: 14px;
            font-weight: 600;
            margin-bottom: 12px;
            color: var(--vscode-foreground);
            display: flex;
            align-items: center;
            justify-content: space-between;
        }
        
        .refresh-btn {
            background: none;
            border: none;
            color: var(--vscode-foreground);
            cursor: pointer;
            padding: 4px;
            opacity: 0.7;
            transition: opacity 0.2s;
        }
        
        .refresh-btn:hover {
            opacity: 1;
        }
        
        /* Status Row */
        .status-row {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 8px;
            background: var(--vscode-input-background);
            border-radius: 4px;
            margin-bottom: 8px;
            border-left: 3px solid transparent;
        }
        
        .status-row.active { border-left-color: #4ade80; }
        .status-row.disabled { border-left-color: #f87171; }
        .status-row.offline { border-left-color: #fbbf24; }
        
        .status-label {
            font-size: 12px;
            color: var(--vscode-descriptionForeground);
        }
        
        .status-value {
            font-size: 12px;
            font-weight: 500;
            display: flex;
            align-items: center;
            gap: 4px;
        }
        
        .status-icon {
            font-size: 14px;
        }
        
        .status-indicator {
            display: inline-block;
            width: 8px;
            height: 8px;
            border-radius: 50%;
            margin-right: 6px;
        }
        
        .status-indicator.green { background-color: #4ade80; }
        .status-indicator.yellow { background-color: #fbbf24; }
        .status-indicator.red { background-color: #f87171; }
        .status-indicator.gray { background-color: #9e9e9e; }
        
        .status-indicator.pulsing {
            animation: pulse 2s infinite;
        }
        
        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }
        
        /* Metric Row */
        .metric-row {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 6px 0;
            border-bottom: 1px solid var(--vscode-panel-border);
        }
        
        .metric-row:last-child {
            border-bottom: none;
        }
        
        .metric-label {
            font-size: 12px;
            color: var(--vscode-descriptionForeground);
        }
        
        .metric-value {
            font-size: 12px;
            font-weight: 500;
            font-family: var(--vscode-editor-font-family);
        }
        
        /* Toggle Switch */
        .toggle-row {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 8px 0;
        }
        
        .toggle-label {
            font-size: 12px;
        }
        
        .toggle-switch {
            position: relative;
            width: 40px;
            height: 20px;
        }
        
        .toggle-switch input {
            opacity: 0;
            width: 0;
            height: 0;
        }
        
        .toggle-slider {
            position: absolute;
            cursor: pointer;
            top: 0;
            left: 0;
            right: 0;
            bottom: 0;
            background-color: var(--vscode-input-background);
            transition: 0.3s;
            border-radius: 20px;
        }
        
        .toggle-slider:before {
            position: absolute;
            content: "";
            height: 14px;
            width: 14px;
            left: 3px;
            bottom: 3px;
            background-color: white;
            transition: 0.3s;
            border-radius: 50%;
        }
        
        input:checked + .toggle-slider {
            background-color: #4ade80;
        }
        
        input:checked + .toggle-slider:before {
            transform: translateX(20px);
        }
        
        /* Button Styles */
        .btn {
            width: 100%;
            padding: 8px 12px;
            margin-top: 8px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 12px;
            font-weight: 500;
            transition: opacity 0.2s;
        }
        
        .btn:hover {
            opacity: 0.8;
        }
        
        .btn-primary {
            background: var(--vscode-button-background);
            color: var(--vscode-button-foreground);
        }
        
        .btn-secondary {
            background: var(--vscode-button-secondaryBackground);
            color: var(--vscode-button-secondaryForeground);
        }
        
        /* Activity Log Styles */
        .activity-item {
            padding: 6px 8px;
            margin-bottom: 4px;
            background: var(--vscode-input-background);
            border-radius: 4px;
            border-left: 3px solid transparent;
            font-size: 11px;
            cursor: pointer;
            transition: opacity 0.2s;
        }
        
        /* Indexed Repo Styles */
        .repo-item {
            padding: 8px;
            margin-bottom: 4px;
            background: var(--vscode-input-background);
            border-radius: 4px;
            border-left: 3px solid transparent;
            font-size: 11px;
        }
        
        .repo-item.active { border-left-color: #4ade80; }
        .repo-item.stale { border-left-color: #fbbf24; }
        .repo-item.missing { border-left-color: #f87171; }
        
        .repo-item-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 2px;
        }
        
        .repo-item-name {
            font-weight: 600;
            font-size: 11px;
        }
        
        .repo-item-badge {
            font-size: 9px;
            padding: 1px 5px;
            border-radius: 3px;
            font-weight: 500;
        }
        
        .repo-item.active {
            opacity: 1;
            border-left: 3px solid #4ade80;
            background: rgba(74, 222, 128, 0.05);
            box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
        }
        
        .repo-item-badge.active {
            background: rgba(74, 222, 128, 0.15);
            color: #4ade80;
            font-weight: bold;
            border: 1px solid rgba(74, 222, 128, 0.3);
        }
        
        .repo-item-badge.stale {
            background: rgba(251, 191, 36, 0.15);
            color: #fbbf24;
        }
        
        .repo-item-badge.missing {
            background: rgba(248, 113, 113, 0.15);
            color: #f87171;
        }
        
        .repo-item-meta {
            font-size: 10px;
            color: var(--vscode-descriptionForeground);
            margin-bottom: 4px;
        }
        
        .repo-item-actions {
            display: flex;
            justify-content: flex-end;
        }
        
        .repo-item-remove {
            background: none;
            border: none;
            color: var(--vscode-descriptionForeground);
            cursor: pointer;
            font-size: 10px;
            padding: 2px 4px;
            opacity: 0.6;
            transition: opacity 0.2s, color 0.2s;
        }
        
        .repo-item-remove:hover {
            opacity: 1;
            color: #f87171;
        }
        
        .activity-item:hover {
            opacity: 0.8;
        }
        
        .activity-item.success { border-left-color: #4ade80; }
        .activity-item.error { border-left-color: #f87171; }
        .activity-item.warning { border-left-color: #fbbf24; }
        .activity-item.info { border-left-color: #60a5fa; }
        
        .activity-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 2px;
        }
        
        .activity-type {
            font-weight: 500;
        }
        
        .activity-time {
            font-size: 10px;
            color: var(--vscode-descriptionForeground);
        }
        
        .activity-details {
            font-size: 10px;
            color: var(--vscode-descriptionForeground);
        }
    </style>
</head>
<body>
    <!-- Section 1: Indexed Repositories (TOP) -->
    <div class="section">
        <div class="section-title">
            <span><i class="codicon codicon-database"></i> Indexed Repositories</span>
            <button class="refresh-btn" onclick="refreshStatus()" title="Refresh Registry" style="display: flex; align-items: center; gap: 4px; padding: 2px 6px; border-radius: 4px; background: rgba(96, 165, 250, 0.1); color: #60a5fa; border: 1px solid rgba(96, 165, 250, 0.2); font-size: 10px; font-weight: bold; cursor: pointer; transition: all 0.2s;">
                <i class="codicon codicon-sync"></i> Refresh
            </button>
        </div>
        
        <div id="indexed-repos-list" style="max-height: 220px; overflow-y: auto;">
            <div style="text-align: center; padding: 12px; color: var(--vscode-descriptionForeground); font-size: 11px;">
                No indexed repositories found
            </div>
        </div>

        <button class="btn btn-primary" onclick="reindexRepository()" id="reindex-btn">Index Current Workspace</button>
        
        <div style="margin-top: 4px;">
            <div style="background: var(--vscode-input-background); border-radius: 4px; height: 6px; overflow: hidden; display: none;" id="progress-bar-container">
                <div id="progress-bar" style="background: #4ade80; height: 100%; width: 0%; transition: width 0.3s;"></div>
            </div>
        </div>
        
        <button class="btn btn-secondary" onclick="cleanupOrphans()">Clean Up Orphaned Indexes</button>
    </div>

    <!-- Section 2: Engine Status -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-pulse"></i> Engine Status</div>
        
        <div class="metric-row">
            <span class="metric-label">Repository:</span>
            <span class="metric-value" id="repo-path" style="font-size: 10px; word-break: break-all;">-</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Initialization:</span>
            <span class="metric-value" id="init-status">
                <span class="status-indicator gray"></span>
                <span>Unknown</span>
            </span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Connection:</span>
            <span class="metric-value" id="connection-status">
                <span class="status-indicator gray"></span>
                <span>Unknown</span>
            </span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Last Indexed:</span>
            <span class="metric-value" id="last-index-time">Never</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Daemon Uptime:</span>
            <span class="metric-value" id="daemon-uptime">0s</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Files Indexed:</span>
            <span class="metric-value" id="files-indexed">0</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Chunks Indexed:</span>
            <span class="metric-value" id="chunks-indexed">0</span>
        </div>
    </div>

    <!-- Section 3: Performance & Cache -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-graph"></i> Performance</div>
        
        <div class="metric-row">
            <span class="metric-label">Search P50 / P95 / P99:</span>
            <span class="metric-value"><span id="latency-p50">0</span> / <span id="latency-p95">0</span> / <span id="latency-p99">0</span>ms</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Embedding Coverage:</span>
            <span class="metric-value" id="embedding-coverage">0%</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Memory:</span>
            <span class="metric-value"><span id="memory-usage">0</span> / <span id="peak-memory">0</span> MB</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Total Searches:</span>
            <span class="metric-value" id="total-searches">0</span>
        </div>
        
        <div style="margin-top: 8px; padding-top: 8px; border-top: 1px solid var(--vscode-panel-border);">
            <div class="section-title" style="font-size: 11px; margin-bottom: 4px;"><i class="codicon codicon-zap"></i> Pre-Fetch Cache</div>
        
            <div class="status-row active" id="prefetch-status">
                <span class="status-label">Status</span>
                <span class="status-value">
                    <span class="status-icon codicon" id="cache-status-icon"></span>
                    <span id="cache-status-text">Active</span>
                </span>
            </div>
            
            <div class="metric-row">
                <span class="metric-label">Hit Rate:</span>
                <span class="metric-value"><span id="cache-hit-rate">0%</span> (<span id="cache-hits">0</span> hits / <span id="cache-misses">0</span> misses)</span>
            </div>
            
            <div class="metric-row">
                <span class="metric-label">Cache Size:</span>
                <span class="metric-value" id="cache-size">0/100</span>
            </div>
            
            <div class="toggle-row">
                <span class="toggle-label">Enable Pre-Fetch</span>
                <label class="toggle-switch">
                    <input type="checkbox" id="prefetch-toggle" checked onchange="togglePrefetch()">
                    <span class="toggle-slider"></span>
                </label>
            </div>
            
            <button class="btn btn-secondary" onclick="clearCache()">Clear Cache</button>
        </div>
    </div>

    <!-- Section 4: Settings -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-settings-gear"></i> Settings</div>
        
        <div class="toggle-row">
            <span class="toggle-label">Auto-Index on Open</span>
            <label class="toggle-switch">
                <input type="checkbox" id="auto-index-toggle" checked onchange="toggleAutoIndex()">
                <span class="toggle-slider"></span>
            </label>
        </div>
        
        <div class="toggle-row">
            <span class="toggle-label">Auto-Start Daemon</span>
            <label class="toggle-switch">
                <input type="checkbox" id="auto-daemon-toggle" checked onchange="toggleAutoDaemon()">
                <span class="toggle-slider"></span>
            </label>
        </div>
        
        <div class="toggle-row">
            <span class="toggle-label">Auto-Sync MCP</span>
            <label class="toggle-switch">
                <input type="checkbox" id="auto-sync-toggle" onchange="toggleAutoSync()">
                <span class="toggle-slider"></span>
            </label>
        </div>
    </div>

    <!-- Section 5: Integrations (IDE-adaptive) -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-plug"></i> Integrations</div>
        
        <button class="btn btn-primary" onclick="quickSearch()">Quick Search</button>
        <button class="btn btn-secondary" id="sync-mcp-btn" onclick="syncMcpConfig()">Sync MCP Config</button>
        <button class="btn btn-secondary" onclick="updateBinary()">Update / Repair</button>
    </div>

    <!-- Section 6: Activity Log -->
    <div class="section">
        <div class="section-title">
            <span><i class="codicon codicon-list-unordered"></i> Activity Log</span>
            <button class="refresh-btn" onclick="clearActivityLog()" title="Clear Log"><i class="codicon codicon-close"></i></button>
        </div>
        
        <div id="activity-log" style="max-height: 200px; overflow-y: auto;">
            <div style="text-align: center; padding: 20px; color: var(--vscode-descriptionForeground); font-size: 11px;">
                No recent activity
            </div>
        </div>
    </div>

    <!-- Section 7: About -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-info"></i> About</div>
        
        <div class="metric-row">
            <span class="metric-label">Extension:</span>
            <span class="metric-value" id="extension-version">-</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">IDE:</span>
            <span class="metric-value" id="ide-info">-</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Platform:</span>
            <span class="metric-value" id="platform-info">-</span>
        </div>
        
        <button class="btn btn-secondary" onclick="copyDiagnostics()">Copy Diagnostics</button>
        <button class="btn btn-secondary" onclick="openLogs()">Open Logs</button>
    </div>

    <script>
        const vscode = acquireVsCodeApi();

        // -- State --
        let _ideName = 'Visual Studio Code';
        let _systemInfo = {};
        let _systemStatus = {};
        let _performanceMetrics = {};
        let _indexedRepos = [];

        // -- Actions --
        function refreshStatus() {
            vscode.postMessage({ command: 'refreshStatus' });
        }

        function togglePrefetch() {
            const enabled = document.getElementById('prefetch-toggle').checked;
            vscode.postMessage({ command: 'togglePrefetch', enabled });
        }
        
        function clearCache() {
            vscode.postMessage({ command: 'clearCache' });
        }

        function reindexRepository() {
            vscode.postMessage({ command: 'reindexRepository' });
        }
        
        function clearIndex() {
            if (confirm('Are you sure you want to clear the entire index?')) {
                vscode.postMessage({ command: 'clearIndex' });
            }
        }
        
        function toggleAutoIndex() {
            const enabled = document.getElementById('auto-index-toggle').checked;
            vscode.postMessage({ command: 'toggleAutoIndex', enabled });
        }
        
        function toggleAutoDaemon() {
            const enabled = document.getElementById('auto-daemon-toggle').checked;
            vscode.postMessage({ command: 'toggleAutoDaemon', enabled });
        }
        
        function toggleAutoSync() {
            const enabled = document.getElementById('auto-sync-toggle').checked;
            vscode.postMessage({ command: 'toggleAutoSync', enabled });
        }
        
        function quickSearch() {
            vscode.postMessage({ command: 'quickSearch' });
        }
        
        function syncMcpConfig() {
            vscode.postMessage({ command: 'syncMcpConfig' });
        }
        
        function cleanupOrphans() {
            vscode.postMessage({ command: 'cleanupOrphans' });
        }

        function updateBinary() {
            vscode.postMessage({ command: 'updateBinary' });
        }
        
        function clearActivityLog() {
            vscode.postMessage({ command: 'clearActivityLog' });
        }
        
        function copyDiagnostics() {
            vscode.postMessage({ command: 'copyDiagnostics' });
        }
        
        function openLogs() {
            vscode.postMessage({ command: 'openLogs' });
        }
        
        function clickActivity(index) {
            vscode.postMessage({ command: 'viewActivityDetails', index });
        }

        function removeIndexedRepo(hash) {
            vscode.postMessage({ command: 'removeIndexedRepo', hash });
        }

        // -- Message listener --
        window.addEventListener('message', event => {
            const message = event.data;
            
            switch (message.type) {
                case 'updateCacheStats':
                    updateCacheStats(message.data);
                    break;
                case 'updateSystemStatus':
                    updateSystemStatus(message.status);
                    break;
                case 'updatePerformanceMetrics':
                    updatePerformanceMetrics(message.metrics);
                    break;
                case 'updateRepositoryInfo':
                    updateRepositoryInfo(message.repoPath);
                    break;
                case 'updateAutomationSettings':
                    updateAutomationSettings(message.settings);
                    break;
                case 'updateActivityLog':
                    updateActivityLog(message.activities);
                    break;
                case 'updateSystemInfo':
                    updateSystemInfo(message.info);
                    break;
                case 'updateIndexedRepos':
                    updateIndexedRepos(message.repos, message.activeRepoPath);
                    break;
            }
        });
        
        // -- Update functions --
        function updateCacheStats(data) {
            const statusRow = document.getElementById('prefetch-status');
            statusRow.className = 'status-row ' + data.status;
            
            const iconElement = document.getElementById('cache-status-icon');
            iconElement.className = 'status-icon codicon';
            
            if (data.status === 'active') {
                iconElement.classList.add('codicon-zap');
            } else if (data.status === 'offline') {
                iconElement.classList.add('codicon-warning');
            } else {
                iconElement.classList.add('codicon-circle-slash');
            }
            
            document.getElementById('cache-status-text').textContent = data.statusText;
            document.getElementById('cache-hit-rate').textContent = data.hitRate;
            document.getElementById('cache-hits').textContent = data.hits;
            document.getElementById('cache-misses').textContent = data.misses;
            document.getElementById('cache-size').textContent = data.cacheSize;
            document.getElementById('prefetch-toggle').checked = data.prefetchEnabled;
        }
        
        function updateSystemStatus(status) {
            _systemStatus = status || {};
            if (!status) {
                document.getElementById('init-status').innerHTML = 
                    '<span class="status-indicator gray"></span><span>Unknown</span>';
                document.getElementById('connection-status').innerHTML = 
                    '<span class="status-indicator gray"></span><span>Unknown</span>';
                return;
            }
            
            const initIndicator = status.initialization_status === 'ready' ? 'green' :
                                 status.initialization_status === 'error' ? 'red' : 'yellow';
            document.getElementById('init-status').innerHTML = 
                \`<span class="status-indicator \${initIndicator}"></span>\` +
                \`<span>\${capitalize(status.initialization_status)}</span>\`;
            
            const connIndicator = status.connection_health === 'connected' ? 'green pulsing' :
                                 status.connection_health === 'reconnecting' ? 'yellow' : 'red';
            document.getElementById('connection-status').innerHTML = 
                \`<span class="status-indicator \${connIndicator}"></span>\` +
                \`<span>\${capitalize(status.connection_health)}</span>\`;
            
            if (status.last_index_time) {
                document.getElementById('last-index-time').textContent = formatRelativeTime(status.last_index_time);
            } else {
                document.getElementById('last-index-time').textContent = 'Never';
            }
            
            document.getElementById('daemon-uptime').textContent = formatUptime(status.daemon_uptime_seconds);
            document.getElementById('files-indexed').textContent = status.files_indexed.toString();
            document.getElementById('chunks-indexed').textContent = status.chunks_indexed.toString();
        }
        
        function updatePerformanceMetrics(metrics) {
            if (!metrics) return;
            _performanceMetrics = metrics;
            
            document.getElementById('latency-p50').textContent = metrics.search_latency_p50_ms.toFixed(1);
            document.getElementById('latency-p95').textContent = metrics.search_latency_p95_ms.toFixed(1);
            document.getElementById('latency-p99').textContent = metrics.search_latency_p99_ms.toFixed(1);
            
            document.getElementById('embedding-coverage').textContent = \`\${metrics.embedding_coverage_percent.toFixed(1)}%\`;
            
            const memMB = (metrics.memory_usage_bytes / (1024 * 1024)).toFixed(1);
            const peakMB = (metrics.peak_memory_usage_bytes / (1024 * 1024)).toFixed(1);
            document.getElementById('memory-usage').textContent = memMB;
            document.getElementById('peak-memory').textContent = peakMB;
            
            document.getElementById('total-searches').textContent = metrics.total_searches.toString();
        }
        
        function updateRepositoryInfo(repoPath) {
            if (!repoPath) return;
            const parts = repoPath.split(/[\\\\/]/);
            const folderName = parts[parts.length - 1] || repoPath;
            document.getElementById('repo-path').textContent = folderName;
            document.getElementById('repo-path').title = repoPath;
        }
        
        function updateAutomationSettings(settings) {
            if (!settings) return;
            document.getElementById('auto-index-toggle').checked = settings.autoIndex;
            document.getElementById('auto-daemon-toggle').checked = settings.autoStartDaemon;
            document.getElementById('auto-sync-toggle').checked = settings.autoSyncMcp;
        }
        
        function updateActivityLog(activities) {
            const logContainer = document.getElementById('activity-log');
            
            if (!activities || activities.length === 0) {
                logContainer.innerHTML = '<div style="text-align: center; padding: 20px; color: var(--vscode-descriptionForeground); font-size: 11px;">No recent activity</div>';
                return;
            }
            
            logContainer.innerHTML = activities.map((activity, index) => {
                const statusClass = activity.status === 'success' ? 'success' : 
                                   activity.status === 'error' ? 'error' : 
                                   activity.status === 'warning' ? 'warning' : 'info';
                
                return \`
                    <div class="activity-item \${statusClass}" onclick="clickActivity(\${index})">
                        <div class="activity-header">
                            <span class="activity-type">\${activity.type}</span>
                            <span class="activity-time">\${activity.time}</span>
                        </div>
                        <div class="activity-details">\${activity.details}</div>
                    </div>
                \`;
            }).join('');
        }
        
        function updateSystemInfo(info) {
            if (!info) return;
            _systemInfo = info;
            _ideName = info.ideName || 'Visual Studio Code';
            
            document.getElementById('extension-version').textContent = 'v' + (info.version || 'unknown');
            document.getElementById('platform-info').textContent = info.platform || '-';
            document.getElementById('ide-info').textContent = \`\${_ideName} \${info.ideVersion || ''}\`;
            
            // Adapt Sync MCP button label based on IDE
            const syncBtn = document.getElementById('sync-mcp-btn');
            const name = _ideName.toLowerCase();
            if (name.includes('cursor')) {
                syncBtn.textContent = 'Sync MCP to Cursor';
            } else if (name.includes('kiro')) {
                syncBtn.textContent = 'Sync MCP to Kiro';
            } else if (name.includes('windsurf')) {
                syncBtn.textContent = 'Sync MCP to Windsurf';
            } else if (name.includes('antigravity')) {
                syncBtn.style.display = 'none'; // Built-in, no sync needed
            } else if (name.includes('cloud code')) {
                syncBtn.textContent = 'Sync MCP to Cloud Code';
            } else {
                // VS Code / Codium / etc.
                syncBtn.textContent = 'Sync MCP to Claude/Copilot';
            }
        }

        function updateIndexedRepos(repos, activeRepoPath) {
            const container = document.getElementById('indexed-repos-list');
            _indexedRepos = repos || [];
            
            if (!repos || repos.length === 0) {
                container.innerHTML = '<div style="text-align: center; padding: 12px; color: var(--vscode-descriptionForeground); font-size: 11px;">No indexed repositories found</div>';
                return;
            }
            
            const normalizedActive = (activeRepoPath || '').replace(/\\\\/g, '/').toLowerCase();
            
            container.innerHTML = repos.map(repo => {
                const normalizedRepo = repo.repoPath.replace(/\\\\/g, '/').toLowerCase();
                const isActive = normalizedActive && normalizedRepo === normalizedActive;
                const isMissing = !repo.exists;
                
                let statusClass = 'stale';
                let badgeText = 'Ready';
                
                if (isMissing) {
                    statusClass = 'missing';
                    badgeText = 'Missing';
                } else if (isActive) {
                    statusClass = 'active';
                    badgeText = 'Active';
                }
                
                const indexedAgo = formatRelativeTimeMs(repo.lastIndexedAt);
                
                return \`
                    <div class="repo-item \${statusClass}" title="\${repo.repoPath}">
                        <div class="repo-item-header">
                            <span class="repo-item-name">\${repo.name}</span>
                            <span class="repo-item-badge \${statusClass}">\${badgeText}</span>
                        </div>
                        <div class="repo-item-meta">
                            \${repo.filesIndexed} files, \${repo.chunksIndexed} chunks -- \${indexedAgo}
                        </div>
                        <div class="repo-item-actions">
                            <button class="repo-item-remove" onclick="removeIndexedRepo('\${repo.hash}')" title="Remove from registry">
                                <i class="codicon codicon-close"></i> Remove
                            </button>
                        </div>
                    </div>
                \`;
            }).join('');
        }
        
        function formatRelativeTimeMs(timestampMs) {
            const seconds = Math.floor((Date.now() - timestampMs) / 1000);
            
            if (seconds < 60) return \`\${seconds}s ago\`;
            if (seconds < 3600) return \`\${Math.floor(seconds / 60)}m ago\`;
            if (seconds < 86400) return \`\${Math.floor(seconds / 3600)}h ago\`;
            return \`\${Math.floor(seconds / 86400)}d ago\`;
        }
        
        function formatRelativeTime(timestamp) {
            const now = Math.floor(Date.now() / 1000);
            const diff = now - timestamp;
            
            if (diff < 60) return \`\${diff}s ago\`;
            if (diff < 3600) return \`\${Math.floor(diff / 60)}m ago\`;
            if (diff < 86400) return \`\${Math.floor(diff / 3600)}h ago\`;
            return \`\${Math.floor(diff / 86400)}d ago\`;
        }
        
        function formatUptime(seconds) {
            if (seconds < 60) return \`\${seconds}s\`;
            if (seconds < 3600) return \`\${Math.floor(seconds / 60)}m \${seconds % 60}s\`;
            const hours = Math.floor(seconds / 3600);
            const mins = Math.floor((seconds % 3600) / 60);
            return \`\${hours}h \${mins}m\`;
        }
        
        function capitalize(str) {
            return str.charAt(0).toUpperCase() + str.slice(1);
        }
    </script>
</body>
</html>`;
  }
}
