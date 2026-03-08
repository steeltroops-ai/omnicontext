/**
 * LSP-enhanced symbol extraction at cursor position.
 *
 * Uses three resolution strategies in order of precision:
 * 1. VS Code's DocumentSymbolProvider (LSP-backed, gives AST node + kind)
 * 2. VS Code's HoverProvider (LSP-backed, gives type signatures)
 * 3. VS Code's DefinitionProvider (LSP-backed, gives definition location)
 * 4. Word-at-cursor fallback (no LSP required)
 *
 * The combination produces rich context: symbol name, kind, type signature,
 * and definition file -- everything the daemon needs for precise pre-fetch.
 */

import * as vscode from "vscode";

/** Rich symbol info returned by the enhanced extraction. */
export interface SymbolInfo {
  /** Symbol name (e.g., "validate_token") */
  name: string;
  /** Fully qualified name if available (e.g., "auth::middleware::validate_token") */
  fqn?: string;
  /** Symbol kind from LSP (Function, Class, Method, etc.) */
  kind?: string;
  /** Type signature from hover (e.g., "fn validate_token(token: &str) -> Result<Claims>") */
  type_signature?: string;
  /** File where the symbol is defined (for cross-file pre-fetch) */
  definition_file?: string;
  /** Line where the symbol is defined */
  definition_line?: number;
}

export class SymbolExtractor {
  /** Cache to avoid re-querying LSP within the same debounce window. */
  private cache: Map<string, { info: SymbolInfo; timestamp: number }> =
    new Map();
  private readonly CACHE_TTL_MS = 500; // 500ms TTL matches cursor debounce

  /**
   * Extract rich symbol info at cursor position using LSP.
   *
   * Returns a full SymbolInfo object with name, kind, type signature,
   * and definition location when LSP data is available.
   */
  public async getSymbolInfoAtPosition(
    document: vscode.TextDocument,
    position: vscode.Position,
  ): Promise<SymbolInfo | undefined> {
    const cacheKey = `${document.uri.fsPath}:${position.line}:${position.character}`;
    const cached = this.cache.get(cacheKey);
    if (cached && Date.now() - cached.timestamp < this.CACHE_TTL_MS) {
      return cached.info;
    }

    const info = await this.resolveSymbolInfo(document, position);
    if (info) {
      this.cache.set(cacheKey, { info, timestamp: Date.now() });
      // Evict old cache entries to prevent memory leak
      if (this.cache.size > 200) {
        const oldest = this.cache.keys().next().value;
        if (oldest) this.cache.delete(oldest);
      }
    }
    return info;
  }

  /**
   * Legacy API: extract symbol name at cursor position.
   * Kept for backward compatibility with existing eventTracker.
   */
  public async getSymbolAtPosition(
    document: vscode.TextDocument,
    position: vscode.Position,
  ): Promise<string | undefined> {
    const info = await this.getSymbolInfoAtPosition(document, position);
    return info?.name;
  }

  /**
   * Core resolution: chains DocumentSymbol -> Hover -> Definition providers.
   */
  private async resolveSymbolInfo(
    document: vscode.TextDocument,
    position: vscode.Position,
  ): Promise<SymbolInfo | undefined> {
    let name: string | undefined;
    let kind: string | undefined;
    let fqn: string | undefined;
    let typeSignature: string | undefined;
    let definitionFile: string | undefined;
    let definitionLine: number | undefined;

    // --- Strategy 1: DocumentSymbolProvider (AST node + kind) ---
    try {
      const symbols = await vscode.commands.executeCommand<
        vscode.DocumentSymbol[]
      >("vscode.executeDocumentSymbolProvider", document.uri);

      if (symbols && symbols.length > 0) {
        const symbol = this.findSymbolAtPosition(symbols, position);
        if (symbol) {
          name = symbol.name;
          kind = vscode.SymbolKind[symbol.kind];
          // Build FQN from parent chain
          fqn = this.buildFqn(symbols, position);
        }
      }
    } catch {
      // LSP not ready or unavailable -- continue with other strategies
    }

    // --- Strategy 2: HoverProvider (type signatures) ---
    try {
      const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
        "vscode.executeHoverProvider",
        document.uri,
        position,
      );

      if (hovers && hovers.length > 0) {
        for (const hover of hovers) {
          for (const content of hover.contents) {
            const text =
              content instanceof vscode.MarkdownString
                ? content.value
                : typeof content === "string"
                  ? content
                  : "";
            // Extract code block content (type signatures from LSP)
            const codeMatch = text.match(/```\w*\n([\s\S]*?)\n```/);
            if (codeMatch) {
              typeSignature = this.limitLength(codeMatch[1].trim(), 200);
              // If we didn't get a name from symbols, extract from signature
              if (!name) {
                name = this.extractNameFromSignature(typeSignature);
              }
              break;
            }
          }
          if (typeSignature) break;
        }
      }
    } catch {
      // Hover provider not available
    }

