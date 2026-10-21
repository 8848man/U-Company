#!/usr/bin/env node
// U컴퍼니 모의 AI (ADR-0052). 플러그인 프로토콜 v1(`initialize` → `send_task`)을 따르는 테스트용 어댑터.
// 요청문으로 단계를 알아보고 시나리오 파일을 그대로 재생한다. 실제 모델이 아니며 토큰을 쓰지 않는다.
//
// 사용: node mock-adapter.mjs [--scenario <json>] [--mode qa-fail-once,plan-form-once,needs-scope-once] [--state <dir>]
//   qa-fail-once     같은 목표의 첫 QA를 FAIL로 내고 다음 QA부터 통과(재작업 경로)
//   plan-form-once   첫 PM 계획을 형식 위반(수용 기준 9개)으로 내고 앱의 재요청에 고쳐 냄(ADR-0048)
//   needs-scope-once 재작업 계획이 README.md를 함께 요구함(승인 범위에 없으면 needs_scope, ADR-0053)
// 의존성 없음. 진단은 stderr로만 쓴다(앱은 stderr를 버린다).

import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve, sep } from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));

export function parseArgs(argv) {
  const options = { scenario: join(here, 'scenarios', 'todo-app.json'), modes: new Set(), state: join(tmpdir(), 'u-company-mock-ai') };
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index + 1];
    if (argv[index] === '--scenario' && value) { options.scenario = resolve(value); index += 1; }
    else if (argv[index] === '--mode' && value) { for (const mode of value.split(',')) if (mode.trim()) options.modes.add(mode.trim()); index += 1; }
    else if (argv[index] === '--state' && value) { options.state = resolve(value); index += 1; }
  }
  return options;
}

/** Which engine stage sent this request, from the fixed wording of the engine's prompts. */
export function stageOf(prompt) {
  if (prompt.includes('Return ONLY JSON: {"kind":"discuss')) return 'conversation_decision';
  if (prompt.includes('READ-ONLY goal conversation') || prompt.includes('READ-ONLY escalation')) return 'conversation';
  if (prompt.includes('reviewing a CEO-approved Git bootstrap')) return 'bootstrap_review';
  if (prompt.includes('planning repairs after QA rejected')) return 'repair_plan';
  if (prompt.includes('Decompose the CEO goal into remaining FILE-CHANGING')) return 'plan';
  if (prompt.includes("maintains this project's U-SDD")) return 'sdd';
  if (prompt.includes('Work only inside this isolated task folder')) return 'implement';
  if (prompt.includes("independently reviewing the team's deliverables")) return 'review_deliverables';
  if (prompt.includes("reviewing the team's work for this CEO goal")) return 'review';
  if (prompt.includes('Analyze the CEO goal and project')) return 'analysis';
  if (prompt.includes('organizing reference material')) return 'curation';
  return 'other';
}

function between(text, start, end) {
  const from = text.indexOf(start);
  if (from < 0) return '';
  const rest = text.slice(from + start.length);
  const to = rest.indexOf(end);
  return (to < 0 ? rest : rest.slice(0, to)).trim();
}

function listAfter(prompt, start, end) {
  return between(prompt, start, end).split(',').map(item => item.trim()).filter(Boolean);
}

function goalOf(prompt) {
  return between(prompt, 'CEO goal: ', '\n') || between(prompt, 'for this CEO goal: ', '. ');
}

/** Tasks and their acceptance-criterion counts from the QA prompt's deliverable list. */
export function reviewItems(prompt) {
  const items = [];
  const section = prompt.slice(prompt.indexOf('Deliverables:\n'));
  for (const line of section.split('\n')) {
    const task = line.match(/^- taskId: (\S+)/);
    if (task) { items.push({ taskId: task[1], files: [], criteria: 0 }); continue; }
    const current = items[items.length - 1];
    if (!current) continue;
    const files = line.match(/^ {2}files: (.*)$/);
    if (files) current.files = files[1].split(',').map(item => item.trim()).filter(Boolean);
    if (/^ {4}\d+\. /.test(line)) current.criteria += 1;
  }
  return items;
}

function counter(options, key) {
  mkdirSync(options.state, { recursive: true });
  const file = join(options.state, `${createHash('sha1').update(key).digest('hex').slice(0, 16)}.count`);
  const count = existsSync(file) ? Number(readFileSync(file, 'utf8')) || 0 : 0;
  writeFileSync(file, String(count + 1));
  return count;
}

function insideFolder(root, relative) {
  const target = resolve(root, relative);
  return target.startsWith(resolve(root) + sep) ? target : null;
}

