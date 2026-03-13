/**
 * BootstrapService: Zero-friction binary resolution and auto-download.
 *
 * Execution order on extension activation:
 *   1. Check extensionPath/bin/<platform> for bundled binary (fastest path).
 *   2. Check ~/.omnicontext/bin (standalone installer path).
 *   3. Check ~/.cargo/bin (developer path).
 *   4. Check system PATH.
 *   5. If nothing found: download latest release from GitHub into globalStoragePath.
 *
 * This ensures the extension works for ALL users regardless of whether
 * they have Rust, Cargo, or any prior setup.
 */

import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import * as https from "https";
import * as crypto from "crypto";
import { execSync } from "child_process";

export interface BootstrapResult {
  cliBinary: string;
  daemonBinary: string;
  mcpBinary: string;
  /** True when the ONNX Runtime shared library is present next to the binary
   *  (onnxruntime.dll on Windows; libonnxruntime.so / .dylib on Unix).
   *  On Unix this is always true when the library is system-wide; the value
   *  is only meaningfully false on Windows where a missing DLL is fatal. */
  onnxDllPresent: boolean;
}

export type BootstrapPhase =
  | "checking"
  | "downloading"
  | "extracting"
  | "verifying"
  | "ready"
  | "failed";

export interface BootstrapStatus {
  phase: BootstrapPhase;
  message: string;
  progressPercent?: number;
}

type StatusCallback = (status: BootstrapStatus) => void;

const REPO_OWNER = "steeltroops-ai";
const REPO_NAME = "omnicontext";
const BINARY_NAME = "omnicontext";

// ---------------------------------------------------------------------------
// Platform helpers
// ---------------------------------------------------------------------------

function getPlatformTarget(): string {
  const arch = process.arch === "arm64" ? "aarch64" : "x86_64";
  switch (process.platform) {
    case "win32":
      return `${arch}-pc-windows-msvc`;
    case "darwin":
      return `${arch}-apple-darwin`;
    default:
      return `${arch}-unknown-linux-gnu`;
  }
}

function getBinaryExt(): string {
  return process.platform === "win32" ? ".exe" : "";
}

function getBinNames(): { cli: string; daemon: string; mcp: string } {
  const ext = getBinaryExt();
  return {
    cli: `${BINARY_NAME}${ext}`,
    daemon: `${BINARY_NAME}-daemon${ext}`,
    mcp: `${BINARY_NAME}-mcp${ext}`,
  };
}

// ---------------------------------------------------------------------------
// Binary verification
// ---------------------------------------------------------------------------

function isBinaryExecutable(binPath: string): boolean {
  if (!fs.existsSync(binPath)) {
    return false;
  }
  try {
    fs.accessSync(binPath, fs.constants.X_OK);
    return true;
  } catch {
    // On Windows, X_OK check is unreliable. Fall back to existence check.
    return process.platform === "win32";
  }
}

function tryRunVersion(binPath: string): string | null {
  try {
    const result = execSync(`"${binPath}" --version`, {
      timeout: 3000,
      stdio: ["ignore", "pipe", "ignore"],
    });
    return result.toString().trim();
  } catch {
    return null;
  }
}

function parseSemver(versionText: string | null): string | null {
  if (!versionText) {
    return null;
  }
  const match = versionText.match(/(\d+)\.(\d+)\.(\d+)/);
  return match ? `${match[1]}.${match[2]}.${match[3]}` : null;
}

function compareSemver(a: string, b: string): number {
  const aa = a.split(".").map((v) => parseInt(v, 10));
  const bb = b.split(".").map((v) => parseInt(v, 10));
  for (let i = 0; i < 3; i++) {
    const av = aa[i] ?? 0;
    const bv = bb[i] ?? 0;
    if (av > bv) return 1;
    if (av < bv) return -1;
  }
  return 0;
}

// ---------------------------------------------------------------------------
// Candidate resolution (ordered by preference)
// ---------------------------------------------------------------------------