    // --- Strategy 3: DefinitionProvider (cross-file location) ---
    try {
      const definitions = await vscode.commands.executeCommand<
        vscode.Location[] | vscode.LocationLink[]
      >("vscode.executeDefinitionProvider", document.uri, position);

      if (definitions && definitions.length > 0) {
        const def = definitions[0];
        if ("targetUri" in def) {
          // LocationLink
          definitionFile = def.targetUri.fsPath;
          definitionLine = def.targetRange.start.line;
        } else if ("uri" in def) {
          // Location
          definitionFile = def.uri.fsPath;
          definitionLine = def.range.start.line;
        }
      }
    } catch {
      // Definition provider not available
    }

    // --- Strategy 4: Word fallback ---
    if (!name) {
      name = this.getWordAtPosition(document, position);
    }

    if (!name) return undefined;

    return {
      name: this.limitLength(name, 100),
      fqn: fqn ? this.limitLength(fqn, 200) : undefined,
      kind,
      type_signature: typeSignature,
      definition_file: definitionFile,
      definition_line: definitionLine,
    };
  }

  /**
   * Build FQN by walking the DocumentSymbol tree and collecting parent names.
   */
  private buildFqn(
    symbols: vscode.DocumentSymbol[],
    position: vscode.Position,
  ): string | undefined {
    const path: string[] = [];
    this.collectFqnPath(symbols, position, path);
    return path.length > 0 ? path.join("::") : undefined;
  }

  private collectFqnPath(
    symbols: vscode.DocumentSymbol[],
    position: vscode.Position,
    path: string[],
  ): boolean {
    for (const symbol of symbols) {
      if (symbol.range.contains(position)) {
        path.push(symbol.name);
        if (symbol.children && symbol.children.length > 0) {
          this.collectFqnPath(symbol.children, position, path);
        }
        return true;
      }
    }
    return false;
  }

  /**
   * Extract a function/method name from a type signature string.
   */
  private extractNameFromSignature(sig: string): string | undefined {
    // Rust: fn name(
    const rustMatch = sig.match(/fn\s+(\w+)/);
    if (rustMatch) return rustMatch[1];

    // TypeScript/JS: function name( or name(
    const tsMatch = sig.match(/(?:function\s+)?(\w+)\s*\(/);
    if (tsMatch) return tsMatch[1];

    // Python: def name(
    const pyMatch = sig.match(/def\s+(\w+)/);
    if (pyMatch) return pyMatch[1];

    // class Name
    const classMatch = sig.match(/class\s+(\w+)/);
    if (classMatch) return classMatch[1];

    return undefined;
  }

  /**
   * Recursively find the most specific symbol at the given position.
   */
  private findSymbolAtPosition(
    symbols: vscode.DocumentSymbol[],
    position: vscode.Position,
  ): vscode.DocumentSymbol | undefined {
    for (const symbol of symbols) {
      if (symbol.range.contains(position)) {
        if (symbol.children && symbol.children.length > 0) {
          const child = this.findSymbolAtPosition(symbol.children, position);
          if (child) return child;
        }
        return symbol;
      }
    }
    return undefined;
  }

  /**
   * Fallback: extract word at cursor position.
   */
  private getWordAtPosition(
    document: vscode.TextDocument,
    position: vscode.Position,
  ): string | undefined {
    const range = document.getWordRangeAtPosition(position);
    if (!range) return undefined;
    return document.getText(range);
  }

  /**
   * Limit string length.
   */
  private limitLength(s: string, max: number = 100): string {
    return s.length > max ? s.substring(0, max) : s;
  }
}