export function respond(prompt, cwd, options, scenario) {
  const stage = stageOf(prompt);
  switch (stage) {
    case 'conversation_decision':
      return JSON.stringify({ kind: 'discuss', reply: '모의 AI예요. 이 대화는 시험용이라 목표를 바꾸지 않고 답만 해요.' });
    case 'conversation':
      return '모의 AI예요. 저장된 계획을 그대로 따르고 있어요. 실제 판단이 필요하면 실제 AI 연결로 바꿔 주세요.';
    case 'bootstrap_review':
      return 'PASS\n모의 AI 확인: Git 첫 커밋 기록을 그대로 받아들였어요(실제 검사는 하지 않음).';
    case 'analysis':
      return `요약: ${scenario.summary}\n업무: 개발\n${scenario.analysis}\n(모의 AI가 시나리오 ${scenario.name}을 재생했어요.)`;
    case 'plan': {
      const retried = prompt.includes("Your previous answer was rejected by the app's format check");
      const tasks = scenario.tasks.map(task => ({ ...task }));
      if (options.modes.has('plan-form-once') && !retried) {
        tasks[0] = { ...tasks[0], acceptance: Array.from({ length: 9 }, (_, index) => `기준 ${index + 1}`) };
      }
      return JSON.stringify({ tasks, reusedTaskIds: [], squad: { structure: 'standard', reason: '모의 AI: 독립된 파일 묶음 두 개라 기본 팀이면 충분해요' } });
    }
    case 'repair_plan': {
      const approved = listAfter(prompt, 'You may modify ONLY these CEO-approved files: ', '. Never request');
      // The engine lists approved files in lower case; the repair fixes the first task's files.
      const first = scenario.tasks[0];
      const owned = first.files.filter(file => approved.includes(file.toLowerCase()));
      const files = owned.length ? owned : approved.slice(0, 1);
      // needs-scope-once: also ask for README.md, which stays in the repair once it is approved.
      if (options.modes.has('needs-scope-once') || approved.includes('readme.md')) files.push('README.md');
      return JSON.stringify({ tasks: [{ ...first, title: `${first.title} 고치기`, files, dependsOn: [] }] });
    }
    case 'implement': {
      const files = listAfter(prompt, 'change ONLY these declared files: ', '. The engine');
      const written = [];
      const repair = prompt.includes(`\nTask: ${REPAIR_PREFIX}`);
      for (const file of files) {
        const target = insideFolder(cwd, file);
        if (!target) continue;
        const base = scenario.files[file] ?? scenario.files[Object.keys(scenario.files).find(name => name.toLowerCase() === file.toLowerCase()) ?? ''];
        let content = base ?? `# ${file}\n\n모의 AI가 만든 파일이에요. 실행 방법: index.html을 브라우저로 열고, 테스트는 \`node --test\`로 돌려요.\n`;
        // A repair changes the file visibly, as a real fix would (the engine rejects a task with no change).
        if (repair) content += remark(file);
        mkdirSync(dirname(target), { recursive: true });
        writeFileSync(target, content);
        written.push(file);
      }
      return `모의 AI가 파일 ${written.length}개를 만들었어요: ${written.join(', ')}.\n테스트 근거: 시나리오에 포함된 테스트 파일을 그대로 썼어요(모의 AI는 테스트를 실행하지 않음).`;
    }
    case 'review_deliverables': {
      const items = reviewItems(prompt);
      const failNow = options.modes.has('qa-fail-once') && counter(options, `qa:${resolve(cwd)}:${goalOf(prompt)}`) === 0;
      const deliverables = items.map((item, index) => {
        const missing = item.files.filter(file => { const target = insideFolder(cwd, file); return !target || !existsSync(target); });
        const criteria = Array.from({ length: item.criteria }, (_, position) => {
          const met = missing.length === 0 && !(failNow && index === 0 && position === 1);
          return { met, evidence: met ? `${item.files.join(', ')} 존재(모의 AI 확인)` : missing.length ? `없는 파일: ${missing.join(', ')}` : '모의 AI가 일부러 실패로 판정(qa-fail-once)' };
        });
        return { taskId: item.taskId, verdict: criteria.every(c => c.met) ? 'PASS' : 'FAIL', criteria, unverified: ['모의 AI는 파일 존재만 확인해요'] };
      });
      const verdict = deliverables.every(item => item.verdict === 'PASS') ? 'PASS' : 'FAIL';
      return JSON.stringify({ verdict, deliverables, risks: ['모의 AI 판정이라 실제 품질은 확인되지 않았어요'] });
    }
    case 'review':
      return 'PASS\n모의 AI 확인: 재사용 결과를 그대로 받아들였어요.';
    case 'sdd':
      return '모의 AI는 SDD 문서를 바꾸지 않았어요.';
    case 'curation':
      return JSON.stringify({ title: '모의 자료', summary: '모의 AI 요약', tags: [], sections: [], relations: [] });
    default:
      return '모의 AI가 이 요청 종류를 알아보지 못했어요.';
  }
}

const REPAIR_PREFIX = '재작업 ';

function remark(file) {
  if (/\.(js|mjs|css)$/i.test(file)) return '\n/* 재작업 반영(모의 AI) */\n';
  if (/\.html?$/i.test(file)) return '\n<!-- 재작업 반영(모의 AI) -->\n';
  return '\n재작업 반영(모의 AI)\n';
}

function usageFor(prompt, text) {
  return { input_tokens: Math.ceil(prompt.length / 4), output_tokens: Math.ceil(text.length / 4) };
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const scenario = JSON.parse(readFileSync(options.scenario, 'utf8'));
  const lines = createInterface({ input: process.stdin });
  lines.on('line', line => {
    let message;
    try { message = JSON.parse(line); } catch { return; }
    if (message.method === 'initialize') {
      process.stdout.write(`${JSON.stringify({ id: message.id, result: { protocolVersion: 1, name: 'u-company-mock-ai' } })}\n`);
    } else if (message.method === 'send_task') {
      const { prompt = '', cwd = process.cwd() } = message.params ?? {};
      try {
        const text = respond(prompt, cwd, options, scenario);
        process.stdout.write(`${JSON.stringify({ id: message.id, result: { text, sessionId: null, usage: usageFor(prompt, text) } })}\n`);
      } catch (error) {
        process.stderr.write(`${error?.stack ?? error}\n`);
        process.stdout.write(`${JSON.stringify({ id: message.id, error: { message: String(error?.message ?? error) } })}\n`);
      }
    }
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
