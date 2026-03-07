#!/usr/bin/env node
/**
 * Programmatic wrapper for OmniContext MCP server (TypeScript/Node.js version)
 * 
 * Provides controlled access to omnicontext tools without automatically
 * injecting full response bodies into context.
 */

import { spawn } from 'child_process';
import { existsSync } from 'fs';
import { homedir } from 'os';
import { join, resolve } from 'path';

interface MCPRequest {
    jsonrpc: string;
    id: number;
    method: string;
    params: {
        name: string;
        arguments: Record<string, any>;
    };
}

interface MCPResponse {
    result?: {
        content?: Array<{ text: string }>;
        error?: string;
    };
    error?: any;
}

interface SearchResult {
    title?: string;
    file?: string;
    line?: string;
    score?: string;
    code_preview?: string;
}

interface SymbolInfo {
    query: string;
    found: number;
    symbols: string[];
}

interface DependencyGraph {
    upstream: string[];
    downstream: string[];
}

export class OmniContextWrapper {
    private repoPath: string;
    private mcpExe: string;

    constructor(repoPath: string, mcpExePath?: string) {
        this.repoPath = resolve(repoPath);

        if (mcpExePath) {
            this.mcpExe = mcpExePath;
        } else {
            // Auto-detect MCP executable
            const home = homedir();
            const candidates = [
                join(home, '.omnicontext', 'bin', 'omnicontext-mcp.exe'),
                join(home, '.omnicontext', 'bin', 'omnicontext-mcp'),
                join('target', 'release', 'omnicontext-mcp.exe'),
                join('target', 'release', 'omnicontext-mcp'),
            ];

            const found = candidates.find(p => existsSync(p));
            if (!found) {
                throw new Error('OmniContext MCP executable not found');
            }
            this.mcpExe = found;
        }
    }

    private async callTool(toolName: string, params: Record<string, any>): Promise<any> {
        const request: MCPRequest = {
            jsonrpc: '2.0',
            id: 1,
            method: 'tools/call',
            params: {
                name: toolName,
                arguments: params,
            },
        };

        return new Promise((resolve, reject) => {
            const proc = spawn(this.mcpExe, ['--repo', this.repoPath]);

            let stdout = '';
            let stderr = '';

            proc.stdout.on('data', (data) => {
                stdout += data.toString();
            });

            proc.stderr.on('data', (data) => {
                stderr += data.toString();
            });

            proc.on('close', (code) => {
                if (code !== 0) {
                    reject(new Error(`Process failed: ${stderr}`));
                    return;
                }

                try {
                    const response: MCPResponse = JSON.parse(stdout);
                    resolve(response.result || {});
                } catch (e) {
                    reject(new Error(`Invalid JSON response: ${e}`));
                }
            });

            proc.on('error', (err) => {
                reject(err);
            });

            // Send request
            proc.stdin.write(JSON.stringify(request));
            proc.stdin.end();

            // Timeout after 30 seconds
            setTimeout(() => {
                proc.kill();
                reject(new Error('Tool call timed out'));
            }, 30000);
        });
    }

    /**
     * Get concise status summary (< 200 tokens)
     */
    async getStatusSummary(): Promise<string> {
        try {
            const result = await this.callTool('get_status', {});
            const content = result.content?.[0]?.text || '';

            const lines = content.split('\n');
            const summary = lines
                .filter(line =>
                    ['Files:', 'Chunks:', 'Symbols:', 'Vectors:', 'Search mode:']
                        .some(k => line.includes(k))
                )
                .slice(0, 5);

            return summary.join('\n');
        } catch (error) {
            return `Error: ${error}`;
        }
    }

