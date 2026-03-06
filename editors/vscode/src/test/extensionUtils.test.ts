/**
 * Unit tests for extensionUtils -- the pure-function core of the extension.
 *
 * These tests run without a VS Code instance, verifying:
 * - IPC pipe name derivation (determinism, platform correctness)
 * - CLI context assembly (token budget, empty results, sanitization)
 * - JSON-RPC protocol helpers (request building, response parsing)
 * - Reconnection backoff calculation
 * - Binary path derivation
 * - Preflight formatting
 * - Display sanitization
 */

import * as assert from "assert";
import {
  derivePipeName,
  assembleCliContext,
  sanitizeForDisplay,
  buildJsonRpcRequest,
  parseJsonRpcResponse,
  calculateBackoffDelay,
  deriveMcpBinaryPath,
  deriveDaemonBinaryPath,
  formatPreflightContext,
  getKnownMcpClients,
  buildMcpServerEntry,
  mergeMcpConfig,
  CliSearchResult,
  McpClientTarget,
} from "../extensionUtils";

suite("extensionUtils", () => {
  // -----------------------------------------------------------------------
  // derivePipeName
  // -----------------------------------------------------------------------
  suite("derivePipeName", () => {
    test("should return deterministic pipe name for same input", () => {
      const name1 = derivePipeName("/home/user/project");
      const name2 = derivePipeName("/home/user/project");
      assert.strictEqual(name1, name2);
    });

    test("should return different pipe names for different paths", () => {
      const name1 = derivePipeName("/home/user/project-a");
      const name2 = derivePipeName("/home/user/project-b");
      assert.notStrictEqual(name1, name2);
    });

    test("should be case-insensitive", () => {
      const name1 = derivePipeName("C:\\Users\\Dev\\MyProject");
      const name2 = derivePipeName("c:\\users\\dev\\myproject");
      assert.strictEqual(name1, name2);
    });

    test("should contain omnicontext prefix", () => {
      const name = derivePipeName("/test/repo");
      assert.ok(
        name.includes("omnicontext-"),
        `Expected omnicontext- prefix in: ${name}`,
      );
    });

    test("should use hash of exactly 12 hex characters", () => {
      const name = derivePipeName("/test/repo");
      const match = name.match(/omnicontext-([a-f0-9]+)/);
      assert.ok(match, "Should contain hash");
      assert.strictEqual(match![1].length, 12, "Hash should be 12 chars");
    });

    test("should produce valid pipe path for win32", () => {
      if (process.platform === "win32") {
        const name = derivePipeName("C:\\Users\\test");
        assert.ok(
          name.startsWith("\\\\.\\pipe\\"),
          `Expected pipe path, got: ${name}`,
        );
      }
    });

    test("should produce .sock path for non-win32", () => {
      if (process.platform !== "win32") {
        const name = derivePipeName("/home/test");
        assert.ok(
          name.endsWith(".sock"),
          `Expected .sock suffix, got: ${name}`,
        );
      }
    });
  });

  // -----------------------------------------------------------------------
  // assembleCliContext
  // -----------------------------------------------------------------------
  suite("assembleCliContext", () => {
    const makeResult = (
      symbol: string,
      content: string,
      score = 0.9,
    ): CliSearchResult => ({
      symbol,
      kind: "function",
      score,
      file: "src/main.rs",
      content,
      line_start: 1,
      line_end: 5,
    });

    test("should return null for empty results", () => {
      const result = assembleCliContext([], 8192, 42);
      assert.strictEqual(result, null);
    });

    test("should return null for null-ish results", () => {
      const result = assembleCliContext(null as any, 8192, 42);
      assert.strictEqual(result, null);
    });

    test("should assemble context with proper structure", () => {
      const results = [makeResult("foo", "fn foo() {}")];
      const assembled = assembleCliContext(results, 8192, 100);

      assert.ok(assembled);
      assert.ok(assembled!.system_context.includes("<context_engine>"));
      assert.ok(assembled!.system_context.includes("</context_engine>"));
      assert.ok(assembled!.system_context.includes("foo"));
      assert.ok(assembled!.system_context.includes("fn foo() {}"));
      assert.strictEqual(assembled!.entries_count, 1);
      assert.strictEqual(assembled!.token_budget, 8192);
      assert.strictEqual(assembled!.elapsed_ms, 100);
    });

    test("should respect token budget", () => {
      // Each result has ~10 chars, which is ~3 tokens (ceil(10/4) = 3)
      const results = Array.from({ length: 100 }, (_, i) =>
        makeResult(`fn_${i}`, `content_${i}`),
      );

      const assembled = assembleCliContext(results, 10, 0);
      assert.ok(assembled);
      // Should have far fewer entries than 100
      assert.ok(
        assembled!.entries_count < 100,
        `Expected entries < 100, got ${assembled!.entries_count}`,
      );
      assert.ok(
        assembled!.tokens_used <= 10,
        `Expected tokens <= 10, got ${assembled!.tokens_used}`,
      );
    });

    test("should include file and kind in output", () => {
      const results = [makeResult("bar", "let bar = 42;")];
      const assembled = assembleCliContext(results, 8192, 0);

      assert.ok(assembled);
      assert.ok(assembled!.system_context.includes("File: src/main.rs"));
      assert.ok(assembled!.system_context.includes("function"));
    });

    test("should include score in output", () => {
      const results = [makeResult("baz", "const baz = 1;", 0.8765)];
      const assembled = assembleCliContext(results, 8192, 0);

      assert.ok(assembled);
      assert.ok(assembled!.system_context.includes("0.8765"));
    });

    test("should handle multiple results", () => {
      const results = [
        makeResult("fn_a", "function a() {}"),
        makeResult("fn_b", "function b() {}"),
        makeResult("fn_c", "function c() {}"),
      ];
      const assembled = assembleCliContext(results, 8192, 0);

      assert.ok(assembled);
      assert.strictEqual(assembled!.entries_count, 3);
      assert.ok(assembled!.system_context.includes("fn_a"));
      assert.ok(assembled!.system_context.includes("fn_b"));
      assert.ok(assembled!.system_context.includes("fn_c"));
    });

    test("should sanitize symbol names in output", () => {
      const results = [makeResult("a\x00b", "content")];
      const assembled = assembleCliContext(results, 8192, 0);

      assert.ok(assembled);
      assert.ok(!assembled!.system_context.includes("\x00"));
    });
  });

  // -----------------------------------------------------------------------
  // sanitizeForDisplay
  // -----------------------------------------------------------------------
  suite("sanitizeForDisplay", () => {
    test("should pass through normal text", () => {
      assert.strictEqual(sanitizeForDisplay("hello world"), "hello world");
    });

    test("should strip null bytes", () => {
      assert.strictEqual(sanitizeForDisplay("hello\x00world"), "helloworld");
    });

    test("should strip control characters", () => {
      const result = sanitizeForDisplay("line1\x01\x02\x03line2");
      assert.strictEqual(result, "line1line2");
    });

    test("should preserve newlines, carriage returns, and tabs", () => {
      const input = "line1\nline2\r\nline3\ttab";
      assert.strictEqual(sanitizeForDisplay(input), input);
    });

    test("should prevent code fence escaping", () => {
      const result = sanitizeForDisplay("before``` ```after");
      assert.ok(!result.includes("```"), "Should not contain triple backticks");
    });

    test("should truncate to 500 characters", () => {
      const long = "x".repeat(1000);
      const result = sanitizeForDisplay(long);
      assert.strictEqual(result.length, 500);
    });

    test("should handle empty string", () => {
      assert.strictEqual(sanitizeForDisplay(""), "");
    });

    test("should handle null/undefined input", () => {
      assert.strictEqual(sanitizeForDisplay(null as any), "");
      assert.strictEqual(sanitizeForDisplay(undefined as any), "");
    });
  });

  // -----------------------------------------------------------------------
  // buildJsonRpcRequest
  // -----------------------------------------------------------------------
  suite("buildJsonRpcRequest", () => {
    test("should build valid JSON-RPC 2.0 request", () => {
      const payload = buildJsonRpcRequest(1, "search", { query: "test" });
      const parsed = JSON.parse(payload.trim());

      assert.strictEqual(parsed.jsonrpc, "2.0");
      assert.strictEqual(parsed.id, 1);
      assert.strictEqual(parsed.method, "search");
      assert.deepStrictEqual(parsed.params, { query: "test" });
    });

    test("should end with newline", () => {
      const payload = buildJsonRpcRequest(1, "test", {});
      assert.ok(payload.endsWith("\n"), "Should end with newline");
    });

    test("should handle complex params", () => {
      const params = {
        prompt: "find auth",
        active_file: "/src/main.rs",
        open_files: ["/a.rs", "/b.rs"],
        token_budget: 8192,
      };
      const payload = buildJsonRpcRequest(42, "preflight", params);
      const parsed = JSON.parse(payload.trim());

      assert.strictEqual(parsed.id, 42);
      assert.strictEqual(parsed.method, "preflight");
      assert.deepStrictEqual(parsed.params.open_files, ["/a.rs", "/b.rs"]);
    });

    test("should handle empty params", () => {
      const payload = buildJsonRpcRequest(1, "status", {});
      const parsed = JSON.parse(payload.trim());
      assert.deepStrictEqual(parsed.params, {});
    });

    test("should increment id correctly", () => {
      const p1 = JSON.parse(buildJsonRpcRequest(1, "a", {}).trim());
      const p2 = JSON.parse(buildJsonRpcRequest(2, "b", {}).trim());
      assert.strictEqual(p1.id, 1);
      assert.strictEqual(p2.id, 2);
    });
  });

  // -----------------------------------------------------------------------
  // parseJsonRpcResponse
  // -----------------------------------------------------------------------
  suite("parseJsonRpcResponse", () => {
    test("should parse valid success response", () => {
      const line = JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        result: { ok: true },
      });
      const response = parseJsonRpcResponse(line);

      assert.ok(response);
      assert.strictEqual(response!.id, 1);
      assert.deepStrictEqual(response!.result, { ok: true });
    });

    test("should parse valid error response", () => {
      const line = JSON.stringify({
        jsonrpc: "2.0",
        id: 2,
        error: { code: -1, message: "not found" },
      });
      const response = parseJsonRpcResponse(line);

      assert.ok(response);
      assert.strictEqual(response!.id, 2);
      assert.strictEqual(response!.error!.message, "not found");
    });

    test("should return null for empty string", () => {
      assert.strictEqual(parseJsonRpcResponse(""), null);
    });

    test("should return null for whitespace-only string", () => {
      assert.strictEqual(parseJsonRpcResponse("   "), null);
    });

    test("should return null for invalid JSON", () => {
      assert.strictEqual(parseJsonRpcResponse("not json"), null);
    });

    test("should return null for missing id", () => {
      const line = JSON.stringify({ jsonrpc: "2.0", result: {} });
      assert.strictEqual(parseJsonRpcResponse(line), null);
    });

    test("should return null for non-numeric id", () => {
      const line = JSON.stringify({ jsonrpc: "2.0", id: "abc", result: {} });
      assert.strictEqual(parseJsonRpcResponse(line), null);
    });
  });

  // -----------------------------------------------------------------------
  // calculateBackoffDelay
  // -----------------------------------------------------------------------
  suite("calculateBackoffDelay", () => {
    test("should return 1000ms for first attempt", () => {
      assert.strictEqual(calculateBackoffDelay(0), 1000);
    });

    test("should double with each attempt", () => {
      assert.strictEqual(calculateBackoffDelay(1), 2000);
      assert.strictEqual(calculateBackoffDelay(2), 4000);
      assert.strictEqual(calculateBackoffDelay(3), 8000);
    });

    test("should cap at 30000ms", () => {
      assert.strictEqual(calculateBackoffDelay(10), 30000);
      assert.strictEqual(calculateBackoffDelay(20), 30000);
      assert.strictEqual(calculateBackoffDelay(100), 30000);
    });

    test("should cap at exactly 30000ms for attempt 5", () => {
      // 2^5 * 1000 = 32000, capped to 30000
      assert.strictEqual(calculateBackoffDelay(5), 30000);
    });
  });

  // -----------------------------------------------------------------------
  // deriveMcpBinaryPath
  // -----------------------------------------------------------------------
  suite("deriveMcpBinaryPath", () => {
    test("should derive MCP binary path on unix", () => {
      const result = deriveMcpBinaryPath("/usr/local/bin/omnicontext");
      assert.strictEqual(result, "/usr/local/bin/omnicontext-mcp");
    });

    test("should derive MCP binary path on windows", () => {
      const result = deriveMcpBinaryPath("C:\\Program Files\\omnicontext.exe");
      assert.strictEqual(result, "C:\\Program Files\\omnicontext-mcp.exe");
    });

    test("should handle bare binary name", () => {
      const result = deriveMcpBinaryPath("omnicontext");
      assert.strictEqual(result, "omnicontext-mcp");
    });

    test("should not modify if pattern does not match", () => {
      const result = deriveMcpBinaryPath("some-other-binary");
      assert.strictEqual(result, "some-other-binary");
    });
  });

  // -----------------------------------------------------------------------
  // deriveDaemonBinaryPath
  // -----------------------------------------------------------------------
  suite("deriveDaemonBinaryPath", () => {
    test("should derive daemon binary path", () => {
      const result = deriveDaemonBinaryPath("/usr/local/bin/omnicontext");
      assert.strictEqual(result, "/usr/local/bin/omnicontext-daemon");
    });

    test("should derive daemon binary path on windows", () => {
      const result = deriveDaemonBinaryPath("C:\\bin\\omnicontext.exe");
      assert.strictEqual(result, "C:\\bin\\omnicontext-daemon.exe");
    });
  });

  // -----------------------------------------------------------------------
  // formatPreflightContext
  // -----------------------------------------------------------------------
  suite("formatPreflightContext", () => {
    test("should prepend cached indicator when from cache", () => {
      const result = formatPreflightContext("code context here", 5, true);
      assert.ok(result.startsWith("[Cached context, 5ms]"));
      assert.ok(result.includes("code context here"));
    });

    test("should prepend fresh indicator when not from cache", () => {
      const result = formatPreflightContext("code context here", 150, false);
      assert.ok(result.startsWith("[Fresh search, 150ms]"));
      assert.ok(result.includes("code context here"));
    });

    test("should preserve original context content", () => {
      const original = "### fn_main\n```\nfn main() {}\n```\n";
      const result = formatPreflightContext(original, 42, false);
      assert.ok(result.includes(original));
    });

    test("should include elapsed time", () => {
      const result = formatPreflightContext("ctx", 999, true);
      assert.ok(result.includes("999ms"));
    });
  });

  // -----------------------------------------------------------------------
  // MCP Client Discovery & Config Merging
  // -----------------------------------------------------------------------
  suite("getKnownMcpClients", () => {
    test("should return a non-empty list of clients", () => {
      const clients = getKnownMcpClients();
      assert.ok(clients.length > 0, "Should return at least one client");
    });

    test("should include Claude Desktop", () => {
      const clients = getKnownMcpClients();
      const claude = clients.find((c) => c.name === "Claude Desktop");
      assert.ok(claude, "Should include Claude Desktop");
      assert.ok(
        claude!.configPath.includes("claude"),
        "Config path should reference claude",
      );
    });

    test("should include Cursor", () => {
      const clients = getKnownMcpClients();
      const cursor = clients.find((c) => c.name === "Cursor");
      assert.ok(cursor, "Should include Cursor");
    });

    test("should include Continue.dev", () => {
      const clients = getKnownMcpClients();
      const cont = clients.find((c) => c.name === "Continue.dev");
      assert.ok(cont, "Should include Continue.dev");
    });

    test("should include Kiro with powers namespace", () => {
      const clients = getKnownMcpClients();
      const kiro = clients.find((c) => c.name === "Kiro");
      assert.ok(kiro, "Should include Kiro");
      assert.strictEqual(kiro!.usesPowersNamespace, true);
    });

    test("should include Windsurf", () => {
      const clients = getKnownMcpClients();
      const windsurf = clients.find((c) => c.name === "Windsurf");
      assert.ok(windsurf, "Should include Windsurf");
    });

    test("should include Cline", () => {
      const clients = getKnownMcpClients();
      const cline = clients.find((c) => c.name === "Cline");
      assert.ok(cline, "Should include Cline");
    });

    test("all clients should have non-empty configPath", () => {
      const clients = getKnownMcpClients();
      for (const client of clients) {
        assert.ok(
          client.configPath.length > 0,
          `${client.name} should have a config path`,
        );
      }
    });
  });

  suite("buildMcpServerEntry", () => {
    test("should build entry with correct command and args", () => {
      const entry = buildMcpServerEntry(
        "/usr/bin/omnicontext-mcp",
        "/home/user/project",
      );
      assert.strictEqual(entry.command, "/usr/bin/omnicontext-mcp");
      assert.deepStrictEqual(entry.args, ["--repo", "/home/user/project"]);
      assert.strictEqual(entry.disabled, false);
    });

    test("should handle windows paths", () => {
      const entry = buildMcpServerEntry(
        "C:\\bin\\omnicontext-mcp.exe",
        "C:\\Users\\dev\\project",
      );
      assert.strictEqual(entry.command, "C:\\bin\\omnicontext-mcp.exe");
      assert.deepStrictEqual(entry.args, ["--repo", "C:\\Users\\dev\\project"]);
    });
  });

  suite("mergeMcpConfig", () => {
    test("should create fresh config when no existing JSON", () => {
      const target: McpClientTarget = {
        name: "Test",
        configPath: "/tmp/test.json",
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      };
      const entry = buildMcpServerEntry("/bin/mcp", "/repo");
      const merged = mergeMcpConfig(null, target, entry);

      assert.ok(merged.mcpServers);
      assert.ok(merged.mcpServers.omnicontext);
      assert.strictEqual(merged.mcpServers.omnicontext.command, "/bin/mcp");
    });

    test("should preserve existing config entries", () => {
      const existing = JSON.stringify({
        mcpServers: {
          other_server: { command: "/bin/other", args: [] },
        },
      });
      const target: McpClientTarget = {
        name: "Test",
        configPath: "/tmp/test.json",
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      };
      const entry = buildMcpServerEntry("/bin/mcp", "/repo");
      const merged = mergeMcpConfig(existing, target, entry);

      // Should have both entries
      assert.ok(merged.mcpServers.other_server, "Should preserve other_server");
      assert.ok(merged.mcpServers.omnicontext, "Should add omnicontext");
    });

    test("should use powers namespace for Kiro", () => {
      const target: McpClientTarget = {
        name: "Kiro",
        configPath: "/tmp/kiro.json",
        serversKey: "mcpServers",
        usesPowersNamespace: true,
      };
      const entry = buildMcpServerEntry("/bin/mcp", "/repo");
      const merged = mergeMcpConfig(null, target, entry);

      assert.ok(merged.powers, "Should have powers namespace");
      assert.ok(
        merged.powers.mcpServers,
        "Should have mcpServers under powers",
      );
      assert.ok(
        merged.powers.mcpServers.omnicontext,
        "Should have omnicontext under powers.mcpServers",
      );
    });

    test("should handle invalid existing JSON gracefully", () => {
      const target: McpClientTarget = {
        name: "Test",
        configPath: "/tmp/test.json",
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      };
      const entry = buildMcpServerEntry("/bin/mcp", "/repo");
      const merged = mergeMcpConfig("not valid json{{{", target, entry);

      assert.ok(merged.mcpServers);
      assert.ok(merged.mcpServers.omnicontext);
    });

    test("should update existing omnicontext entry", () => {
      const existing = JSON.stringify({
        mcpServers: {
          omnicontext: { command: "/old/path", args: ["--repo", "/old/repo"] },
        },
      });
      const target: McpClientTarget = {
        name: "Test",
        configPath: "/tmp/test.json",
        serversKey: "mcpServers",
        usesPowersNamespace: false,
      };
      const entry = buildMcpServerEntry("/new/path", "/new/repo");
      const merged = mergeMcpConfig(existing, target, entry);

      assert.strictEqual(merged.mcpServers.omnicontext.command, "/new/path");
      assert.deepStrictEqual(merged.mcpServers.omnicontext.args, [
        "--repo",
        "/new/repo",
      ]);
    });
  });
});
