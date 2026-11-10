import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, mkdir, readFile, readdir, rm, utimes, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { pruneIncremental } from './clean-rust-incremental.mjs';

test('removes only old cache variants when over the limit', async () => {
  const base = await mkdtemp(path.join(os.tmpdir(), 'ucompany-cache-'));
  const root = path.join(base, 'incremental');
  const now = Date.now();
  try {
    await mkdir(root);
    for (const [name, size, age] of [
      ['app-aaaaaaaa', 100, 20],
      ['app-bbbbbbbb', 20, 2],
      ['app-cccccccc', 20, 1],
    ]) {
      const folder = path.join(root, name);
      await mkdir(folder);
      const file = path.join(folder, 'cache.o');
      await writeFile(file, Buffer.alloc(size));
      const time = new Date(now - age * 24 * 60 * 60 * 1000);
      await utimes(file, time, time);
    }
    const result = await pruneIncremental(root, {
      maxBytes: 50, minAgeMs: 7 * 24 * 60 * 60 * 1000,
      now, activeProcessCheck: () => false,
    });
    assert.deepEqual(result.removed, ['app-aaaaaaaa']);
    assert.equal(result.freedBytes, 100);
    assert.equal((await readFile(path.join(root, 'app-bbbbbbbb', 'cache.o'))).length, 20);
    assert.equal((await readFile(path.join(root, 'app-cccccccc', 'cache.o'))).length, 20);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test('does not inspect or delete caches while Cargo is active', async () => {
  const base = await mkdtemp(path.join(os.tmpdir(), 'ucompany-cache-'));
  const root = path.join(base, 'incremental');
  try {
    await mkdir(root);
    await mkdir(path.join(root, 'app-aaaaaaaa'));
    const result = await pruneIncremental(root, { maxBytes: 0, activeProcessCheck: () => true });
    assert.equal(result.status, 'busy');
    assert.deepEqual(await readdir(root), ['app-aaaaaaaa']);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});
