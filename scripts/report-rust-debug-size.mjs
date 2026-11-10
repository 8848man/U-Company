import { appendFile, lstat, mkdir, readdir, statfs } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const DEBUG_BUDGET_BYTES = 5 * 1024 ** 3;

async function walk(directory, totals, part) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    const info = await lstat(fullPath);
    if (info.isSymbolicLink()) continue;
    if (info.isDirectory()) {
      await walk(fullPath, totals, part);
    } else if (info.isFile()) {
      totals.total += info.size;
      totals.parts[part] = (totals.parts[part] ?? 0) + info.size;
      if (entry.name.toLowerCase().endsWith('.pdb')) totals.pdb += info.size;
      totals.files++;
    }
  }
}

export async function measureDebug(directory) {
  const totals = { total: 0, parts: {}, pdb: 0, files: 0 };
  const info = await lstat(directory).catch((error) => {
    if (error.code === 'ENOENT') return null;
    throw error;
  });
  if (!info?.isDirectory() || info.isSymbolicLink()) return null;
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const fullPath = path.join(directory, entry.name);
    const item = await lstat(fullPath);
    if (item.isSymbolicLink()) continue;
    if (item.isDirectory()) await walk(fullPath, totals, entry.name);
    else if (item.isFile()) {
      totals.total += item.size;
      totals.parts.root = (totals.parts.root ?? 0) + item.size;
      if (entry.name.toLowerCase().endsWith('.pdb')) totals.pdb += item.size;
      totals.files++;
    }
  }
  let freeBytes = null;
  try {
    const disk = await statfs(directory);
    freeBytes = disk.bavail * disk.bsize;
  } catch { /* The size report still works if the filesystem omits free space. */ }
  return { ...totals, freeBytes };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const debug = path.resolve(process.argv.find((argument) => argument.startsWith('--debug-dir='))?.slice(12)
    ?? path.join(repo, 'src-tauri', 'target', 'debug'));
  const label = process.argv.find((argument) => argument.startsWith('--record='))?.slice(9);
  try {
    const result = await measureDebug(debug);
    if (!result) {
      console.log('Rust debug size: no build output yet.');
    } else {
      const gib = (value) => (value / 1024 ** 3).toFixed(2);
      const parts = Object.entries(result.parts).sort((a, b) => b[1] - a[1])
        .map(([name, bytes]) => `${name}=${gib(bytes)}`).join(' ');
      console.log(`Rust debug size: total=${gib(result.total)} GiB ${parts} pdb=${gib(result.pdb)} GiB`);
      console.log(`Rust debug capacity: ${result.total <= DEBUG_BUDGET_BYTES ? 'within 5 GiB budget' : 'review needed (over 5 GiB; check legacy artifacts first)'}`);
      if (result.freeBytes !== null) console.log(`Disk free: ${gib(result.freeBytes)} GiB`);
      if (label) {
        const logDir = path.join(repo, '.run-data');
        await mkdir(logDir, { recursive: true });
        await appendFile(path.join(logDir, 'rust-build-storage.jsonl'),
          `${JSON.stringify({ at: new Date().toISOString(), label, debug, ...result })}\n`);
      }
    }
  } catch (error) {
    console.warn(`Rust debug size: measurement unavailable (${error.message}).`);
    process.exitCode = 2;
  }
}
