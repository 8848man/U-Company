import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { DEBUG_BUDGET_BYTES, measureDebug } from './report-rust-debug-size.mjs';

test('counts debug output by area and PDB bytes without double counting', async () => {
  const base = await mkdtemp(path.join(os.tmpdir(), 'ucompany-size-'));
  try {
    await mkdir(path.join(base, 'deps'));
    await mkdir(path.join(base, 'incremental'));
    await writeFile(path.join(base, 'app.pdb'), Buffer.alloc(10));
    await writeFile(path.join(base, 'deps', 'library.pdb'), Buffer.alloc(20));
    await writeFile(path.join(base, 'incremental', 'cache.o'), Buffer.alloc(30));
    const result = await measureDebug(base);
    assert.equal(result.total, 60);
    assert.equal(result.parts.root, 10);
    assert.equal(result.parts.deps, 20);
    assert.equal(result.parts.incremental, 30);
    assert.equal(result.pdb, 30);
    assert.equal(result.files, 3);
    assert.equal(DEBUG_BUDGET_BYTES, 5 * 1024 ** 3);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});
