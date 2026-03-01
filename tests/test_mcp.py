import subprocess
import json
import sys

def send_rpc(proc, msg):
    payload = json.dumps(msg) + "\n"
    proc.stdin.write(payload.encode("utf-8"))
    proc.stdin.flush()
    line = proc.stdout.readline()
    try:
        return json.loads(line)
    except:
        return line

def main():
    proc = subprocess.Popen(["target/release/omnicontext-mcp.exe", "--repo", "."],
                            stdin=subprocess.PIPE,
                            stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE)

    print(send_rpc(proc, {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0.0"}
        }
    }))

    print(send_rpc(proc, {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "notifications/initialized"
    }))

    print(send_rpc(proc, {
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/list",
        "params": {}
    }))

    print(send_rpc(proc, {
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "context_window",
            "arguments": {
                "query": "Engine struct",
                "limit": 5,
                "token_budget": 500
            }
        }
    }))

    proc.terminate()

if __name__ == "__main__":
    main()