function resolveCandidateDirs(context: vscode.ExtensionContext): string[] {
  const home = process.env.HOME || process.env.USERPROFILE || "";
  const cargoBin = path.join(home, ".cargo", "bin");
  const omniLocalBin = path.join(home, ".omnicontext", "bin");
  const localBin = path.join(home, ".local", "bin");

  // extensionPath/bin/<platform> is the highest priority — bundled binaries
  const bundledBinDir = path.join(
    context.extensionPath,
    "bin",
    getPlatformTarget(),
  );

  // globalStoragePath/bin is where we auto-download to
  const downloadedBinDir = path.join(context.globalStoragePath, "bin");

  return [
    bundledBinDir, // 1. Bundled inside VSIX (fastest, works offline)
    downloadedBinDir, // 2. Auto-downloaded by this service
    omniLocalBin, // 3. Standalone install.ps1 / install.sh
    localBin, // 4. Linux ~/.local/bin
    cargoBin, // 5. Developer cargo install
  ];
}

export async function resolveBinaries(
  context: vscode.ExtensionContext,
): Promise<BootstrapResult | null> {
  const names = getBinNames();
  const candidates = resolveCandidateDirs(context);

  for (const dir of candidates) {
    const cliBin = path.join(dir, names.cli);
    if (isBinaryExecutable(cliBin)) {
      const daemonBin = path.join(dir, names.daemon);
      const mcpBin = path.join(dir, names.mcp);

      // DLL / shared-library co-location check.
      // On Windows we require onnxruntime.dll in the same directory.
      // On Linux/macOS we accept any of the common versioned or unversioned
      // names, but fall back to `true` so that a missing lib never blocks
      // startup on Unix systems that may load it from LD_LIBRARY_PATH.
      let onnxDllPresent: boolean;
      if (process.platform === "win32") {
        onnxDllPresent = fs.existsSync(path.join(dir, "onnxruntime.dll"));
      } else if (process.platform === "darwin") {
        onnxDllPresent =
          fs.existsSync(path.join(dir, "libonnxruntime.dylib")) ||
          // versioned name produced by downloadOnnxRuntime (e.g. libonnxruntime.1.24.3.dylib)
          fs
            .readdirSync(dir)
            .some(
              (f) => f.startsWith("libonnxruntime") && f.endsWith(".dylib"),
            ) ||
          true; // don't block on macOS — system or brew dylib may be available
      } else {
        // Linux
        onnxDllPresent =
          fs.existsSync(path.join(dir, "libonnxruntime.so")) ||
          fs.existsSync(path.join(dir, "libonnxruntime.so.1")) ||
          // versioned name (e.g. libonnxruntime.so.1.24.3)
          fs
            .readdirSync(dir)
            .some(
              (f) =>
                f.startsWith("libonnxruntime.so") ||
                f.startsWith("libonnxruntime-"),
            ) ||
          true; // don't block on Linux — lib may be installed system-wide
      }

      return {
        cliBinary: cliBin,
        daemonBinary: isBinaryExecutable(daemonBin) ? daemonBin : cliBin,
        mcpBinary: isBinaryExecutable(mcpBin) ? mcpBin : cliBin,
        onnxDllPresent,
      };
    }
  }

  return null;
}

// ---------------------------------------------------------------------------
// GitHub Release download
// ---------------------------------------------------------------------------

async function fetchLatestReleaseTag(): Promise<string> {
  return new Promise((resolve, reject) => {
    const url = `https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases`;
    const req = https.get(
      url,
      {
        headers: {
          "User-Agent": "omnicontext-vscode",
          Accept: "application/vnd.github+json",
        },
      },
      (res) => {
        let data = "";
        res.on("data", (chunk) => (data += chunk));
        res.on("end", () => {
          try {
            const releases: any[] = JSON.parse(data);
            const release = releases.find(
              (r: any) => r.assets && r.assets.length > 0,
            );
            if (release) {
              resolve(release.tag_name as string);
            } else {
              reject(
                new Error("No published releases with binary assets found"),
              );
            }
          } catch (err) {
            reject(new Error(`Failed to parse GitHub API response: ${err}`));
          }
        });
      },
    );
    req.on("error", reject);
    req.setTimeout(10000, () => {
      req.destroy();
      reject(new Error("GitHub API request timed out"));
    });
  });
}

function buildAssetUrl(tag: string): string {
  const cleanTag = tag.replace(/^v/, "");
  const target = getPlatformTarget();
  const ext = process.platform === "win32" ? ".zip" : ".tar.gz";
  const assetName = `omnicontext-${cleanTag}-${target}${ext}`;
  return `https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${tag}/${assetName}`;
}

