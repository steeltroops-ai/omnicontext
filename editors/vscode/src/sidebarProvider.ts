/**
 * Webview provider for OmniContext sidebar.
 * Provides comprehensive system status, metrics, and controls.
 */

import * as vscode from 'vscode';
import { CacheStatsManager } from './cacheStats';
import { EventTracker } from './eventTracker';

interface SystemStatus {
    initialization_status: 'initializing' | 'ready' | 'error';
    connection_health: 'connected' | 'disconnected' | 'reconnecting';
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
    status: 'success' | 'error' | 'warning' | 'info';
    time: string;
    details: string;
    timestamp: number;
}

export class OmniSidebarProvider implements vscode.WebviewViewProvider {
    private _view?: vscode.WebviewView;
    private activityLog: ActivityLogEntry[] = [];
    private readonly maxActivityEntries = 10;

    constructor(
        private readonly extensionUri: vscode.Uri,
        private readonly cacheStatsManager: CacheStatsManager,
        private readonly eventTracker: EventTracker,
        private readonly sendIpcRequest: (method: string, params: any) => Promise<any>
    ) { }

    /**
     * Resolve the webview view.
     */
    public async resolveWebviewView(
        webviewView: vscode.WebviewView,
        context: vscode.WebviewViewResolveContext,
        token: vscode.CancellationToken
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

        // Initial refresh
        await this.refresh();
    }