    /**
     * Search code and return filtered results
     */
    async searchCodeFiltered(
        query: string,
        limit: number = 5,
        maxTokensPerResult: number = 200
    ): Promise<SearchResult[]> {
        try {
            const result = await this.callTool('search_code', { query, limit });
            const content = result.content?.[0]?.text || '';

            const filtered: SearchResult[] = [];
            let currentResult: SearchResult = {};

            for (const line of content.split('\n')) {
                if (line.startsWith('**') && line.includes('**', 2)) {
                    if (Object.keys(currentResult).length > 0) {
                        filtered.push(currentResult);
                    }
                    currentResult = { title: line.replace(/\*\*/g, '').trim() };
                } else if (line.includes(':') && Object.keys(currentResult).length > 0) {
                    const [key, ...valueParts] = line.split(':');
                    const value = valueParts.join(':').trim();
                    currentResult[key.trim().toLowerCase()] = value;
                } else if (line.trim().startsWith('```') && Object.keys(currentResult).length > 0) {
                    currentResult.code_preview = '[code truncated]';
                }
            }

            if (Object.keys(currentResult).length > 0) {
                filtered.push(currentResult);
            }

            return filtered.slice(0, limit);
        } catch (error) {
            return [{ title: `Error: ${error}` }];
        }
    }

    /**
     * Get symbol info with minimal context
     */
    async getSymbolInfo(symbolName: string): Promise<SymbolInfo> {
        try {
            const result = await this.callTool('get_symbol', { name: symbolName, limit: 3 });
            const content = result.content?.[0]?.text || '';

            const symbols = content
                .split('\n')
                .filter(line => line.startsWith('- **'))
                .map(line => line.replace(/^- \*\*/, '').replace(/\*\*.*$/, '').trim())
                .slice(0, 3);

            return {
                query: symbolName,
                found: symbols.length,
                symbols,
            };
        } catch (error) {
            return {
                query: symbolName,
                found: 0,
                symbols: [`Error: ${error}`],
            };
        }
    }

    /**
     * Get high-level architecture (< 500 tokens)
     */
    async getArchitectureSummary(): Promise<string> {
        try {
            const result = await this.callTool('get_architecture', {});
            const content = result.content?.[0]?.text || '';

            const lines = content.split('\n');
            const summary: string[] = [];
            let skipSection = false;

            for (const line of lines) {
                if (line.startsWith('##')) {
                    skipSection = line.includes('Indexed Content') || line.includes('Recommendations');
                }

                if (!skipSection && (line.startsWith('#') || line.startsWith('-') || line.includes(':'))) {
                    summary.push(line);
                }
            }

            return summary.slice(0, 20).join('\n');
        } catch (error) {
            return `Error: ${error}`;
        }
    }

    /**
     * Find patterns with minimal output
     */
    async findPatternsSummary(pattern: string, limit: number = 3): Promise<string> {
        try {
            const result = await this.callTool('find_patterns', { pattern, limit });
            const content = result.content?.[0]?.text || '';

            const summary = content
                .split('\n')
                .filter(line => line.toLowerCase().includes('file:') || line.toLowerCase().includes('line:'))
                .slice(0, 10);

            return summary.join('\n');
        } catch (error) {
            return `Error: ${error}`;
        }
    }

    /**
     * Get dependency graph in compact format
     */
    async getDependenciesGraph(symbol: string, direction: string = 'both'): Promise<DependencyGraph> {
        try {
            const result = await this.callTool('get_dependencies', { symbol, direction });
            const content = result.content?.[0]?.text || '';

            const deps: DependencyGraph = { upstream: [], downstream: [] };
            let currentSection: 'upstream' | 'downstream' | null = null;

            for (const line of content.split('\n')) {
                if (line.includes('Upstream')) {
                    currentSection = 'upstream';
                } else if (line.includes('Downstream')) {
                    currentSection = 'downstream';
                } else if (line.startsWith('- ') && currentSection) {
                    deps[currentSection].push(line.replace(/^- `?/, '').replace(/`?$/, '').trim());
                }
            }

            return deps;
        } catch (error) {
            return { upstream: [`Error: ${error}`], downstream: [] };
        }
    }

