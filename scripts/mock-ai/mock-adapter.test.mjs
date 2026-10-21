import { describe, expect, it } from 'vitest';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs, respond, reviewItems, stageOf } from './mock-adapter.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const scenario = JSON.parse(readFileSync(join(here, 'scenarios', 'todo-app.json'), 'utf8'));
const options = (modes = []) => ({ ...parseArgs(['--state', mkdtempSync(join(tmpdir(), 'mock-state-'))]), modes: new Set(modes) });

describe('mock AI adapter', () => {
  it('speaks plugin protocol v1 over stdio', () => {
    const input = [
      JSON.stringify({ id: 1, method: 'initialize', params: { protocolVersion: 1 } }),
      JSON.stringify({ id: 2, method: 'send_task', params: { prompt: 'You are 민준 (CTO) in an AI company. Analyze the CEO goal and project.', cwd: tmpdir(), writable: false } }),
    ].join('\n') + '\n';
    const run = spawnSync(process.execPath, [join(here, 'mock-adapter.mjs')], { input, encoding: 'utf8' });
    const [init, reply] = run.stdout.trim().split('\n').map(line => JSON.parse(line));
    expect(init.result.protocolVersion).toBe(1);
    expect(reply.result.text.startsWith('요약: ')).toBe(true);
    expect(reply.result.text).toContain('업무: 개발');
    expect(reply.result.usage.input_tokens).toBeGreaterThan(0);
  });

  it('recognizes each engine stage from its prompt wording', () => {
    expect(stageOf('... Decompose the CEO goal into remaining FILE-CHANGING implementation tasks ...')).toBe('plan');
    expect(stageOf('You are x planning repairs after QA rejected the team')).toBe('repair_plan');
    expect(stageOf('Work only inside this isolated task folder and change ONLY these declared files: a.js. The engine')).toBe('implement');
    expect(stageOf("independently reviewing the team's deliverables for this CEO goal")).toBe('review_deliverables');
    expect(stageOf('Return ONLY JSON: {"kind":"discuss|direction"}')).toBe('conversation_decision');
  });

  it('plans the scenario tasks, and breaks the first plan once in plan-form-once', () => {
    const prompt = 'Decompose the CEO goal into remaining FILE-CHANGING implementation tasks';
    expect(JSON.parse(respond(prompt, tmpdir(), options(), scenario)).tasks).toHaveLength(2);
    const broken = JSON.parse(respond(prompt, tmpdir(), options(['plan-form-once']), scenario));
    expect(broken.tasks[0].acceptance).toHaveLength(9);
    const fixed = JSON.parse(respond(`${prompt}\n\nYour previous answer was rejected by the app's format check: x`, tmpdir(), options(['plan-form-once']), scenario));
    expect(fixed.tasks[0].acceptance).toHaveLength(3);
  });

  it('writes declared files whose tests pass with node --test', () => {
    const folder = mkdtempSync(join(tmpdir(), 'mock-work-'));
    const files = scenario.tasks.flatMap(task => task.files);
    respond(`Work only inside this isolated task folder and change ONLY these declared files: ${files.join(', ')}. The engine rejects`, folder, options(), scenario);
    for (const file of files) expect(existsSync(join(folder, file))).toBe(true);
    const test = spawnSync(process.execPath, ['--test'], { cwd: folder, encoding: 'utf8' });
    expect(test.status, test.stdout + test.stderr).toBe(0);
  });

  it('never writes outside the task folder', () => {
    const folder = mkdtempSync(join(tmpdir(), 'mock-work-'));
    const text = respond('Work only inside this isolated task folder and change ONLY these declared files: ../escape.txt. The engine', folder, options(), scenario);
    expect(text).toContain('0개');
    expect(existsSync(join(folder, '..', 'escape.txt'))).toBe(false);
  });

  it('judges every criterion, failing once in qa-fail-once', () => {
    const folder = mkdtempSync(join(tmpdir(), 'mock-qa-'));
    respond('Work only inside this isolated task folder and change ONLY these declared files: todo.js, todo.test.js. The engine', folder, options(), scenario);
    const prompt = "independently reviewing the team's deliverables for this CEO goal: 앱. Inspect\nDeliverables:\n- taskId: t1\n  deliverable: d\n  files: todo.js, todo.test.js\n  evidence (read-only inputs): none\n  acceptance criteria (answer in this order):\n    1. a\n    2. b\n    3. c\nNewly integrated diff summary:\n";
    expect(reviewItems(prompt)).toEqual([{ taskId: 't1', files: ['todo.js', 'todo.test.js'], criteria: 3 }]);
    const modes = options(['qa-fail-once']);
    const first = JSON.parse(respond(prompt, folder, modes, scenario));
    expect(first.verdict).toBe('FAIL');
    expect(first.deliverables[0].criteria).toHaveLength(3);
    expect(JSON.parse(respond(prompt, folder, modes, scenario)).verdict).toBe('PASS');
  });

  it('asks for README.md in a repair until it is approved (needs-scope-once)', () => {
    const repair = approved => `planning repairs after QA rejected the team's integrated work. Use these CEO-approved files: ${approved}. A new file needs the CEO's approval`;
    const asking = JSON.parse(respond(repair('todo.js, todo.test.js'), tmpdir(), options(['needs-scope-once']), scenario));
    expect(asking.tasks[0].files).toContain('README.md');
    const inScope = JSON.parse(respond(repair('todo.js, todo.test.js, readme.md'), tmpdir(), options(['needs-scope-once']), scenario));
    expect(inScope.tasks[0].files).toEqual(['todo.js', 'todo.test.js', 'README.md']);
    const plain = JSON.parse(respond(repair('todo.js, todo.test.js'), tmpdir(), options(), scenario));
    expect(plain.tasks[0].files).toEqual(['todo.js', 'todo.test.js']);
  });
});
