import json
import os
import queue
import subprocess
import threading
from pathlib import Path

repo = Path(r"c:/Omniverse/Projects/omnicontext")
candidates = [
    repo / "target" / "debug" / "omnicontext-mcp.exe",
    repo / "target" / "release" / "omnicontext-mcp.exe",
    Path.home() / ".omnicontext" / "bin" / "omnicontext-mcp.exe",
]
mcp_bin = next((c for c in candidates if c.exists()), None)
if mcp_bin is None:
    print(json.dumps({"fatal": "mcp binary not found"}, indent=2))
    raise SystemExit(1)

tools = [
    ("search_code", {"query": "Engine", "limit": 3}),
    ("context_window", {"query": "Engine", "limit": 5, "token_budget": 1200}),
    ("get_symbol", {"name": "Engine", "limit": 3}),
    ("get_file_summary", {"path": "crates/omni-core/src/pipeline/mod.rs"}),
    ("get_status", {}),
    ("get_dependencies", {"symbol": "omni_core::pipeline::Engine", "direction": "both"}),
    ("find_patterns", {"pattern": "error handling", "limit": 3}),
    ("get_architecture", {}),
    ("explain_codebase", {}),
    ("get_module_map", {"max_depth": 2}),
    ("search_by_intent", {"query": "how indexing works", "limit": 3, "token_budget": 1600}),
    ("set_workspace", {"path": str(repo), "auto_index": False}),
    ("get_blast_radius", {"symbol": "omni_core::pipeline::Engine", "max_depth": 2}),
    ("get_recent_changes", {"commit_count": 3, "include_diff": False}),
    ("get_call_graph", {"symbol": "omni_core::pipeline::Engine", "depth": 1, "mermaid": False}),
    ("get_branch_context", {"include_diffs": False}),
]


def read_line_with_timeout(stream, timeout_sec):
    q = queue.Queue(maxsize=1)

    def _reader():
        try:
            q.put(stream.readline())
        except Exception as exc:
            q.put(exc)

    thread = threading.Thread(target=_reader, daemon=True)
    thread.start()

    try:
        value = q.get(timeout=timeout_sec)
    except queue.Empty:
        return None, "timeout"

    if isinstance(value, Exception):
        return None, str(value)

    text = value.strip()
    if not text:
        return None, "empty"

    try:
        return json.loads(text), None
    except Exception:
        return {"raw": text}, None


def run_tool(name, args):
    env = os.environ.copy()
    env["OMNI_SKIP_MODEL_DOWNLOAD"] = "1"

    proc = subprocess.Popen(
        [str(mcp_bin), "--repo", str(repo), "--no-auto-index"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
    )

    def send(msg):
        proc.stdin.write(json.dumps(msg) + "\n")
        proc.stdin.flush()

    result = {"tool": name, "init_ok": False, "ok": False, "error": None, "preview": ""}

    try:
        send(
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "audit", "version": "1.0.0"},
                },
            }
        )

        init_resp, init_err = read_line_with_timeout(proc.stdout, 10)
        if init_err is not None or not isinstance(init_resp, dict) or "result" not in init_resp:
            result["error"] = f"initialize_failed:{init_err or init_resp}"
            return result

        result["init_ok"] = True

        send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        send(
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {"name": name, "arguments": args},
            }
        )

        call_resp, call_err = read_line_with_timeout(proc.stdout, 20)
        if call_err is not None:
            result["error"] = f"call_failed:{call_err}"
            return result

        if isinstance(call_resp, dict) and "result" in call_resp and "error" not in call_resp:
            result["ok"] = True
            content = call_resp.get("result", {}).get("content", [])
            if isinstance(content, list) and content:
                first = content[0]
                if isinstance(first, dict):
                    text = str(first.get("text", ""))
                else:
                    text = str(first)
                result["preview"] = text[:140].replace("\n", " ")
        else:
            result["error"] = call_resp.get("error") if isinstance(call_resp, dict) else str(call_resp)

        return result
    except Exception as exc:
        result["error"] = str(exc)
        return result
    finally:
        try:
            proc.terminate()
            proc.wait(timeout=2)
        except Exception:
            proc.kill()

        try:
            stderr_tail = proc.stderr.read()
            if stderr_tail and not result["ok"]:
                result["stderr_tail"] = stderr_tail[-800:]
        except Exception:
            pass


report = {
    "binary": str(mcp_bin),
    "results": [run_tool(name, args) for name, args in tools],
}

print(json.dumps(report, indent=2))
