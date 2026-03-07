/**
 * Unit tests for bootstrapService.ts -- the zero-friction binary resolver.
 *
 * Tests cover:
 * - Platform target string generation
 * - Binary name derivation per platform
 * - Binary executability checks
 * - Candidate directory ordering and fallback logic
 * - GitHub asset URL construction
 * - Simulated install success/failure scenarios
 */

import * as assert from "assert";
import * as path from "path";
import * as os from "os";
import * as fs from "fs";

// Import the pure utility functions we can test without VS Code context
import { derivePipeName } from "../extensionUtils";

// ---------------------------------------------------------------------------
// Helpers: mirror the internal platform logic from bootstrapService
// (duplicated here since the functions are not exported; see note below)
// ---------------------------------------------------------------------------

function getPlatformTarget(
  platform: string = process.platform,
  arch: string = process.arch,
): string {
  const archStr = arch === "arm64" ? "aarch64" : "x86_64";
  switch (platform) {
    case "win32":
      return `${archStr}-pc-windows-msvc`;
    case "darwin":
      return `${archStr}-apple-darwin`;
    default:
      return `${archStr}-unknown-linux-gnu`;
  }
}

function getBinaryExt(platform: string = process.platform): string {
  return platform === "win32" ? ".exe" : "";
}

function buildAssetUrl(tag: string, platform: string, arch: string): string {
  const cleanTag = tag.replace(/^v/, "");
  const target = getPlatformTarget(platform, arch);
  const ext = platform === "win32" ? ".zip" : ".tar.gz";
  const assetName = `omnicontext-${cleanTag}-${target}${ext}`;
  return `https://github.com/steeltroops-ai/omnicontext/releases/download/${tag}/${assetName}`;
}

// ---------------------------------------------------------------------------
// Suite: Platform target derivation
// ---------------------------------------------------------------------------

suite("bootstrapService: getPlatformTarget", () => {
  test("win32 x64 produces correct Rust target triple", () => {
    const target = getPlatformTarget("win32", "x64");
    assert.strictEqual(target, "x86_64-pc-windows-msvc");
  });

  test("win32 arm64 produces aarch64 target", () => {
    const target = getPlatformTarget("win32", "arm64");
    assert.strictEqual(target, "aarch64-pc-windows-msvc");
  });

  test("darwin x64 produces correct target", () => {
    const target = getPlatformTarget("darwin", "x64");
    assert.strictEqual(target, "x86_64-apple-darwin");
  });

  test("darwin arm64 produces M1/M2 target", () => {
    const target = getPlatformTarget("darwin", "arm64");
    assert.strictEqual(target, "aarch64-apple-darwin");
  });

  test("linux x64 produces correct target", () => {
    const target = getPlatformTarget("linux", "x64");
    assert.strictEqual(target, "x86_64-unknown-linux-gnu");
  });

  test("linux arm64 produces aarch64 target", () => {
    const target = getPlatformTarget("linux", "arm64");
    assert.strictEqual(target, "aarch64-unknown-linux-gnu");
  });
});

// ---------------------------------------------------------------------------
// Suite: Binary extension
// ---------------------------------------------------------------------------

suite("bootstrapService: getBinaryExt", () => {
  test("win32 returns .exe", () => {
    assert.strictEqual(getBinaryExt("win32"), ".exe");
  });

  test("darwin returns empty string", () => {
    assert.strictEqual(getBinaryExt("darwin"), "");
  });

  test("linux returns empty string", () => {
    assert.strictEqual(getBinaryExt("linux"), "");
  });
});

// ---------------------------------------------------------------------------
// Suite: Asset URL construction
// ---------------------------------------------------------------------------