async function downloadFile(
  url: string,
  destPath: string,
  onProgress: (percent: number) => void,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destPath);

    const handleResponse = (res: any) => {
      // Follow redirects (GitHub uses 302 for asset downloads)
      if (
        res.statusCode === 301 ||
        res.statusCode === 302 ||
        res.statusCode === 307
      ) {
        const redirectUrl = res.headers["location"];
        if (!redirectUrl) {
          reject(new Error("Redirect with no location header"));
          return;
        }
        https.get(redirectUrl, handleResponse).on("error", reject);
        return;
      }

      if (res.statusCode !== 200) {
        reject(
          new Error(
            `HTTP ${res.statusCode} downloading binary. Check if release exists.`,
          ),
        );
        return;
      }

      const totalBytes = parseInt(res.headers["content-length"] || "0", 10);
      let receivedBytes = 0;

      res.pipe(file);
      res.on("data", (chunk: Buffer) => {
        receivedBytes += chunk.length;
        if (totalBytes > 0) {
          onProgress(Math.round((receivedBytes / totalBytes) * 100));
        }
      });

      file.on("finish", () => {
        file.close();
        resolve();
      });
    };

    https
      .get(
        url,
        { headers: { "User-Agent": "omnicontext-vscode" } },
        handleResponse,
      )
      .on("error", (err) => {
        fs.unlink(destPath, () => {});
        reject(err);
      });
  });
}

async function extractArchive(
  archivePath: string,
  destDir: string,
): Promise<void> {
  fs.mkdirSync(destDir, { recursive: true });

  if (archivePath.endsWith(".zip")) {
    // Use PowerShell on Windows (available on Win8+)
    execSync(
      `powershell -NoProfile -Command "Expand-Archive -Path '${archivePath}' -DestinationPath '${destDir}' -Force"`,
      { timeout: 60000 },
    );
  } else {
    // tar is available on macOS and all modern Linux
    execSync(`tar -xzf "${archivePath}" -C "${destDir}"`, { timeout: 60000 });
  }
}

// ---------------------------------------------------------------------------
// ONNX Runtime download: Windows (.dll), Linux (.so), macOS (.dylib)
// Dynamically fetches the latest stable version from GitHub.
// ---------------------------------------------------------------------------

async function fetchLatestOnnxRuntimeVersion(): Promise<string> {
  return new Promise((resolve) => {
    const url =
      "https://api.github.com/repos/microsoft/onnxruntime/releases/latest";
    const req = https.get(
      url,
      {
        headers: {
          "User-Agent": "omnicontext-vscode",
          Accept: "application/vnd.github+json",
        },
      },
      (res) => {
        let data = "";
        res.on("data", (chunk) => (data += chunk));
        res.on("end", () => {
          try {
            const release = JSON.parse(data);
            if (release && release.tag_name) {
              resolve(release.tag_name.replace(/^v/, ""));
            } else {
              resolve("1.24.3"); // 2026 Stable Fallback
            }
          } catch {
            resolve("1.24.3");
          }
        });
      },
    );
    req.on("error", () => resolve("1.24.3"));
    req.setTimeout(5000, () => {
      req.destroy();
      resolve("1.24.3");
    });
  });
}

function buildOnnxUrl(version: string): { url: string; libName: string } {
  const arch = process.arch === "arm64" ? "arm64" : "x64";
  const ver = version;
  switch (process.platform) {
    case "win32":
      return {
        url: `https://github.com/microsoft/onnxruntime/releases/download/v${ver}/onnxruntime-win-x64-${ver}.zip`,
        libName: "onnxruntime.dll",
      };
    case "darwin":
      return {
        url: `https://github.com/microsoft/onnxruntime/releases/download/v${ver}/onnxruntime-osx-${arch}-${ver}.tgz`,
        libName: `libonnxruntime.${ver}.dylib`,
      };
    default: // linux
      return {
        url: `https://github.com/microsoft/onnxruntime/releases/download/v${ver}/onnxruntime-linux-x64-${ver}.tgz`,
        libName: `libonnxruntime.so.${ver}`,
      };
  }
}

