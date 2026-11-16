import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { zstdCompressSync } from 'node:zlib';
import { claudeUsage, codexUsage, collectRun, summarize } from './collect.mjs';

const line = (value) => JSON.stringify(value);

test('claude usage counts each API response once and totals input with cache', () => {
  const usage = { input_tokens: 10, cache_creation_input_tokens: 100, cache_read_input_tokens: 1000, output_tokens: 5 };
  const text = [
    line({ type: 'assistant', message: { id: 'm1', model: 'claude-x', usage } }),
    line({ type: 'assistant', message: { id: 'm1', model: 'claude-x', usage } }),
    line({ type: 'assistant', message: { id: 'm2', model: 'claude-x', usage: { ...usage, cache_creation_input_tokens: 0 } } }),
    line({ type: 'user', message: { content: 'x' } }),
    'not json',
  ].join('\n');
  const sum = claudeUsage([text]);
  assert.equal(sum.responses, 2);
  assert.equal(sum.input, 1110 + 1010);
  assert.equal(sum.cacheRead, 2000);
  assert.equal(sum.output, 10);
  assert.deepEqual(sum.models, ['claude-x']);
});

test('codex usage sums per-response records and falls back to the cumulative total', () => {
  const record = (id, input, cached) => line({ type: 'token_usage_record', payload: { response_id: id, usage: { input_tokens: input, cached_input_tokens: cached, output_tokens: 7, reasoning_output_tokens: 2 } } });
  const sum = codexUsage([[record('r1', 100, 40), record('r2', 200, 150), record('r2', 200, 150)].join('\n')]);
  assert.equal(sum.responses, 2);
  assert.equal(sum.input, 300);
  assert.equal(sum.cacheRead, 190);
  assert.equal(sum.freshInput, 110);
  assert.equal(sum.reasoning, 4);

  const cumulative = codexUsage([line({ type: 'event_msg', payload: { type: 'token_count', info: { total_token_usage: { input_tokens: 500, cached_input_tokens: 400, output_tokens: 9 } } } })]);
  assert.equal(cumulative.responses, null);
  assert.equal(cumulative.input, 500);
});

test('collectRun finds each CLI log by session id', async () => {
  const base = await mkdtemp(path.join(os.tmpdir(), 'ucompany-bench-'));
  try {
    const claude = path.join(base, 'claude');
    const codex = path.join(base, 'codex');
    await mkdir(path.join(claude, 'projects', 'C--tmp-run', 's1', 'subagents'), { recursive: true });
    await writeFile(path.join(claude, 'projects', 'C--tmp-run', 's1.jsonl'), line({ type: 'assistant', message: { id: 'a', usage: { input_tokens: 1, cache_read_input_tokens: 9, output_tokens: 1 } } }));
    await writeFile(path.join(claude, 'projects', 'C--tmp-run', 's1', 'subagents', 'agent-1.jsonl'), line({ type: 'assistant', message: { id: 'b', usage: { input_tokens: 5, output_tokens: 1 } } }));
    await mkdir(path.join(codex, 'sessions', '2026', '11', '17'), { recursive: true });
    await writeFile(path.join(codex, 'sessions', '2026', '11', '17', 'rollout-2026-11-17T00-00-00-s2.jsonl'), line({ type: 'token_usage_record', payload: { response_id: 'r', usage: { input_tokens: 4, cached_input_tokens: 0, output_tokens: 1 } } }));
    const homes = { claude, codex };
    assert.equal(collectRun({ provider: 'claude', sessionId: 's1' }, homes).log.input, 15);
    assert.equal(collectRun({ provider: 'claude', sessionId: 's1' }, homes).log.files, 2);
    assert.equal(collectRun({ provider: 'codex', sessionId: 's2' }, homes).log.input, 4);
    assert.equal(collectRun({ provider: 'codex', sessionId: 'missing' }, homes).log, null);
    await mkdir(path.join(codex, 'sessions', '2026', '10', '21'), { recursive: true });
    await writeFile(path.join(codex, 'sessions', '2026', '10', '21', 'rollout-2026-10-21T00-00-00-s3.jsonl.zst'), zstdCompressSync(Buffer.from(line({ type: 'token_usage_record', payload: { response_id: 'r', usage: { input_tokens: 6, cached_input_tokens: 2, output_tokens: 1 } } }))));
    assert.equal(collectRun({ provider: 'codex', sessionId: 's3' }, homes).log.input, 6);
  } finally {
    await rm(base, { recursive: true, force: true });
  }
});

test('summarize compares median Claude/Codex input per condition', () => {
  const run = (provider, condition, input) => ({ provider, condition, check: { pass: 4, fail: 0 }, undeclaredChanges: [], log: { input, output: 1, responses: 2, cacheShare: 0.5 } });
  const result = summarize([
    run('claude', 'before', 800), run('claude', 'before', 1000), run('claude', 'before', 900),
    run('codex', 'before', 100), run('claude', 'after', 400), run('codex', 'after', 100),
  ]);
  assert.equal(result.multiplier.before, 9);
  assert.equal(result.multiplier.after, 4);
  assert.ok(Math.abs(result.change - (4 / 9 - 1)) < 1e-9);
  assert.equal(result.cells['claude/before'].passed, 3);
  assert.equal(result.cells['claude/before'].inputPerResponse, 450);
});
