/**
 * Symbol extraction at cursor position.
 * Uses VS Code's language features with fallback to word extraction.
 */

import * as vscode from 'vscode';

export class SymbolExtractor {
    /**
     * Extract symbol at cursor position.
     * First tries VS Code's document symbol provider, then falls back to word extraction.
     */
    public async getSymbolAtPosition(
        document: vscode.TextDocument,
        position: vscode.Position
    ): Promise<string | undefined> {
        try {
            // Try VS Code's built-in symbol provider
            const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
                'vscode.executeDocumentSymbolProvider',
                document.uri
            );

            if (symbols && symbols.length > 0) {
                const symbol = this.findSymbolAtPosition(symbols, position);
                if (symbol) {
                    return this.limitLength(symbol.name);
                }
            }
        } catch (err) {
            // Fall through to word extraction
            console.debug('Symbol provider failed, falling back to word extraction:', err);
        }

        // Fallback: extract word at cursor
        return this.getWordAtPosition(document, position);
    }

    /**
     * Recursively find the most specific symbol at the given position.
     */
    private findSymbolAtPosition(
        symbols: vscode.DocumentSymbol[],
        position: vscode.Position
    ): vscode.DocumentSymbol | undefined {
        for (const symbol of symbols) {
            if (symbol.range.contains(position)) {
                // Check children first (more specific)
                if (symbol.children && symbol.children.length > 0) {
                    const child = this.findSymbolAtPosition(symbol.children, position);
                    if (child) {
                        return child;
                    }
                }
                // Return this symbol if no more specific child found
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
        position: vscode.Position
    ): string | undefined {
        const range = document.getWordRangeAtPosition(position);
        if (!range) {
            return undefined;
        }

        const word = document.getText(range);
        return this.limitLength(word);
    }

    /**
     * Limit symbol length to 100 characters.
     */
    private limitLength(symbol: string): string {
        if (symbol.length > 100) {
            return symbol.substring(0, 100);
        }
        return symbol;
    }
}