async function downloadOnnxRuntime(
  destDir: string,
  onStatus: StatusCallback,
): Promise<boolean> {
  const latestVersion = await fetchLatestOnnxRuntimeVersion();
  const { url, libName } = buildOnnxUrl(latestVersion);
  const tmpDir = path.join(destDir, "_onnx_tmp");
  const ext = url.endsWith(".zip") ? ".zip" : ".tgz";
  const archivePath = path.join(tmpDir, `onnxruntime${ext}`);

  fs.mkdirSync(tmpDir, { recursive: true });

  onStatus({
    phase: "downloading",
    message: `Downloading ONNX Runtime ${latestVersion} from Microsoft...`,
    progressPercent: 0,
  });

  try {
    await downloadFile(url, archivePath, (pct) => {
      onStatus({
        phase: "downloading",
        message: `Downloading ONNX Runtime ${latestVersion}... ${pct}%`,
        progressPercent: pct,
      });
    });
  } catch (err: any) {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    onStatus({
      phase: "verifying",
      message: `ONNX Runtime download failed: ${err.message}. Context injection may not work.`,
    });
    return false;
  }

  onStatus({ phase: "extracting", message: "Extracting ONNX Runtime..." });

  const extractDir = path.join(tmpDir, "extracted");
  try {
    await extractArchive(archivePath, extractDir);
  } catch (err: any) {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    onStatus({
      phase: "verifying",
      message: `ONNX Runtime extraction failed: ${err.message}`,
    });
    return false;
  }

  // Find the library file anywhere in the extracted tree
  let libSrc = findBinaryInExtracted(extractDir, libName);
  if (!libSrc) {
    // Fallback: find any matching lib
    libSrc = findBinaryInExtracted(
      extractDir,
      process.platform === "win32" ? "onnxruntime.dll" : "libonnxruntime",
    );
  }

  if (libSrc && fs.existsSync(libSrc)) {
    const dest = path.join(destDir, path.basename(libSrc));
    fs.copyFileSync(libSrc, dest);

    // On Windows, also copy the provider shared DLL if present
    if (process.platform === "win32") {
      const providerDll = findBinaryInExtracted(
        extractDir,
        "onnxruntime_providers_shared.dll",
      );
      if (providerDll) {
        fs.copyFileSync(
          providerDll,
          path.join(destDir, "onnxruntime_providers_shared.dll"),
        );
      }
    }

    // On Unix: create an unversioned symlink (libonnxruntime.so -> libonnxruntime.so.1.23.0)
    if (process.platform !== "win32") {
      const link = path.join(
        destDir,
        process.platform === "darwin"
          ? "libonnxruntime.dylib"
          : "libonnxruntime.so",
      );
      try {
        if (fs.existsSync(link)) fs.unlinkSync(link);
        fs.symlinkSync(path.basename(libSrc), link);
      } catch {
        // Symlink failure is non-fatal; some systems load by full versioned name
      }
    }
  } else {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    onStatus({
      phase: "verifying",
      message: "ONNX Runtime library file not found in downloaded archive.",
    });
    return false;
  }

  fs.rmSync(tmpDir, { recursive: true, force: true });
  return true;
}

