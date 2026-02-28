import * as vscode from "vscode";
import * as cp from "child_process";
import * as path from "path";

let statusBarItem: vscode.StatusBarItem;
let outputChannel: vscode.OutputChannel;

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
  );

  // Auto-index on workspace open
  const config = vscode.workspace.getConfiguration("omnicontext");
  if (config.get<boolean>("autoIndex", true)) {
    runIndex(true);
  }
}

export function deactivate() {
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

  // Try common install locations
  const candidates = [
    "omnicontext", // PATH
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

function getWorkspaceRoot(): string {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders || folders.length === 0) {
    vscode.window.showWarningMessage("No workspace folder open");
    return "";
  }
  return folders[0].uri.fsPath;
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

async function runIndex(silent: boolean = false) {
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
  const binary = getBinaryPath();
  const root = getWorkspaceRoot();
  if (!binary || !root) return;

  const query = await vscode.window.showInputBox({
    prompt: "Search your codebase",
    placeHolder:
      "e.g. authentication handler, Config::new, how does caching work?",
  });

  if (!query) return;

  try {
    const result = cp.execSync(
      `"${binary}" search "${query}" --json --limit 20`,
      { encoding: "utf-8", timeout: 30_000, cwd: root },
    );

    const data = JSON.parse(result);

    if (data.count === 0) {
      vscode.window.showInformationMessage(`No results for "${query}"`);
      return;
    }

    // Show results as quick pick
    const items = data.results.map((r: any, i: number) => ({
      label: `${i + 1}. ${r.symbol}`,
      description: `${r.kind} | score: ${r.score.toFixed(4)}`,
      detail: `${r.file}:${r.line_start}-${r.line_end}`,
      file: r.file,
      line: r.line_start,
    }));

    const selected = await vscode.window.showQuickPick(items, {
      placeHolder: `${data.count} results for "${query}" (${data.elapsed_ms}ms)`,
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
  const binary = getBinaryPath();
  const root = getWorkspaceRoot();
  if (!binary || !root) return;

  try {
    const result = cp.execSync(`"${binary}" status "${root}" --json`, {
      encoding: "utf-8",
      timeout: 10_000,
      cwd: root,
    });

    const data = JSON.parse(result);
    outputChannel.clear();
    outputChannel.appendLine("OmniContext Status");
    outputChannel.appendLine("---");
    outputChannel.appendLine(`Repository:     ${data.repo_path}`);
    outputChannel.appendLine(`Search mode:    ${data.search_mode}`);
    outputChannel.appendLine(`Files indexed:  ${data.files_indexed}`);
    outputChannel.appendLine(`Chunks indexed: ${data.chunks_indexed}`);
    outputChannel.appendLine(`Symbols indexed:${data.symbols_indexed}`);
    outputChannel.appendLine(`Vectors indexed:${data.vectors_indexed}`);
    outputChannel.appendLine(`Dep edges:      ${data.dep_edges}`);
    outputChannel.appendLine(`Graph nodes:    ${data.graph_nodes}`);
    outputChannel.appendLine(`Graph edges:    ${data.graph_edges}`);
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
