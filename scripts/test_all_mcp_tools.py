#!/usr/bin/env python3
"""
Comprehensive test of all 19 OmniContext MCP tools.
Launches the MCP server, initializes the protocol, and calls each tool sequentially.
"""

import subprocess, json, sys, time, os, threading, queue

MCP_BIN = os.path.expanduser("~/.omnicontext/bin/omnicontext-mcp.exe")
REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

BACKSLASH = "\\"

class McpClient:
    def __init__(self):
        self.proc = subprocess.Popen(
            [MCP_BIN, "--repo", REPO, "--no-auto-index"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=0
        )
        self.responses = queue.Queue()
        self._reader = threading.Thread(target=self._read_stdout, daemon=True)
        self._reader.start()

    def _read_stdout(self):
        """Background thread - streaming JSON parser for concatenated objects."""
        buf = b""
        depth = 0
        in_string = False
        escape_next = False

        while True:
            try:
                ch = self.proc.stdout.read(1)
                if not ch:
                    break
                buf += ch

                c = chr(ch[0])

                if escape_next:
                    escape_next = False
                    continue

                if c == BACKSLASH and in_string:
                    escape_next = True
                    continue

                if c == '"':
                    in_string = not in_string
                    continue

                if in_string:
                    continue

                if c == '{':
                    depth += 1
                elif c == '}':
                    depth -= 1
                    if depth == 0:
                        try:
                            obj = json.loads(buf.decode('utf-8'))
                            self.responses.put(obj)
                        except json.JSONDecodeError:
                            pass
                        buf = b""
            except Exception:
                break

    def send(self, msg):
        data = json.dumps(msg).encode('utf-8') + b"\n"
        self.proc.stdin.write(data)
        self.proc.stdin.flush()

    def recv(self, timeout=120):
        try:
            return self.responses.get(timeout=timeout)
        except queue.Empty:
            return None

    def initialize(self):
        self.send({
            "jsonrpc": "2.0", "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        })
        resp = self.recv(timeout=120)
        self.send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
        time.sleep(0.5)
        return resp

    def call_tool(self, tool_name, arguments=None, msg_id=2, timeout=120):
        self.send({
            "jsonrpc": "2.0", "id": msg_id,
            "method": "tools/call",
            "params": {"name": tool_name, "arguments": arguments or {}}
        })
        start = time.time()
        while time.time() - start < timeout:
            remaining = timeout - (time.time() - start)
            resp = self.recv(timeout=max(remaining, 0.1))
            if resp and resp.get("id") == msg_id:
                return resp
        return None

    def close(self):
        try:
            self.proc.stdin.close()
            self.proc.terminate()
            self.proc.wait(timeout=5)
        except Exception:
            try:
                self.proc.kill()
            except Exception:
                pass


def main():
    print("Starting MCP server...", flush=True)
    client = McpClient()

    print("Initializing...", flush=True)
    init = client.initialize()
    if init and "result" in init:
        info = init["result"].get("serverInfo", {})
        print(f"Server: {info.get('name')} v{info.get('version')}")
    else:
        print(f"Init failed: {init}")
        client.close()
        sys.exit(1)

    # All 19 tools with test arguments
    TOOLS = [
        ("get_status", {}),
        ("search_code", {"query": "Engine", "limit": 3}),
        ("get_symbol", {"name": "Engine"}),
        ("get_file_summary", {"path": "crates/omni-core/src/lib.rs"}),
        ("get_dependencies", {"symbol": "Engine", "depth": 1}),
        ("find_patterns", {"pattern": "error handling", "limit": 3}),
        ("get_architecture", {}),
        ("get_module_map", {}),
        ("context_window", {"query": "search engine", "max_tokens": 2000}),
        ("search_by_intent", {"query": "how does indexing work", "limit": 3}),
        ("get_blast_radius", {"symbol": "Engine"}),
        ("get_recent_changes", {"limit": 5}),
        ("get_call_graph", {"symbol": "Engine::search"}),
        ("get_branch_context", {}),
        ("get_co_changes", {"file_path": "crates/omni-core/src/lib.rs", "limit": 5}),
        ("audit_plan", {"plan": "Refactor the search module to add caching"}),
        ("explain_codebase", {}),
        ("generate_manifest", {"format": "claude"}),
        ("set_workspace", {"path": REPO}),
    ]

    print(f"\n{'='*60}")
    print(f"TESTING {len(TOOLS)} MCP TOOLS SEQUENTIALLY")
    print(f"{'='*60}\n")

    passed = 0
    failed = 0
    results = {}

    for i, (tool_name, args) in enumerate(TOOLS):
        label = f"[{i+1:2d}/{len(TOOLS)}] {tool_name}"
        sys.stdout.write(f"{label}... ")
        sys.stdout.flush()

        try:
            resp = client.call_tool(tool_name, args, msg_id=i + 2, timeout=120)

            if resp is None:
                print("FAIL (timeout)")
                results[tool_name] = "FAIL: timeout"
                failed += 1
            elif "error" in resp:
                err = resp["error"]
                msg = err.get("message", str(err)) if isinstance(err, dict) else str(err)
                print(f"FAIL ({msg[:70]})")
                results[tool_name] = f"FAIL: {msg[:100]}"
                failed += 1
            elif "result" in resp:
                content = resp["result"].get("content", [])
                if content:
                    text = content[0].get("text", "")
                    if len(text) > 0:
                        print(f"OK ({len(text)} chars)")
                        results[tool_name] = f"OK: {len(text)} chars"
                        passed += 1
                    else:
                        print("FAIL (empty)")
                        results[tool_name] = "FAIL: empty"
                        failed += 1
                else:
                    print("WARN (no content)")
                    results[tool_name] = "WARN: no content"
                    failed += 1
            else:
                print("FAIL (unexpected response)")
                results[tool_name] = "FAIL: unexpected"
                failed += 1
        except Exception as e:
            print(f"ERROR ({e})")
            results[tool_name] = f"ERROR: {e}"
            failed += 1

    print(f"\n{'='*60}")
    print(f"RESULTS: {passed} PASSED, {failed} FAILED / {len(TOOLS)} total")
    print(f"{'='*60}")
    for tool, status in results.items():
        marker = "PASS" if "OK" in status else "FAIL"
        print(f"  [{marker}] {tool}: {status}")
    print(f"\nSuccess rate: {passed}/{len(TOOLS)} ({100*passed/len(TOOLS):.0f}%)")

    client.close()
    print("Server shut down.")
    return passed == len(TOOLS)


if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
