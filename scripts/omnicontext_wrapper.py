#!/usr/bin/env python3
"""
Programmatic wrapper for OmniContext MCP server.

This script provides controlled access to omnicontext tools without
automatically injecting full response bodies into context. You can:
- Filter and summarize responses
- Chain multiple tool calls
- Extract only relevant information
- Control token usage precisely
"""

import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional


class OmniContextWrapper:
    """Wrapper for OmniContext MCP server with controlled output."""
    
    def __init__(self, repo_path: str, mcp_exe_path: Optional[str] = None):
        self.repo_path = Path(repo_path).resolve()
        
        # Auto-detect MCP executable
        if mcp_exe_path:
            self.mcp_exe = Path(mcp_exe_path)
        else:
            # Try common locations
            home = Path.home()
            candidates = [
                home / ".omnicontext" / "bin" / "omnicontext-mcp.exe",
                home / ".omnicontext" / "bin" / "omnicontext-mcp",
                Path("target/release/omnicontext-mcp.exe"),
                Path("target/release/omnicontext-mcp"),
            ]
            self.mcp_exe = next((p for p in candidates if p.exists()), None)
            
        if not self.mcp_exe or not self.mcp_exe.exists():
            raise FileNotFoundError(f"OmniContext MCP executable not found")
    
    def _call_tool(self, tool_name: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Call an MCP tool and return raw response."""
        cmd = [str(self.mcp_exe), "--repo", str(self.repo_path)]
        
        # Create MCP request
        request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": params
            }
        }
        
        try:
            result = subprocess.run(
                cmd,
                input=json.dumps(request),
                capture_output=True,
                text=True,
                timeout=30
            )
            
            if result.returncode != 0:
                return {"error": f"Process failed: {result.stderr}"}
            
            response = json.loads(result.stdout)
            return response.get("result", {})
        except subprocess.TimeoutExpired:
            return {"error": "Tool call timed out"}
        except json.JSONDecodeError as e:
            return {"error": f"Invalid JSON response: {e}"}
        except Exception as e:
            return {"error": f"Tool call failed: {e}"}
    
    # ===== High-level API with controlled output =====
    
    def get_status_summary(self) -> str:
        """Get concise status summary (< 200 tokens)."""
        result = self._call_tool("get_status", {})
        
        if "error" in result:
            return f"Error: {result['error']}"
        
        # Extract only key metrics
        content = result.get("content", [{}])[0].get("text", "")
        lines = content.split("\n")
        
        summary = []
        for line in lines:
            if any(k in line for k in ["Files:", "Chunks:", "Symbols:", "Vectors:", "Search mode:"]):
                summary.append(line.strip())
        
        return "\n".join(summary[:5])  # Max 5 lines
    
    def search_code_filtered(
        self, 
        query: str, 
        limit: int = 5,
        max_tokens_per_result: int = 200
    ) -> List[Dict[str, str]]:
        """Search code and return filtered results."""
        result = self._call_tool("search_code", {"query": query, "limit": limit})
        
        if "error" in result:
            return [{"error": result["error"]}]
        
        content = result.get("content", [{}])[0].get("text", "")
        
        # Parse and filter results
        filtered = []
        current_result = {}
        
        for line in content.split("\n"):
            if line.startswith("**") and "**" in line[2:]:
                # New result
                if current_result:
                    filtered.append(current_result)
                current_result = {"title": line.strip("* ")}
            elif ":" in line and current_result:
                key, value = line.split(":", 1)
                current_result[key.strip().lower()] = value.strip()
            elif line.strip().startswith("```") and current_result:
                # Truncate code snippets
                current_result["code_preview"] = "[code truncated]"
        
        if current_result:
            filtered.append(current_result)
        
        return filtered[:limit]
    
    def get_symbol_info(self, symbol_name: str) -> Dict[str, Any]:
        """Get symbol info with minimal context."""
        result = self._call_tool("get_symbol", {"name": symbol_name, "limit": 3})
        
        if "error" in result:
            return {"error": result["error"]}
        
        content = result.get("content", [{}])[0].get("text", "")
        
        # Extract just the symbol list, not full code
        symbols = []
        for line in content.split("\n"):
            if line.startswith("- **"):
                symbols.append(line.strip("- "))
        
        return {
            "query": symbol_name,
            "found": len(symbols),
            "symbols": symbols[:3]  # Max 3 results
        }
    
    def get_architecture_summary(self) -> str:
        """Get high-level architecture (< 500 tokens)."""
        result = self._call_tool("get_architecture", {})
        
        if "error" in result:
            return f"Error: {result['error']}"
        
        content = result.get("content", [{}])[0].get("text", "")
        
        # Extract only summary sections, skip detailed listings
        lines = content.split("\n")
        summary = []
        skip_section = False
        
        for line in lines:
            if line.startswith("##"):
                skip_section = "Indexed Content" in line or "Recommendations" in line
            
            if not skip_section and (line.startswith("#") or line.startswith("-") or ":" in line):
                summary.append(line)
        
        return "\n".join(summary[:20])  # Max 20 lines
    
    def find_patterns_summary(self, pattern: str, limit: int = 3) -> str:
        """Find patterns with minimal output."""
        result = self._call_tool("find_patterns", {"pattern": pattern, "limit": limit})
        
        if "error" in result:
            return f"Error: {result['error']}"
        
        content = result.get("content", [{}])[0].get("text", "")
        
        # Return just file paths and line numbers, not full code
        summary = []
        for line in content.split("\n"):
            if "file:" in line.lower() or "line:" in line.lower():
                summary.append(line.strip())
        
        return "\n".join(summary[:10])  # Max 10 lines
    
    def get_dependencies_graph(self, symbol: str, direction: str = "both") -> Dict[str, List[str]]:
        """Get dependency graph in compact format."""
        result = self._call_tool("get_dependencies", {
            "symbol": symbol,
            "direction": direction
        })
        
        if "error" in result:
            return {"error": [result["error"]]}
        
        content = result.get("content", [{}])[0].get("text", "")
        
        # Parse into structured format
        deps = {"upstream": [], "downstream": []}
        current_section = None
        
        for line in content.split("\n"):
            if "Upstream" in line:
                current_section = "upstream"
            elif "Downstream" in line:
                current_section = "downstream"
            elif line.startswith("- ") and current_section:
                deps[current_section].append(line.strip("- `"))
        
        return deps
    
    def context_window_compact(
        self, 
        query: str, 
        token_budget: int = 2000
    ) -> str:
        """Get context window with strict token budget."""
        result = self._call_tool("context_window", {
            "query": query,
            "limit": 10,
            "token_budget": token_budget
        })
        
        if "error" in result:
            return f"Error: {result['error']}"
        
        content = result.get("content", [{}])[0].get("text", "")
        
        # Truncate to budget (rough estimate: 4 chars = 1 token)
        max_chars = token_budget * 4
        if len(content) > max_chars:
            content = content[:max_chars] + "\n\n[... truncated to fit token budget]"
        
        return content
    
    # ===== Chained operations =====
    
    def analyze_symbol_full(self, symbol_name: str) -> Dict[str, Any]:
        """Chain multiple calls to fully analyze a symbol."""
        analysis = {
            "symbol": symbol_name,
            "info": self.get_symbol_info(symbol_name),
            "dependencies": self.get_dependencies_graph(symbol_name),
        }
        
        # Only fetch context if symbol was found
        if analysis["info"].get("found", 0) > 0:
            analysis["context"] = self.context_window_compact(
                symbol_name, 
                token_budget=1000
            )
        
        return analysis
    
    def search_and_analyze(self, query: str, top_n: int = 3) -> Dict[str, Any]:
        """Search and analyze top results in one call."""
        results = self.search_code_filtered(query, limit=top_n)
        
        analysis = {
            "query": query,
            "results_found": len(results),
            "top_results": results,
        }
        
        # Get architecture context for the query
        analysis["architecture"] = self.get_architecture_summary()
        
        return analysis
    
    def health_check(self) -> Dict[str, Any]:
        """Quick health check of the index."""
        status = self.get_status_summary()
        
        # Parse key metrics
        metrics = {}
        for line in status.split("\n"):
            if ":" in line:
                key, value = line.split(":", 1)
                metrics[key.strip().lower()] = value.strip()
        
        return {
            "healthy": "error" not in status.lower(),
            "metrics": metrics,
            "raw_status": status
        }


def main():
    """CLI interface for the wrapper."""
    if len(sys.argv) < 3:
        print("Usage: omnicontext_wrapper.py <repo_path> <command> [args...]")
        print("\nCommands:")
        print("  status              - Get status summary")
        print("  search <query>      - Search code")
        print("  symbol <name>       - Get symbol info")
        print("  deps <symbol>       - Get dependencies")
        print("  analyze <symbol>    - Full symbol analysis")
        print("  health              - Health check")
        sys.exit(1)
    
    repo_path = sys.argv[1]
    command = sys.argv[2]
    
    wrapper = OmniContextWrapper(repo_path)
    
    if command == "status":
        print(wrapper.get_status_summary())
    
    elif command == "search" and len(sys.argv) > 3:
        query = " ".join(sys.argv[3:])
        results = wrapper.search_code_filtered(query)
        print(json.dumps(results, indent=2))
    
    elif command == "symbol" and len(sys.argv) > 3:
        symbol = sys.argv[3]
        info = wrapper.get_symbol_info(symbol)
        print(json.dumps(info, indent=2))
    
    elif command == "deps" and len(sys.argv) > 3:
        symbol = sys.argv[3]
        deps = wrapper.get_dependencies_graph(symbol)
        print(json.dumps(deps, indent=2))
    
    elif command == "analyze" and len(sys.argv) > 3:
        symbol = sys.argv[3]
        analysis = wrapper.analyze_symbol_full(symbol)
        print(json.dumps(analysis, indent=2))
    
    elif command == "health":
        health = wrapper.health_check()
        print(json.dumps(health, indent=2))
    
    else:
        print(f"Unknown command: {command}")
        sys.exit(1)


if __name__ == "__main__":
    main()