    /**
     * Get context window with strict token budget
     */
    async contextWindowCompact(query: string, tokenBudget: number = 2000): Promise<string> {
        try {
            const result = await this.callTool('context_window', {
                query,
                limit: 10,
                token_budget: tokenBudget,
            });

            let content = result.content?.[0]?.text || '';

            // Truncate to budget (rough estimate: 4 chars = 1 token)
            const maxChars = tokenBudget * 4;
            if (content.length > maxChars) {
                content = content.substring(0, maxChars) + '\n\n[... truncated to fit token budget]';
            }

            return content;
        } catch (error) {
            return `Error: ${error}`;
        }
    }

    /**
     * Chain multiple calls to fully analyze a symbol
     */
    async analyzeSymbolFull(symbolName: string): Promise<any> {
        const info = await this.getSymbolInfo(symbolName);
        const dependencies = await this.getDependenciesGraph(symbolName);

        const analysis: any = {
            symbol: symbolName,
            info,
            dependencies,
        };

        // Only fetch context if symbol was found
        if (info.found > 0) {
            analysis.context = await this.contextWindowCompact(symbolName, 1000);
        }

        return analysis;
    }

    /**
     * Search and analyze top results in one call
     */
    async searchAndAnalyze(query: string, topN: number = 3): Promise<any> {
        const results = await this.searchCodeFiltered(query, topN);
        const architecture = await this.getArchitectureSummary();

        return {
            query,
            results_found: results.length,
            top_results: results,
            architecture,
        };
    }

    /**
     * Quick health check of the index
     */
    async healthCheck(): Promise<any> {
        const status = await this.getStatusSummary();

        const metrics: Record<string, string> = {};
        for (const line of status.split('\n')) {
            if (line.includes(':')) {
                const [key, value] = line.split(':');
                metrics[key.trim().toLowerCase()] = value.trim();
            }
        }

        return {
            healthy: !status.toLowerCase().includes('error'),
            metrics,
            raw_status: status,
        };
    }
}

// CLI interface
async function main() {
    const args = process.argv.slice(2);

    if (args.length < 2) {
        console.log('Usage: omnicontext_wrapper.ts <repo_path> <command> [args...]');
        console.log('\nCommands:');
        console.log('  status              - Get status summary');
        console.log('  search <query>      - Search code');
        console.log('  symbol <name>       - Get symbol info');
        console.log('  deps <symbol>       - Get dependencies');
        console.log('  analyze <symbol>    - Full symbol analysis');
        console.log('  health              - Health check');
        process.exit(1);
    }

    const [repoPath, command, ...cmdArgs] = args;
    const wrapper = new OmniContextWrapper(repoPath);

    try {
        switch (command) {
            case 'status':
                console.log(await wrapper.getStatusSummary());
                break;

            case 'search':
                if (cmdArgs.length === 0) {
                    console.error('Error: search requires a query');
                    process.exit(1);
                }
                const searchResults = await wrapper.searchCodeFiltered(cmdArgs.join(' '));
                console.log(JSON.stringify(searchResults, null, 2));
                break;

            case 'symbol':
                if (cmdArgs.length === 0) {
                    console.error('Error: symbol requires a name');
                    process.exit(1);
                }
                const symbolInfo = await wrapper.getSymbolInfo(cmdArgs[0]);
                console.log(JSON.stringify(symbolInfo, null, 2));
                break;

            case 'deps':
                if (cmdArgs.length === 0) {
                    console.error('Error: deps requires a symbol');
                    process.exit(1);
                }
                const deps = await wrapper.getDependenciesGraph(cmdArgs[0]);
                console.log(JSON.stringify(deps, null, 2));
                break;

            case 'analyze':
                if (cmdArgs.length === 0) {
                    console.error('Error: analyze requires a symbol');
                    process.exit(1);
                }
                const analysis = await wrapper.analyzeSymbolFull(cmdArgs[0]);
                console.log(JSON.stringify(analysis, null, 2));
                break;

            case 'health':
                const health = await wrapper.healthCheck();
                console.log(JSON.stringify(health, null, 2));
                break;

            default:
                console.error(`Unknown command: ${command}`);
                process.exit(1);
        }
    } catch (error) {
        console.error(`Error: ${error}`);
        process.exit(1);
    }
}

if (require.main === module) {
    main();
}