function findBinaryInExtracted(dir: string, name: string): string | null {
  // Try flat layout first
  const flat = path.join(dir, name);
  if (fs.existsSync(flat)) {
    return flat;
  }

  // Recursive search (handles nested dirs)
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isDirectory()) {
      const found = findBinaryInExtracted(path.join(dir, entry.name), name);
      if (found) {
        return found;
      }
    } else if (entry.name === name || entry.name.startsWith(name)) {
      return path.join(dir, entry.name);
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Main bootstrap entry point
// ---------------------------------------------------------------------------

export async function bootstrap(
  context: vscode.ExtensionContext,
  onStatus: StatusCallback,
): Promise<BootstrapResult> {
  const expectedVersion = String(context.extension.packageJSON.version || "");

  onStatus({
    phase: "checking",
    message: "Checking for OmniContext engine...",
  });

  // Step 1: Try to resolve from known locations
  const existing = await resolveBinaries(context);
  if (existing) {
    const installedVersion = parseSemver(tryRunVersion(existing.cliBinary));
    const versionRelation =
      installedVersion && expectedVersion
        ? compareSemver(installedVersion, expectedVersion)
        : 0;

    if (installedVersion && expectedVersion && versionRelation < 0) {
      onStatus({
        phase: "checking",
        message: `Found OmniContext ${installedVersion}; updating to ${expectedVersion}...`,
      });
      // Continue to download/install path below.
    } else {
      if (installedVersion && expectedVersion && versionRelation > 0) {
        onStatus({
          phase: "checking",
          message: `Found newer OmniContext ${installedVersion}; keeping existing binary.`,
        });
      }

      if (!existing.onnxDllPresent) {
        // Active repair: download ONNX Runtime into the same directory as the binary
        const binDir = path.dirname(existing.cliBinary);
        onStatus({
          phase: "downloading",
          message: "Engine found. ONNX Runtime missing — downloading now...",
        });
        const ok = await downloadOnnxRuntime(binDir, onStatus);
        if (ok) {
          onStatus({ phase: "ready", message: "OmniContext engine is ready." });
          return { ...existing, onnxDllPresent: true };
        } else {
          onStatus({
            phase: "ready",
            message:
              "ONNX Runtime download failed. Context injection may not work. Use Repair in sidebar.",
          });
        }
      } else {
        onStatus({ phase: "ready", message: "OmniContext engine is ready." });
      }
      return existing;
    }
  }

  // Step 2: Auto-download from GitHub Releases
  const version = context.extension.packageJSON.version;
  let tag = `v${version}`;

  onStatus({
    phase: "checking",
    message: `Resolving engine version ${tag}...`,
  });

  // Attempt to build URL; if it doesn't work, we'll catch during download
  const downloadUrl = buildAssetUrl(tag);
  const ext = process.platform === "win32" ? ".zip" : ".tar.gz";
  const archiveName = `omnicontext-${tag}${ext}`;
  const downloadDir = context.globalStoragePath;
  fs.mkdirSync(downloadDir, { recursive: true });
  const archivePath = path.join(downloadDir, archiveName);
  const extractDir = path.join(downloadDir, "extracted");
  const finalBinDir = path.join(downloadDir, "bin");

  onStatus({
    phase: "downloading",
    message: `Downloading OmniContext ${tag}...`,
    progressPercent: 0,
  });

  try {
    await downloadFile(downloadUrl, archivePath, (percent) => {
      onStatus({
        phase: "downloading",
        message: `Downloading OmniContext ${tag}... ${percent}%`,
        progressPercent: percent,
      });
    });
  } catch (err: any) {
    onStatus({
      phase: "failed",
      message: `Download failed: ${err.message}. URL: ${downloadUrl}`,
    });
    throw err;
  }

  onStatus({ phase: "extracting", message: "Extracting binaries..." });

  try {
    if (fs.existsSync(extractDir)) {
      fs.rmSync(extractDir, { recursive: true, force: true });
    }
    await extractArchive(archivePath, extractDir);
    fs.rmSync(archivePath, { force: true }); // cleanup archive
  } catch (err: any) {
    onStatus({ phase: "failed", message: `Extraction failed: ${err.message}` });
    throw err;
  }

  // Copy binaries to the stable bin dir
  fs.mkdirSync(finalBinDir, { recursive: true });
  const names = getBinNames();

  onStatus({ phase: "verifying", message: "Installing binaries..." });

  for (const nameKey of ["cli", "daemon", "mcp"] as const) {
    const name = names[nameKey];
    const src = findBinaryInExtracted(extractDir, name);
    if (src) {
      const dest = path.join(finalBinDir, name);
      fs.copyFileSync(src, dest);
      // Set executable bit on Unix
      if (process.platform !== "win32") {
        fs.chmodSync(dest, 0o755);
      }
    }
  }

  // After binary install: download ONNX Runtime if not in archive
  if (process.platform === "win32") {
    const onnxSrc = findBinaryInExtracted(extractDir, "onnxruntime.dll");
    if (onnxSrc) {
      fs.copyFileSync(onnxSrc, path.join(finalBinDir, "onnxruntime.dll"));
    }
  } else {
    // On Unix, copy any libonnxruntime files found in archive
    const libExt = process.platform === "darwin" ? ".dylib" : ".so";
    const libSrc = findBinaryInExtracted(extractDir, `libonnxruntime`);
    if (libSrc) {
      fs.copyFileSync(libSrc, path.join(finalBinDir, path.basename(libSrc)));
    }
  }

  // Cleanup extracted dir
  fs.rmSync(extractDir, { recursive: true, force: true });

  onStatus({ phase: "verifying", message: "Verifying installation..." });

  const result = await resolveBinaries(context);
  if (!result) {
    const msg =
      "Binary installation succeeded but verification failed. Try restarting VS Code.";
    onStatus({ phase: "failed", message: msg });
    throw new Error(msg);
  }

  // If ONNX Runtime wasn't bundled in the release, download it now
  if (!result.onnxDllPresent) {
    onStatus({
      phase: "downloading",
      message:
        "ONNX Runtime not bundled in release — downloading from Microsoft...",
    });
    const onnxOk = await downloadOnnxRuntime(finalBinDir, onStatus);
    onStatus({
      phase: "ready",
      message: onnxOk
        ? `OmniContext ${tag} installed and ready.`
        : `OmniContext ${tag} installed. ONNX Runtime download failed — run Repair in the sidebar.`,
    });
    return { ...result, onnxDllPresent: onnxOk };
  }

  onStatus({
    phase: "ready",
    message: `OmniContext ${tag} installed and ready.`,
  });

  // On first install (wasn't already resolved), run setup --all to configure
  // all detected AI IDEs with the universal MCP entry.
  if (!existing) {
    runSetupAllSilent(result.cliBinary, onStatus);
  }

  return result;
}

// ---------------------------------------------------------------------------
// Auto-update: background check + silent upgrade
// ---------------------------------------------------------------------------

/**
 * Check if a newer version is available on GitHub and silently update if so.
 *
 * Called once per VS Code session, 30 seconds after activation (so it doesn't
 * block startup). Shows a notification if an update was applied.
 *
 * @param context - Extension context for globalStoragePath
 * @param currentBinary - Path to the currently active CLI binary
 * @param currentVersion - Semver string of installed binary (e.g., "1.1.1")
 * @param onStatus - Status callback for progress reporting
 */
export async function checkAndApplyUpdate(
  context: vscode.ExtensionContext,
  currentBinary: string,
  currentVersion: string,
  onStatus: StatusCallback,
): Promise<boolean> {
  try {
    onStatus({
      phase: "checking",
      message: "Checking for OmniContext updates...",
    });

    const latestTag = await fetchLatestReleaseTag();
    const latestVersion = parseSemver(latestTag);

    if (!latestVersion) {
      return false;
    }

    if (compareSemver(latestVersion, currentVersion) <= 0) {
      onStatus({
        phase: "ready",
        message: `OmniContext ${currentVersion} is up to date.`,
      });
      return false; // already current
    }

    onStatus({
      phase: "downloading",
      message: `Updating OmniContext ${currentVersion} → ${latestVersion}...`,
      progressPercent: 0,
    });

    // Back up the current binary before downloading
    const binDir = path.dirname(currentBinary);
    const names = getBinNames();
    const backups: Array<{ src: string; bak: string }> = [];

    for (const nameKey of ["cli", "daemon", "mcp"] as const) {
      const binPath = path.join(binDir, names[nameKey]);
      const bakPath = `${binPath}.bak`;
      if (fs.existsSync(binPath)) {
        try {
          fs.copyFileSync(binPath, bakPath);
          backups.push({ src: binPath, bak: bakPath });
        } catch {
          // Non-fatal: proceed without backup
        }
      }
    }

    const downloadUrl = buildAssetUrl(latestTag);
    const ext = process.platform === "win32" ? ".zip" : ".tar.gz";
    const archivePath = path.join(context.globalStoragePath, `omnicontext-update${ext}`);
    const extractDir = path.join(context.globalStoragePath, "update-extracted");

    try {
      await downloadFile(downloadUrl, archivePath, (percent) => {
        onStatus({
          phase: "downloading",
          message: `Downloading OmniContext ${latestVersion}... ${percent}%`,
          progressPercent: percent,
        });
      });

      onStatus({ phase: "extracting", message: "Extracting update..." });

      if (fs.existsSync(extractDir)) {
        fs.rmSync(extractDir, { recursive: true, force: true });
      }
      await extractArchive(archivePath, extractDir);
      fs.rmSync(archivePath, { force: true });

      onStatus({ phase: "verifying", message: "Installing update..." });

      for (const nameKey of ["cli", "daemon", "mcp"] as const) {
        const name = names[nameKey];
        const src = findBinaryInExtracted(extractDir, name);
        if (src) {
          const dest = path.join(binDir, name);
          fs.copyFileSync(src, dest);
          if (process.platform !== "win32") {
            fs.chmodSync(dest, 0o755);
          }
        }
      }

      fs.rmSync(extractDir, { recursive: true, force: true });

      // Clean up backups on success
      for (const { bak } of backups) {
        try { fs.unlinkSync(bak); } catch { /* non-fatal */ }
      }

      onStatus({
        phase: "ready",
        message: `OmniContext updated to ${latestVersion}.`,
      });

      // Re-run setup --all after update so any new IDE entries are applied
      runSetupAllSilent(path.join(binDir, names.cli), onStatus);

      vscode.window.showInformationMessage(
        `OmniContext updated to v${latestVersion}. Restart VS Code to use the new version.`,
        "Restart Now",
      ).then((choice) => {
        if (choice === "Restart Now") {
          vscode.commands.executeCommand("workbench.action.reloadWindow");
        }
      });

      return true;
    } catch (err: any) {
      // Restore backups on failure
      for (const { src, bak } of backups) {
        try {
          if (fs.existsSync(bak)) fs.copyFileSync(bak, src);
        } catch { /* non-fatal */ }
      }
      for (const { bak } of backups) {
        try { fs.unlinkSync(bak); } catch { /* non-fatal */ }
      }

      onStatus({
        phase: "ready",
        message: `Update failed: ${err.message}. Keeping ${currentVersion}.`,
      });

      return false;
    }
  } catch {
    // Network errors are non-fatal for update checks
    return false;
  }
}

/**
 * Schedule a background update check 30 seconds after activation.
 * Non-blocking — never delays extension startup.
 */
export function scheduleBackgroundUpdateCheck(
  context: vscode.ExtensionContext,
  currentBinary: string,
  currentVersion: string,
): void {
  // Only check once per session, 30s after activation
  const timer = setTimeout(async () => {
    // Respect user preference
    const config = vscode.workspace.getConfiguration("omnicontext");
    if (config.get<boolean>("autoUpdate", true) === false) {
      return;
    }

    const outputCh = vscode.window.createOutputChannel("OmniContext Updates");
    try {
      const updated = await checkAndApplyUpdate(
        context,
        currentBinary,
        currentVersion,
        (status) => {
          if (status.phase !== "ready" && status.phase !== "checking") {
            outputCh.appendLine(`[auto-update] ${status.phase}: ${status.message}`);
          }
        },
      );
      if (updated) {
        outputCh.appendLine(`[auto-update] Updated successfully.`);
      }
    } catch {
      // Swallow all errors — update check is best-effort
    }
  }, 30_000);

  // Cancel the timer if extension is deactivated before it fires
  context.subscriptions.push({ dispose: () => clearTimeout(timer) });
}

/**
 * Run `omnicontext setup --all` silently in the background.
 * Called after first install and after updates to wire all IDEs.
 */
function runSetupAllSilent(cliBinary: string, onStatus: StatusCallback): void {
  try {
    const { execFile } = require("child_process") as typeof import("child_process");
    execFile(
      cliBinary,
      ["setup", "--all"],
      { timeout: 30_000 },
      (err: Error | null, stdout: string, stderr: string) => {
        if (err) {
          onStatus({
            phase: "ready",
            message: `IDE auto-configuration (setup --all) failed: ${err.message}`,
          });
        } else {
          onStatus({
            phase: "ready",
            message: "All detected AI IDEs configured with OmniContext MCP.",
          });
          const linesConfigured = (stdout + stderr)
            .split("\n")
            .filter((l) => l.includes("✓ wired")).length;
          if (linesConfigured > 0) {
            vscode.window.showInformationMessage(
              `OmniContext configured in ${linesConfigured} AI IDE(s). Restart them to activate.`,
            );
          }
        }
      },
    );
  } catch {
    // Non-fatal
  }
}

// ---------------------------------------------------------------------------
// Uninstall cleanup helper
// ---------------------------------------------------------------------------

/**
 * Remove all binaries installed by the extension into globalStoragePath/bin.
 * Called from the extension's deactivate or a manual "Uninstall Engine" command.
 * Does NOT remove binaries installed by the standalone install script
 * (those are in ~/.omnicontext/bin and managed by the user).
 */
export function cleanupInstalledBinaries(
  context: vscode.ExtensionContext,
): void {
  const downloadedBinDir = path.join(context.globalStoragePath, "bin");
  if (fs.existsSync(downloadedBinDir)) {
    try {
      fs.rmSync(downloadedBinDir, { recursive: true, force: true });
    } catch {
      // Non-fatal
    }
  }
}
