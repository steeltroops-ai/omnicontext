/**
 * Repository Registry -- discovers and manages all indexed OmniContext repositories.
 *
 * Maintains a persistent `registry.json` in the OmniContext data directory
 * that maps repository paths to their index metadata. This allows the sidebar
 * to show all indexed repos, their status, and which one is currently active.
 *
 * Data dir: %LOCALAPPDATA%/omnicontext (Windows) or ~/.local/share/omnicontext (Linux/macOS)
 */

import * as fs from "fs";
import * as path from "path";
import * as crypto from "crypto";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface IndexedRepo {
  /** Absolute path to the repository root. */
  repoPath: string;
  /** Display name (folder name). */
  name: string;
  /** SHA-256 hash prefix used as directory name in the data dir. */
  hash: string;
  /** Number of files indexed (last known). */
  filesIndexed: number;
  /** Number of chunks indexed (last known). */
  chunksIndexed: number;
  /** Timestamp of last index operation (epoch ms). */
  lastIndexedAt: number;
  /** Whether the repo path still exists on disk. */
  exists: boolean;
}

export interface RepoRegistryData {
  version: number;
  repos: Record<string, IndexedRepoEntry>;
}

interface IndexedRepoEntry {
  repoPath: string;
  name: string;
  hash: string;
  filesIndexed: number;
  chunksIndexed: number;
  lastIndexedAt: number;
}

// ---------------------------------------------------------------------------
// Registry implementation
// ---------------------------------------------------------------------------

/**
 * Get the OmniContext data directory.
 */
function getOmniDataDir(): string {
  const isWin = process.platform === "win32";
  if (isWin) {
    const localAppData =
      process.env.LOCALAPPDATA ||
      path.join(
        process.env.USERPROFILE || "C:\\Users\\Default",
        "AppData",
        "Local",
      );
    return path.join(localAppData, "omnicontext");
  }
  const home = process.env.HOME || "/tmp";
  return path.join(home, ".local", "share", "omnicontext");
}

/**
 * Get the path to the registry file.
 */
function getRegistryPath(): string {
  return path.join(getOmniDataDir(), "registry.json");
}

/**
 * Compute the same SHA-256 hash used by omni-core for the data directory name.
 * Must match `normalize_repo_hash()` in config.rs exactly.
 *
 * Normalization steps (must be identical in Rust and TypeScript):
 *   1. Strip Windows extended path prefix (\\?\)
 *   2. Replace all backslashes with forward slashes
 *   3. Lowercase the entire string (Windows FS is case-insensitive)
 *   4. Strip trailing separator(s)
 *
 * Without this normalization, the same physical directory produces
 * different hashes depending on how the path is spelled, causing
 * hundreds of duplicate index folders.
 */
export function computeRepoHash(repoPath: string): string {
  let normalized = repoPath;

  // 1. Strip Windows extended path prefix
  if (normalized.startsWith("\\\\?\\")) {
    normalized = normalized.substring(4);
  }

  // 2. Uniform separator: backslash -> forward slash
  normalized = normalized.replace(/\\/g, "/");

  // 3. Case-fold (Windows FS is case-insensitive)
  normalized = normalized.toLowerCase();

  // 4. Strip trailing separator(s)
  while (normalized.endsWith("/")) {
    normalized = normalized.slice(0, -1);
  }

  const hash = crypto.createHash("sha256").update(normalized).digest("hex");
  return hash.substring(0, 8);
}

/**
 * Load the registry from disk.
 */
function loadRegistry(): RepoRegistryData {
  const registryPath = getRegistryPath();
  if (!fs.existsSync(registryPath)) {
    return { version: 1, repos: {} };
  }
  try {
    const content = fs.readFileSync(registryPath, "utf-8");
    const data = JSON.parse(content) as RepoRegistryData;
    return data;
  } catch {
    return { version: 1, repos: {} };
  }
}

/**
 * Save the registry to disk.
 */
function saveRegistry(data: RepoRegistryData): void {
  const registryPath = getRegistryPath();
  const dir = path.dirname(registryPath);

  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }

  fs.writeFileSync(registryPath, JSON.stringify(data, null, 2), "utf-8");
}

/**
 * Register or update a repository in the registry.
 * Called after a successful index operation.
 */
export function registerRepo(
  repoPath: string,
  filesIndexed: number,
  chunksIndexed: number,
): void {
  const registry = loadRegistry();
  const hash = computeRepoHash(repoPath);
  const name = path.basename(repoPath) || repoPath;

  registry.repos[hash] = {
    repoPath,
    name,
    hash,
    filesIndexed,
    chunksIndexed,
    lastIndexedAt: Date.now(),
  };

  saveRegistry(registry);
}

/**
 * Remove a repository from the registry.
 */
export function unregisterRepo(hash: string): void {
  const registry = loadRegistry();
  delete registry.repos[hash];
  saveRegistry(registry);
}

