import subprocess
import json
import sys

def test_mcp():
    mcp_path = r"C:\Users\mayan\.omnicontext\bin\omnicontext-mcp.exe"
    repo_path = r"C:\Omniverse\Projects\omnicontext"
    
    process = subprocess.Popen(
        [mcp_path, "--repo", repo_path],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    # 1. Initialize
    init_request = {
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        },
        "id": 1
    }
    
    print(f"Sending initialize: {json.dumps(init_request)}")
    process.stdin.write(json.dumps(init_request) + "\n")
    process.stdin.flush()
    init_response = process.stdout.readline()
    print(f"Init response: {init_response}")

    # 2. Initialized notification
    initialized_notification = {
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    }
    process.stdin.write(json.dumps(initialized_notification) + "\n")
    process.stdin.flush()

    # 3. List tools
    list_request = {
        "jsonrpc": "2.0",
        "method": "tools/list",
        "params": {},
        "id": 2
    }
    print(f"Sending tools/list: {json.dumps(list_request)}")
    process.stdin.write(json.dumps(list_request) + "\n")
    process.stdin.flush()
    list_response = process.stdout.readline()
    print(f"Tools list response: {list_response}")

    # 4. Call a tool (get_architecture)
    call_request = {
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_architecture",
            "arguments": {}
        },
        "id": 3
    }
    print(f"Calling get_architecture: {json.dumps(call_request)}")
    process.stdin.write(json.dumps(call_request) + "\n")
    process.stdin.flush()
    call_response = process.stdout.readline()
    print(f"Call response: {call_response}")

    process.terminate()

if __name__ == "__main__":
    test_mcp()
