import * as vscode from "vscode";
import * as cp from "child_process";
import * as path from "path";
import * as net from "net";

let statusBarItem: vscode.StatusBarItem;
let outputChannel: vscode.OutputChannel;
let daemonProcess: cp.ChildProcess | null = null;
let ipcClient: net.Socket | null = null;
let contextInjectionEnabled: boolean = true;
let requestCounter = 0;
const pendingRequests = new Map<
  number,
  { resolve: (v: any) => void; reject: (e: Error) => void }
>();

// ---------------------------------------------------------------------------
// Extension lifecycle
// ---------------------------------------------------------------------------

export function activate(context: vscode.ExtensionContext) {
  outputChannel = vscode.window.createOutputChannel("OmniContext");

  // Status bar
  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    100,
  );
  statusBarItem.text = "$(search) OmniContext";
  statusBarItem.tooltip = "OmniContext: Click to search";
  statusBarItem.command = "omnicontext.search";
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

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
  );

  // Register the chat participant for context injection
  registerChatParticipant(context);

  // Auto-start daemon and index on workspace open
  const config = vscode.workspace.getConfiguration("omnicontext");
  if (config.get<boolean>("autoStartDaemon", true)) {
    startDaemon(true);
  } else if (config.get<boolean>("autoIndex", true)) {
    runIndex(true);
  }
}

export function deactivate() {
  stopDaemon();
  statusBarItem?.dispose();
  outputChannel?.dispose();
}

// ---------------------------------------------------------------------------
// Binary resolution
// ---------------------------------------------------------------------------

function getBinaryPath(): string {
  const config = vscode.workspace.getConfiguration("omnicontext");
  const configured = config.get<string>("binaryPath", "");
  if (configured) {
    return configured;
  }

  const candidates = [
    "omnicontext",
    path.join(process.env.HOME || "", ".cargo", "bin", "omnicontext"),
  ];

  for (const candidate of candidates) {
    try {
      cp.execSync(`"${candidate}" --version`, { stdio: "ignore" });
      return candidate;
    } catch {
      // Not found, try next
    }
  }

  vscode.window.showErrorMessage(
    "OmniContext binary not found. Install with: cargo install omni-cli",
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

    daemonProcess.on("exit", (code) => {
      outputChannel.appendLine(`[daemon] exited with code ${code}`);
      daemonProcess = null;
      ipcClient = null;
      statusBarItem.text = "$(search) OmniContext";
    });

    // Wait for daemon to initialize, then connect IPC
    await new Promise((r) => setTimeout(r, 2000));
    await connectIpc(root);

    statusBarItem.text = "$(zap) OmniContext";
    statusBarItem.tooltip = "OmniContext: Daemon active, context injection ON";

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
  if (ipcClient) {
    try {
      sendIpcRequest("shutdown", {}).catch(() => {});
    } catch {
      // Ignore errors during shutdown
    }
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

  return new Promise((resolve, reject) => {
    const client = net.createConnection(pipeName);
    let buffer = "";

    client.on("connect", () => {
      outputChannel.appendLine("[ipc] connected");
      ipcClient = client;
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
      outputChannel.appendLine(`[ipc] error: ${err.message}`);
      ipcClient = null;
      reject(err);
    });

    client.on("close", () => {
      outputChannel.appendLine("[ipc] disconnected");
      ipcClient = null;
    });

    // Timeout
    setTimeout(() => {
      if (!ipcClient) {
        client.destroy();
        reject(new Error("IPC connection timeout"));
      }
    }, 5000);
  });
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

    pendingRequests.set(id, { resolve, reject });

    const payload = JSON.stringify(request) + "\n";
    ipcClient.write(payload);

    // Timeout after 30s
    setTimeout(() => {
      if (pendingRequests.has(id)) {
        pendingRequests.delete(id);
        reject(new Error(`IPC request timeout: ${method}`));
      }
    }, 30000);
  });
}

function derivePipeName(repoRoot: string): string {
  const crypto = require("crypto");
  const normalized = repoRoot.replace(/\\\\\?\\/, "").toLowerCase();
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
          stream.markdown(
            `*OmniContext injected ${contextResult.entries_count} code chunks ` +
              `(${contextResult.tokens_used}/${contextResult.token_budget} tokens, ` +
              `${contextResult.elapsed_ms}ms)*\n\n`,
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

      return result as PreflightResponse;
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

    const result = cp.execSync(
      `"${binary}" search "${prompt.replace(/"/g, '\\"')}" --json --limit 20`,
      { encoding: "utf-8", timeout: 10000, cwd: root },
    );

    const data = JSON.parse(result);
    if (!data.results || data.results.length === 0) return null;

    // Assemble context from CLI results
    let context = "<context_engine>\n";
    context +=
      "OmniContext has analyzed the codebase and identified the following relevant code.\n\n";
    context += "## Relevant Code\n\n";

    let tokensUsed = 0;
    let entriesCount = 0;

    for (const r of data.results) {
      const chunkTokens = Math.ceil(r.content.length / 4);
      if (tokensUsed + chunkTokens > tokenBudget) break;

      context += `### ${r.symbol} (${r.kind}, score: ${r.score.toFixed(4)})\n`;
      context += `File: ${r.file}\n`;
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
      elapsed_ms: data.elapsed_ms || 0,
    };
  } catch {
    return null;
  }
}

interface PreflightResponse {
  system_context: string;
  entries_count: number;
  tokens_used: number;
  token_budget: number;
  elapsed_ms: number;
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
      if (!silent) {
        vscode.window.showInformationMessage(
          `OmniContext: Indexed ${result.files_processed} files, ` +
            `${result.chunks_created} chunks in ${result.elapsed_ms}ms`,
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
  if (!binary || !root) return;

  if (!silent) {
    statusBarItem.text = "$(sync~spin) Indexing...";
  }

  try {
    const result = cp.execSync(`"${binary}" index "${root}" --json`, {
      encoding: "utf-8",
      timeout: 300_000,
      cwd: root,
    });

    const data = JSON.parse(result);
    statusBarItem.text = `$(search) OmniContext (${data.files_processed} files)`;

    if (!silent) {
      vscode.window.showInformationMessage(
        `OmniContext: Indexed ${data.files_processed} files, ` +
          `${data.chunks_created} chunks, ${data.symbols_extracted} symbols ` +
          `in ${data.elapsed_ms}ms`,
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

      const result = cp.execSync(
        `"${binary}" search "${query}" --json --limit 20`,
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

    const selected = await vscode.window.showQuickPick(items, {
      placeHolder: `${data.results.length} results for "${query}"`,
      matchOnDescription: true,
      matchOnDetail: true,
    });

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

      const result = cp.execSync(`"${binary}" status "${root}" --json`, {
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
  if (!binary || !root) return;

  const mcpBinary = binary.replace(/omnicontext(\.exe)?$/, "omnicontext-mcp$1");

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