    /**
     * Refresh the sidebar with latest data.
     */
    public async refresh(): Promise<void> {
        if (!this._view) {
            return;
        }

        // Retrieve cache statistics
        const cacheStats = await this.cacheStatsManager.getStats();

        // Retrieve system status
        const systemStatus = await this.getSystemStatus();

        // Retrieve performance metrics
        const performanceMetrics = await this.getPerformanceMetrics();

        // Get prefetch enabled state from configuration
        const config = vscode.workspace.getConfiguration('omnicontext.prefetch');
        const prefetchEnabled = config.get<boolean>('enabled', true);

        // Determine cache status
        let cacheStatus: 'active' | 'disabled' | 'offline';
        let cacheStatusText: string;

        if (!cacheStats) {
            cacheStatus = 'offline';
            cacheStatusText = 'Offline';
        } else if (!prefetchEnabled) {
            cacheStatus = 'disabled';
            cacheStatusText = 'Disabled';
        } else {
            cacheStatus = 'active';
            cacheStatusText = 'Active';
        }

        // Format cache statistics
        const hitRate = cacheStats ? `${(cacheStats.hit_rate * 100).toFixed(1)}%` : '0%';
        const hits = cacheStats ? cacheStats.hits.toString() : '0';
        const misses = cacheStats ? cacheStats.misses.toString() : '0';
        const cacheSize = cacheStats ? `${cacheStats.size}/${cacheStats.capacity}` : '0/100';

        this._view.webview.postMessage({
            type: 'updateCacheStats',
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

        // Send system status update
        if (systemStatus) {
            this._view.webview.postMessage({
                type: 'updateSystemStatus',
                status: systemStatus,
            });
        }

        // Send performance metrics update
        if (performanceMetrics) {
            this._view.webview.postMessage({
                type: 'updatePerformanceMetrics',
                metrics: performanceMetrics,
            });
        }

        // Send repository info
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders && workspaceFolders.length > 0) {
            const repoPath = workspaceFolders[0].uri.fsPath;
            this._view.webview.postMessage({
                type: 'updateRepositoryInfo',
                repoPath,
            });
        }

        // Send automation settings
        const omniConfig = vscode.workspace.getConfiguration('omnicontext');
        const automationConfig = vscode.workspace.getConfiguration('omnicontext.automation');
        this._view.webview.postMessage({
            type: 'updateAutomationSettings',
            settings: {
                autoIndex: omniConfig.get<boolean>('autoIndex', true),
                autoStartDaemon: omniConfig.get<boolean>('autoStartDaemon', true),
                autoSyncMcp: automationConfig.get<boolean>('autoSyncMcp', false),
            },
        });

        // Send activity log
        this._view.webview.postMessage({
            type: 'updateActivityLog',
            activities: this.activityLog.slice(-this.maxActivityEntries),
        });

        // Send system info
        this._view.webview.postMessage({
            type: 'updateSystemInfo',
            info: {
                version: vscode.extensions.getExtension('steeltroops-ai.omnicontext')?.packageJSON.version || '0.2.0',
                platform: `${process.platform} ${process.arch}`,
            },
        });
    }

    /**
     * Get system status from daemon.
     */
    private async getSystemStatus(): Promise<SystemStatus | null> {
        try {
            const result = await this.sendIpcRequest('system_status', {});
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
            const result = await this.sendIpcRequest('performance_metrics', {});
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
            case 'refreshStatus':
                await this.refresh();
                break;

            case 'clearCache':
                await this.handleClearCache();
                break;

            case 'togglePrefetch':
                await this.handleTogglePrefetch(message.enabled);
                break;

            case 'reindexRepository':
                await this.handleReindexRepository();
                break;

            case 'clearIndex':
                await this.handleClearIndex();
                break;

            case 'toggleAutoIndex':
                await this.handleToggleAutoIndex(message.enabled);
                break;

            case 'toggleAutoDaemon':
                await this.handleToggleAutoDaemon(message.enabled);
                break;

            case 'toggleAutoSync':
                await this.handleToggleAutoSync(message.enabled);
                break;

            case 'quickSearch':
                await this.handleQuickSearch();
                break;

            case 'syncToClaudeMcp':
                await this.handleSyncToClaudeMcp();
                break;

            case 'syncToKiroMcp':
                await this.handleSyncToKiroMcp();
                break;

            case 'clearActivityLog':
                await this.handleClearActivityLog();
                break;

            case 'copyDiagnostics':
                await this.handleCopyDiagnostics();
                break;

            case 'openLogs':
                await this.handleOpenLogs();
                break;

            case 'viewActivityDetails':
                await this.handleViewActivityDetails(message.index);
                break;

            default:
                console.warn('Unknown webview message:', message);
        }
    }

    /**
     * Handle clear cache request.
     */
    private async handleClearCache(): Promise<void> {
        try {
            await this.cacheStatsManager.clearCache();
            vscode.window.showInformationMessage('Cache cleared successfully');
            this.logActivity('Clear Cache', 'success', 'Pre-fetch cache cleared');
            await this.refresh();
        } catch (err) {
            vscode.window.showErrorMessage(`Failed to clear cache: ${err}`);
            this.logActivity('Clear Cache', 'error', `Failed: ${err}`);
        }
    }

    /**
     * Handle toggle prefetch request.
     */
    private async handleTogglePrefetch(enabled: boolean): Promise<void> {
        const config = vscode.workspace.getConfiguration('omnicontext.prefetch');
        await config.update('enabled', enabled, vscode.ConfigurationTarget.Workspace);
        this.eventTracker.setEnabled(enabled);
        await this.refresh();
    }

    /**
     * Handle re-index repository request.
     */
    private async handleReindexRepository(): Promise<void> {
        try {
            this.logActivity('Re-index', 'info', 'Starting repository re-index...');

            // Show progress notification
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: 'Re-indexing repository...',
                    cancellable: false,
                },
                async (progress) => {
                    progress.report({ increment: 0, message: 'Starting indexing...' });

                    // Trigger re-index via IPC
                    const result = await this.sendIpcRequest('index', {});

                    progress.report({ increment: 100, message: 'Complete!' });

                    const message = `Re-indexed ${result.files_processed} files, ${result.chunks_created} chunks in ${result.elapsed_ms}ms`;
                    vscode.window.showInformationMessage(message);
                    this.logActivity('Re-index', 'success', message);
                }
            );

            await this.refresh();
        } catch (err: any) {
            vscode.window.showErrorMessage(`Failed to re-index: ${err.message}`);
            this.logActivity('Re-index', 'error', `Failed: ${err.message}`);
        }
    }

    /**
     * Handle clear index request.
     */
    private async handleClearIndex(): Promise<void> {
        try {
            // Clear the index by sending a clear_index IPC request
            await this.sendIpcRequest('clear_index', {});
            vscode.window.showInformationMessage('Index cleared successfully. Re-indexing recommended.');
            this.logActivity('Clear Index', 'warning', 'Index cleared - re-indexing recommended');
            await this.refresh();
        } catch (err: any) {
            vscode.window.showErrorMessage(`Failed to clear index: ${err.message}`);
            this.logActivity('Clear Index', 'error', `Failed: ${err.message}`);
        }
    }

    /**
     * Handle toggle auto-index request.
     */
    private async handleToggleAutoIndex(enabled: boolean): Promise<void> {
        const config = vscode.workspace.getConfiguration('omnicontext');
        await config.update('autoIndex', enabled, vscode.ConfigurationTarget.Global);
        vscode.window.showInformationMessage(`Auto-index ${enabled ? 'enabled' : 'disabled'}`);
    }

    /**
     * Handle toggle auto-daemon request.
     */
    private async handleToggleAutoDaemon(enabled: boolean): Promise<void> {
        const config = vscode.workspace.getConfiguration('omnicontext');
        await config.update('autoStartDaemon', enabled, vscode.ConfigurationTarget.Global);
        vscode.window.showInformationMessage(`Auto-start daemon ${enabled ? 'enabled' : 'disabled'}`);
    }

    /**
     * Handle toggle auto-sync request.
     */
    private async handleToggleAutoSync(enabled: boolean): Promise<void> {
        const config = vscode.workspace.getConfiguration('omnicontext.automation');
        await config.update('autoSyncMcp', enabled, vscode.ConfigurationTarget.Global);
        vscode.window.showInformationMessage(`Auto-sync MCP ${enabled ? 'enabled' : 'disabled'}`);
    }

    /**
     * Handle quick search request.
     */
    private async handleQuickSearch(): Promise<void> {
        // Trigger the search command
        vscode.commands.executeCommand('omnicontext.search');
    }

    /**
     * Handle sync to Claude MCP request.
     */
    private async handleSyncToClaudeMcp(): Promise<void> {
        // Trigger the sync command
        vscode.commands.executeCommand('omnicontext.syncMcp');
    }

    /**
     * Handle sync to Kiro MCP request.
     */
    private async handleSyncToKiroMcp(): Promise<void> {
        // Trigger the sync command (same as Claude for now)
        vscode.commands.executeCommand('omnicontext.syncMcp');
    }

    /**
     * Log an activity to the activity log.
     */
    public logActivity(type: string, status: 'success' | 'error' | 'warning' | 'info', details: string): void {
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
                type: 'updateActivityLog',
                activities: this.activityLog.slice(-this.maxActivityEntries),
            });
        }
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
        vscode.window.showInformationMessage('Diagnostics copied to clipboard');
        this.logActivity('Copy Diagnostics', 'success', 'System diagnostics copied to clipboard');
    }

    /**
     * Collect system diagnostics.
     */
    private async collectDiagnostics(): Promise<string> {
        const extension = vscode.extensions.getExtension('steeltroops-ai.omnicontext');
        const workspaceFolders = vscode.workspace.workspaceFolders;

        let diagnostics = '# OmniContext Diagnostics\n\n';
        diagnostics += `Extension Version: ${extension?.packageJSON.version || 'unknown'}\n`;
        diagnostics += `VS Code Version: ${vscode.version}\n`;
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
        vscode.commands.executeCommand('workbench.action.output.show');
        this.logActivity('Open Logs', 'info', 'Output channel opened');
    }

    /**
     * Handle view activity details request.
     */
    private async handleViewActivityDetails(index: number): Promise<void> {
        if (index >= 0 && index < this.activityLog.length) {
            const activity = this.activityLog[this.activityLog.length - this.maxActivityEntries + index];
            if (activity) {
                vscode.window.showInformationMessage(
                    `${activity.type}: ${activity.details}`,
                    'View Logs'
                ).then(selection => {
                    if (selection === 'View Logs') {
                        vscode.commands.executeCommand('workbench.action.output.show');
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
            vscode.Uri.joinPath(this.extensionUri, 'node_modules', '@vscode/codicons', 'dist', 'codicon.css')
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
    <!-- System Status Section -->
    <div class="section">
        <div class="section-title">
            <span><i class="codicon codicon-pulse"></i> System Status</span>
            <button class="refresh-btn" onclick="refreshStatus()" title="Refresh Status"><i class="codicon codicon-sync"></i></button>
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

    <!-- Pre-Fetch Cache Section -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-zap"></i> Pre-Fetch Cache</div>
        
        <div class="status-row active" id="prefetch-status">
            <span class="status-label">Status</span>
            <span class="status-value">
                <span class="status-icon codicon" id="cache-status-icon"></span>
                <span id="cache-status-text">Active</span>
            </span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Hit Rate</span>
            <span class="metric-value" id="cache-hit-rate">0%</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Cache Hits</span>
            <span class="metric-value" id="cache-hits">0</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Cache Misses</span>
            <span class="metric-value" id="cache-misses">0</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Cache Size</span>
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

    <!-- Performance Metrics Section -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-graph"></i> Performance Metrics</div>
        
        <div class="metric-row">
            <span class="metric-label">Search Latency (P50):</span>
            <span class="metric-value" id="latency-p50">0ms</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Search Latency (P95):</span>
            <span class="metric-value" id="latency-p95">0ms</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Search Latency (P99):</span>
            <span class="metric-value" id="latency-p99">0ms</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Embedding Coverage:</span>
            <span class="metric-value" id="embedding-coverage">0%</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Memory Usage:</span>
            <span class="metric-value" id="memory-usage">0 MB</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Peak Memory:</span>
            <span class="metric-value" id="peak-memory">0 MB</span>
        </div>
        
        <div class="metric-row">
            <span class="metric-label">Total Searches:</span>
            <span class="metric-value" id="total-searches">0</span>
        </div>
    </div>

    <!-- Repository Management Section -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-folder"></i> Repository Management</div>
        
        <div class="metric-row">
            <span class="metric-label">Current Repository:</span>
            <span class="metric-value" id="repo-path" style="font-size: 10px; word-break: break-all;">-</span>
        </div>
        
        <div class="metric-row" id="indexing-progress-row" style="display: none;">
            <span class="metric-label">Indexing Progress:</span>
            <span class="metric-value" id="indexing-progress">0%</span>
        </div>
        
        <div style="margin-top: 8px;">
            <div style="background: var(--vscode-input-background); border-radius: 4px; height: 6px; overflow: hidden; display: none;" id="progress-bar-container">
                <div id="progress-bar" style="background: #4ade80; height: 100%; width: 0%; transition: width 0.3s;"></div>
            </div>
        </div>
        
        <button class="btn btn-primary" onclick="reindexRepository()" id="reindex-btn">Re-index Repository</button>
        <button class="btn btn-secondary" onclick="clearContextCache()">Clear Context Cache</button>
        <button class="btn btn-secondary" onclick="clearIndex()">Clear Index</button>
    </div>

    <!-- Automation Section -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-settings-gear"></i> Automation</div>
        
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

    <!-- Quick Actions Section -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-rocket"></i> Quick Actions</div>
        
        <button class="btn btn-primary" onclick="quickSearch()">Quick Search</button>
        <button class="btn btn-secondary" onclick="syncToClaudeMcp()">Sync to Claude</button>
        <button class="btn btn-secondary" onclick="syncToKiroMcp()">Sync to Kiro</button>
    </div>

    <!-- Activity Log Section -->
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

    <!-- System Information Section -->
    <div class="section">
        <div class="section-title"><i class="codicon codicon-info"></i> System Information</div>
        
        <div class="metric-row">
            <span class="metric-label">Extension Version:</span>
            <span class="metric-value" id="extension-version">0.2.0</span>
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
        
        function clearContextCache() {
            vscode.postMessage({ command: 'clearCache' });
        }
        
        function clearIndex() {
            if (confirm('Are you sure you want to clear the entire index? This will require a full re-index.')) {
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
        
        function syncToClaudeMcp() {
            vscode.postMessage({ command: 'syncToClaudeMcp' });
        }
        
        function syncToKiroMcp() {
            vscode.postMessage({ command: 'syncToKiroMcp' });
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

        // Listen for updates from extension
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
            }
        });
        
        function updateCacheStats(data) {
            // Update status indicator
            const statusRow = document.getElementById('prefetch-status');
            statusRow.className = 'status-row ' + data.status;
            
            // Use codicon classes
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
            
            // Update metrics
            document.getElementById('cache-hit-rate').textContent = data.hitRate;
            document.getElementById('cache-hits').textContent = data.hits;
            document.getElementById('cache-misses').textContent = data.misses;
            document.getElementById('cache-size').textContent = data.cacheSize;
            
            // Update toggle state
            document.getElementById('prefetch-toggle').checked = data.prefetchEnabled;
        }
        
        function updateSystemStatus(status) {
            if (!status) {
                document.getElementById('init-status').innerHTML = 
                    '<span class="status-indicator gray"></span><span>Unknown</span>';
                document.getElementById('connection-status').innerHTML = 
                    '<span class="status-indicator gray"></span><span>Unknown</span>';
                return;
            }
            
            // Update initialization status
            const initIndicator = status.initialization_status === 'ready' ? 'green' :
                                 status.initialization_status === 'error' ? 'red' : 'yellow';
            document.getElementById('init-status').innerHTML = 
                \`<span class="status-indicator \${initIndicator}"></span>\` +
                \`<span>\${capitalize(status.initialization_status)}</span>\`;
            
            // Update connection health
            const connIndicator = status.connection_health === 'connected' ? 'green pulsing' :
                                 status.connection_health === 'reconnecting' ? 'yellow' : 'red';
            document.getElementById('connection-status').innerHTML = 
                \`<span class="status-indicator \${connIndicator}"></span>\` +
                \`<span>\${capitalize(status.connection_health)}</span>\`;
            
            // Update last index time
            if (status.last_index_time) {
                const relativeTime = formatRelativeTime(status.last_index_time);
                document.getElementById('last-index-time').textContent = relativeTime;
            } else {
                document.getElementById('last-index-time').textContent = 'Never';
            }
            
            // Update daemon uptime
            const uptime = formatUptime(status.daemon_uptime_seconds);
            document.getElementById('daemon-uptime').textContent = uptime;
            
            // Update file and chunk counts
            document.getElementById('files-indexed').textContent = status.files_indexed.toString();
            document.getElementById('chunks-indexed').textContent = status.chunks_indexed.toString();
        }
        
        function updatePerformanceMetrics(metrics) {
            if (!metrics) {
                return;
            }
            
            // Update latency metrics
            document.getElementById('latency-p50').textContent = \`\${metrics.search_latency_p50_ms.toFixed(1)}ms\`;
            document.getElementById('latency-p95').textContent = \`\${metrics.search_latency_p95_ms.toFixed(1)}ms\`;
            document.getElementById('latency-p99').textContent = \`\${metrics.search_latency_p99_ms.toFixed(1)}ms\`;
            
            // Update embedding coverage
            const coveragePercent = metrics.embedding_coverage_percent.toFixed(1);
            document.getElementById('embedding-coverage').textContent = \`\${coveragePercent}%\`;
            
            // Update memory usage (convert bytes to MB)
            const memoryMB = (metrics.memory_usage_bytes / (1024 * 1024)).toFixed(1);
            const peakMemoryMB = (metrics.peak_memory_usage_bytes / (1024 * 1024)).toFixed(1);
            document.getElementById('memory-usage').textContent = \`\${memoryMB} MB\`;
            document.getElementById('peak-memory').textContent = \`\${peakMemoryMB} MB\`;
            
            // Update total searches
            document.getElementById('total-searches').textContent = metrics.total_searches.toString();
        }
        
        function updateRepositoryInfo(repoPath) {
            if (!repoPath) {
                return;
            }
            
            // Extract just the folder name for display
            const parts = repoPath.split(/[\\/]/);
            const folderName = parts[parts.length - 1] || repoPath;
            document.getElementById('repo-path').textContent = folderName;
            document.getElementById('repo-path').title = repoPath; // Full path in tooltip
        }
        
        function updateAutomationSettings(settings) {
            if (!settings) {
                return;
            }
            
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
            if (!info) {
                return;
            }
            
            document.getElementById('extension-version').textContent = info.version || '0.2.0';
            document.getElementById('platform-info').textContent = info.platform || '-';
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