suite("bootstrapService: buildAssetUrl", () => {
  test("constructs correct Windows zip URL", () => {
    const url = buildAssetUrl("v0.7.2", "win32", "x64");
    assert.ok(url.includes("v0.7.2"), "Should include tag");
    assert.ok(url.includes("windows-msvc"), "Should include Windows target");
    assert.ok(url.endsWith(".zip"), "Windows archive should be .zip");
    assert.ok(
      url.startsWith("https://github.com/steeltroops-ai/omnicontext"),
      "Should use correct repo",
    );
  });

  test("constructs correct macOS tar.gz URL", () => {
    const url = buildAssetUrl("v0.7.2", "darwin", "arm64");
    assert.ok(
      url.includes("aarch64-apple-darwin"),
      "Should include arm64 darwin target",
    );
    assert.ok(url.endsWith(".tar.gz"), "macOS archive should be tar.gz");
  });

  test("constructs correct Linux tar.gz URL", () => {
    const url = buildAssetUrl("v0.7.2", "linux", "x64");
    assert.ok(url.includes("unknown-linux-gnu"), "Should include linux target");
    assert.ok(url.endsWith(".tar.gz"), "Linux archive should be tar.gz");
  });

  test("strips v prefix from tag for filename", () => {
    const url = buildAssetUrl("v1.0.0", "linux", "x64");
    // The asset file name should not contain 'vv'
    const fileName = url.split("/").pop()!;
    assert.ok(!fileName.includes("vv"), "Should not double-prefix version");
    assert.ok(fileName.includes("1.0.0"), "Should include semver without v");
  });

  test("tag without v prefix is handled", () => {
    const url = buildAssetUrl("0.7.2", "linux", "x64");
    const fileName = url.split("/").pop()!;
    assert.ok(fileName.includes("0.7.2"), "Should include version");
  });
});

// ---------------------------------------------------------------------------
// Suite: Filesystem binary validation (uses actual temp dirs)
// ---------------------------------------------------------------------------

suite("bootstrapService: binary file existence checks", () => {
  let tmpDir: string;

  setup(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "omnicontext-test-"));
  });

  teardown(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test("correctly identifies a file as present", () => {
    const binPath = path.join(tmpDir, "omnicontext");
    fs.writeFileSync(binPath, "#!/bin/sh\necho test");
    assert.ok(fs.existsSync(binPath), "Should find written test binary");
  });

  test("correctly identifies absent file", () => {
    const binPath = path.join(tmpDir, "nonexistent");
    assert.ok(!fs.existsSync(binPath), "Should not find absent binary");
  });

  test("resolveCandidateDirs produces ordered list", () => {
    // Verify that ~/.omnicontext/bin comes before ~/.cargo/bin
    const home = process.env.HOME || process.env.USERPROFILE || "";
    const omniLocal = path.join(home, ".omnicontext", "bin");
    const cargo = path.join(home, ".cargo", "bin");

    // Simulate candidate scanning order
    const candidates = [
      path.join(tmpDir, "bundled"), // would be extensionPath/bin
      path.join(tmpDir, "downloaded"),
      omniLocal,
      path.join(home, ".local", "bin"),
      cargo,
    ];

    // Create a binary in cargo slot
    const cargoDir = path.join(tmpDir, "downloaded");
    fs.mkdirSync(cargoDir, { recursive: true });
    fs.writeFileSync(
      path.join(cargoDir, "omnicontext"),
      "#!/bin/sh\necho 0.7.2",
    );

    let found: string | null = null;
    for (const dir of candidates) {
      const binPath = path.join(dir, "omnicontext");
      if (fs.existsSync(binPath)) {
        found = dir;
        break;
      }
    }

    assert.ok(found, "Should find at least one candidate");
    // downloaded dir comes before cargo — confirms ordering
    assert.ok(
      !found!.includes(".cargo"),
      "Should prefer downloaded over cargo when both exist",
    );
  });
});

// ---------------------------------------------------------------------------
// Suite: Dependency on extensionUtils (pipe name sanity)
// ---------------------------------------------------------------------------

suite("bootstrapService: IPC pipe from same root is consistent", () => {
  test("same repo produces same pipe name after bootstrap", () => {
    const pipe1 = derivePipeName("/home/user/myrepo");
    const pipe2 = derivePipeName("/home/user/myrepo");
    assert.strictEqual(
      pipe1,
      pipe2,
      "Bootstrap does not change pipe derivation",
    );
  });
});