/**
 * Auto-discover repos that exist on disk but are not tracked in the registry.
 * This handles repos indexed via CLI or a previous installation that never
 * created a registry.json entry. For each discovered hash, we check if
 * index.db exists and add a stub entry so the sidebar shows them.
 *
 * We try to resolve friendly names by matching the hash against any
 * workspace folder paths passed in. If none match, the hash is used as the
 * display name.
 */
export function discoverReposFromDisk(knownPaths?: string[]): void {
  const reposDir = getOmniReposDir();
  if (!fs.existsSync(reposDir)) return;

  const registry = loadRegistry();
  let mutated = false;

  const entries = fs.readdirSync(reposDir, { withFileTypes: true });

  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const dirName = entry.name; // e.g. "80aa8fb8"

    // Already tracked
    if (dirName in registry.repos) continue;

    // Only consider directories that actually have an index.db
    const dbPath = path.join(reposDir, dirName, "index.db");
    if (!fs.existsSync(dbPath)) continue;

    // Try to resolve which workspace folder this hash belongs to
    let resolvedPath = "";
    let resolvedName = dirName; // fallback: hash

    if (knownPaths) {
      for (const p of knownPaths) {
        if (computeRepoHash(p) === dirName) {
          resolvedPath = p;
          resolvedName = path.basename(p) || p;
          break;
        }
      }
    }

    // Get file stats for a rough "last indexed" timestamp
    const stat = fs.statSync(dbPath);

    registry.repos[dirName] = {
      repoPath: resolvedPath,
      name: resolvedName,
      hash: dirName,
      filesIndexed: 0, // unknown -- will be populated on next index
      chunksIndexed: 0,
      lastIndexedAt: Math.floor(stat.mtimeMs),
    };
    mutated = true;
  }

  if (mutated) {
    saveRegistry(registry);
  }
}

/**
 * Get all indexed repositories with existence checks.
 */
export function getIndexedRepos(): IndexedRepo[] {
  const registry = loadRegistry();
  const repos: IndexedRepo[] = [];
  let mutated = false;

  for (const entry of Object.values(registry.repos)) {
    // Check if the actual index.db database still exists
    const existsDb = fs.existsSync(
      path.join(getOmniReposDir(), entry.hash, "index.db"),
    );
    if (!existsDb) {
      // The index was deleted on disk (e.g. by cleanup or it was a temporary test repo)
      delete registry.repos[entry.hash];
      mutated = true;
      continue;
    }

    const exists = entry.repoPath ? fs.existsSync(entry.repoPath) : false;
    repos.push({
      ...entry,
      exists,
    });
  }

  if (mutated) {
    saveRegistry(registry);
  }

  // Sort: most recently indexed first
  repos.sort((a, b) => b.lastIndexedAt - a.lastIndexedAt);
  return repos;
}

/**
 * Check if a specific repo path has been indexed.
 */
export function isRepoIndexed(repoPath: string): boolean {
  const hash = computeRepoHash(repoPath);
  const registry = loadRegistry();
  return hash in registry.repos;
}

/**
 * Get info for a specific indexed repo by path.
 */
export function getRepoInfo(repoPath: string): IndexedRepo | null {
  const hash = computeRepoHash(repoPath);
  const registry = loadRegistry();
  const entry = registry.repos[hash];
  if (!entry) return null;
  return {
    ...entry,
    exists: fs.existsSync(entry.repoPath),
  };
}

/**
 * Get the data directory path for a repo's index database.
 */
export function getRepoDataDir(repoPath: string): string {
  const hash = computeRepoHash(repoPath);
  return path.join(getOmniDataDir(), "repos", hash);
}

/**
 * Check if a repo has a valid index on disk (database file exists).
 */
export function hasIndexOnDisk(repoPath: string): boolean {
  const dataDir = getRepoDataDir(repoPath);
  return fs.existsSync(path.join(dataDir, "index.db"));
}

/**
 * Get the repos directory path.
 */
export function getOmniReposDir(): string {
  return path.join(getOmniDataDir(), "repos");
}

/**
 * Purge orphaned index directories that are not tracked in the registry.
 *
 * Returns the list of removed directory hashes. This is the cleanup for
 * the duplicate-index-folder bug caused by inconsistent path normalization.
 */
export function purgeOrphanedIndexes(): {
  removed: string[];
  kept: string[];
  errors: string[];
} {
  const registry = loadRegistry();
  const reposDir = getOmniReposDir();
  const removed: string[] = [];
  const kept: string[] = [];
  const errors: string[] = [];

  if (!fs.existsSync(reposDir)) {
    return { removed, kept, errors };
  }

  const entries = fs.readdirSync(reposDir, { withFileTypes: true });

  for (const entry of entries) {
    if (!entry.isDirectory()) continue;

    const dirName = entry.name;

    if (dirName in registry.repos) {
      // This is a known, registered repo -- keep it
      kept.push(dirName);
    } else {
      // Orphaned directory -- not tracked in registry
      const dirPath = path.join(reposDir, dirName);
      try {
        fs.rmSync(dirPath, { recursive: true, force: true });
        removed.push(dirName);
      } catch (e: any) {
        errors.push(`${dirName}: ${e.message}`);
      }
    }
  }

  return { removed, kept, errors };
}
