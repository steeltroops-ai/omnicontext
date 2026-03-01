#!/usr/bin/env python3
"""
Test OmniContext MCP server using the MCP protocol (stdio transport)
This tests the actual MCP communication, not just CLI commands
"""

import json
import subprocess
import sys
import time
from pathlib import Path

def send_jsonrpc(process, method, params=None, id=1):
    """Send a JSON-RPC request to the MCP server"""
    request = {
        "jsonrpc": "2.0",
        "method": method,
        "id": id
    }
    if params:
        request["params"] = params
    
    request_str = json.dumps(request) + "\n"
    process.stdin.write(request_str)
    process.stdin.flush()
    print(f"→ Sent: {method}")
    return id

def read_jsonrpc(process, timeout=10):
    """Read a JSON-RPC response from the MCP server"""
    start = time.time()
    while time.time() - start < timeout:
        line = process.stdout.readline()
        if line:
            try:
                response = json.loads(line)
                return response
            except json.JSONDecodeError:
                continue
    return None

def test_mcp_server():
    """Test the MCP server using the protocol"""
    
    # Find binary
    binary_path = Path("target/release/omnicontext-mcp.exe")
    if not binary_path.exists():
        binary_path = Path("target/release/omnicontext-mcp")
    
    if not binary_path.exists():
        print("✗ Binary not found. Run: cargo build -p omni-mcp --release")
        return False
    
    print("=== OmniContext MCP Protocol Test ===\n")
    print(f"Binary: {binary_path}")
    print(f"Repo: {Path.cwd()}\n")
    
    # Start MCP server
    print("→ Starting MCP server...")
    process = subprocess.Popen(
        [str(binary_path), "--repo", str(Path.cwd())],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1
    )
    
    try:
        # Give server time to initialize
        print("→ Waiting for server initialization...")
        time.sleep(3)
        
        tests_passed = 0
        tests_failed = 0
        
        # Test 1: Initialize
        print("\n→ Test 1: MCP Initialize")
        send_jsonrpc(process, "initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }, id=1)
        
        response = read_jsonrpc(process, timeout=15)
        if response and "result" in response:
            print("✓ Initialize successful")
            print(f"  Server: {response['result'].get('serverInfo', {}).get('name', 'unknown')}")
            print(f"  Version: {response['result'].get('serverInfo', {}).get('version', 'unknown')}")
            tests_passed += 1
        else:
            print("✗ Initialize failed")
            if response:
                print(f"  Response: {response}")
            tests_failed += 1
        
        # Test 2: List tools
        print("\n→ Test 2: List Tools")
        send_jsonrpc(process, "tools/list", {}, id=2)
        
        response = read_jsonrpc(process)
        if response and "result" in response:
            tools = response["result"].get("tools", [])
            print(f"✓ Found {len(tools)} tools")
            for tool in tools[:5]:  # Show first 5
                print(f"  - {tool.get('name', 'unknown')}")
            if len(tools) > 5:
                print(f"  ... and {len(tools) - 5} more")
            tests_passed += 1
        else:
            print("✗ List tools failed")
            if response:
                print(f"  Response: {response}")
            tests_failed += 1
        
        # Test 3: Get status
        print("\n→ Test 3: Call get_status tool")
        send_jsonrpc(process, "tools/call", {
            "name": "get_status",
            "arguments": {}
        }, id=3)
        
        response = read_jsonrpc(process)
        if response and "result" in response:
            print("✓ get_status successful")
            content = response["result"].get("content", [])
            if content:
                status_text = content[0].get("text", "")
                # Parse key metrics
                if "chunks" in status_text.lower():
                    print("  Status includes chunk count")
                if "files" in status_text.lower():
                    print("  Status includes file count")
                tests_passed += 1
            else:
                print("✗ get_status returned no content")
                tests_failed += 1
        else:
            print("✗ get_status failed")
            if response:
                print(f"  Response: {response}")
            tests_failed += 1
        
        # Test 4: Search code
        print("\n→ Test 4: Call search_code tool")
        send_jsonrpc(process, "tools/call", {
            "name": "search_code",
            "arguments": {
                "query": "engine",
                "limit": 3
            }
        }, id=4)
        
        response = read_jsonrpc(process, timeout=15)
        if response and "result" in response:
            print("✓ search_code successful")
            content = response["result"].get("content", [])
            if content:
                result_text = content[0].get("text", "")
                if result_text:
                    print(f"  Found results (length: {len(result_text)} chars)")
                    tests_passed += 1
                else:
                    print("  No results found (may be normal for empty index)")
                    tests_passed += 1
            else:
                print("✗ search_code returned no content")
                tests_failed += 1
        else:
            print("✗ search_code failed")
            if response:
                print(f"  Response: {response}")
            tests_failed += 1
        
        # Summary
        print("\n=== Test Results ===")
        print(f"Passed: {tests_passed}")
        print(f"Failed: {tests_failed}")
        total = tests_passed + tests_failed
        success_rate = (tests_passed / total * 100) if total > 0 else 0
        print(f"Success Rate: {success_rate:.1f}%")
        
        if tests_failed == 0:
            print("\n✓ All MCP protocol tests passed!")
            return True
        else:
            print("\n✗ Some tests failed")
            return False
        
    except Exception as e:
        print(f"\n✗ Test error: {e}")
        import traceback
        traceback.print_exc()
        return False
    
    finally:
        # Cleanup
        print("\n→ Shutting down server...")
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
        print("✓ Server stopped")

if __name__ == "__main__":
    success = test_mcp_server()
    sys.exit(0 if success else 1)
