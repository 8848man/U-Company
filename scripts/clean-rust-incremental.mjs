import { execFileSync } from 'node:child_process';
import { lstat, readdir, rm } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const MAX_BYTES = 4 * 1024 ** 3;
const MIN_AGE_MS = 7 * 24 * 60 * 60 * 1000;

function cargoIsRunning() {
  const windows = process.platform === 'win32';
  const output = windows
    ? execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
      '(Get-Process -Name cargo,rustc -ErrorAction SilentlyContinue | Measure-Object).Count'], { encoding: 'utf8' })
    : execFileSync('ps', ['-eo', 'comm='], { encoding: 'utf8' });
  return windows
    ? Number(output.trim()) > 0
    : /^(?:cargo|rustc)(?:\.exe)?$/im.test(output);
}

async function inspectTree(directory) {
  let bytes = 0;
  let newest = 0;
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    const info = await lstat(fullPath);
    if (info.isSymbolicLink()) return null;
    if (info.isDirectory()) {
      const child = await inspectTree(fullPath);
      if (!child) return null;
      bytes += child.bytes;
      newest = Math.max(newest, child.newest);
    } else if (info.isFile()) {
      bytes += info.size;
      newest = Math.max(newest, info.mtimeMs);
    }
  }
  return { bytes, newest };
}

export async function pruneIncremental(root, options = {}) {
  const { maxBytes = MAX_BYTES, minAgeMs = MIN_AGE_MS, now = Date.now(),
    activeProcessCheck = cargoIsRunning, remove = rm } = options;
  const rootInfo = await lstat(root).catch((error) => {
    if (error.code === 'ENOENT') return null;
    throw error;
  });
  if (rootInfo?.isSymbolicLink()) return { status: 'symlink-root' };
  if (!rootInfo?.isDirectory()) return { status: 'missing' };
  if (activeProcessCheck()) return { status: 'busy' };

  const folders = [];
  let totalBytes = 0;
  for (const entry of await readdir(root, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const fullPath = path.join(root, entry.name);
    const contents = await inspectTree(fullPath);
    if (!contents) continue;
    totalBytes += contents.bytes;
    if (!/^[A-Za-z0-9_]+-[a-z0-9]{8,}$/.test(entry.name)) continue;
    folders.push({ ...contents, fullPath, name: entry.name,
      crate: entry.name.slice(0, entry.name.lastIndexOf('-')) });
  }
  const beforeBytes = totalBytes;
  if (totalBytes <= maxBytes) return { status: 'below-limit', beforeBytes, freedBytes: 0, removed: [] };

  // Keep the two most recent variants of every crate, regardless of age.
  const protectedPaths = new Set();
  for (const crate of new Set(folders.map((folder) => folder.crate))) {
    folders.filter((folder) => folder.crate === crate)
      .sort((a, b) => b.newest - a.newest)
      .slice(0, 2).forEach((folder) => protectedPaths.add(folder.fullPath));
  }
  const removed = [];
  for (const folder of folders.sort((a, b) => a.newest - b.newest)) {
    if (totalBytes <= maxBytes) break;
    if (protectedPaths.has(folder.fullPath) || now - folder.newest < minAgeMs) continue;
    if (activeProcessCheck()) break;
    // Recheck immediately before removal; a newly active cache must survive.
    const current = await inspectTree(folder.fullPath);
    if (!current || current.newest !== folder.newest || now - current.newest < minAgeMs) continue;
    await remove(folder.fullPath, { recursive: true, force: false });
    totalBytes -= current.bytes;
    removed.push(folder.name);
  }
  return { status: 'done', beforeBytes, freedBytes: beforeBytes - totalBytes, removed };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)),
    '../src-tauri/target/debug/incremental');
  try {
    const result = await pruneIncremental(root);
    if (result.status === 'busy') console.log('Rust cache cleanup skipped: Cargo or rustc is running.');
    if (result.removed?.length) console.log(`Rust cache cleanup: removed ${result.removed.length} old folders, freed ${(result.freedBytes / 1024 ** 3).toFixed(2)} GiB.`);
  } catch (error) {
    // Cache maintenance must never prevent a build from starting or finishing.
    console.warn(`Rust cache cleanup skipped: ${error.message}`);
  }
}
