import { describe, expect, it } from 'vitest';
import { planCardLabel, planCards, progressLine, stageLines, subject } from './goalStream';
import type { Agent, Approval, Event, Goal, Squad, Task } from './types';

const goal = (status: string): Goal => ({ id: 'g', projectId: 'p', intent: '할 일에 마감일을 넣어줘', status, report: '', createdAt: '2026-10-14T00:00:00Z', resumedFrom: null, reviewLoopsUsed: 0, reviewLoopLimit: 2, workGroup: null });
const agent = (id: string, name: string): Agent => ({ id, name, role: id, instructions: '', reportsTo: null, connectionId: 'c', modelId: '', x: 0, y: 0, enabled: true });
const agents = [agent('cto', '민준'), agent('pm', '서연'), agent('qa', '하늘'), agent('dev-a', '개발자 A'), agent('dev-b', '개발자 B'), agent('dev-c', 'Kim')];
const task = (id: string, assigneeId: string, status: string, title = `업무 ${id}`, files = [`${id}.ts`]): Task => ({ id, goalId: 'g', title, assigneeId, status, files, dependsOn: [], result: '' } as unknown as Task);
const event = (id: number, kind: string, message: string, at = `2026-10-14T00:0${id}:00Z`): Event => ({ id, at, kind, goalId: 'g', taskId: null, agentId: null, message });
const NOW = Date.parse('2026-10-14T00:10:00Z');

describe('subject particle', () => {
  it('uses 이 after a final consonant and 가 otherwise', () => {
    expect(subject('민준')).toBe('민준이');
    expect(subject('서연')).toBe('서연이');
    expect(subject('하늘')).toBe('하늘이');
    expect(subject('개발자 A')).toBe('개발자 A가');
    expect(subject('나리')).toBe('나리가');
  });
});

describe('progressLine', () => {
  it('names the stage owner from the squad, else the default role', () => {
    const squad = { goalId: 'g', members: [{ agentId: 'pm', slot: 'lead', duty: 'analysis', reason: '' }] } as unknown as Squad;
    expect(progressLine(goal('analyzing'), [], agents, [], squad, NOW)).toBe('서연이 목표와 프로젝트를 살펴보는 중…');
    expect(progressLine(goal('planning'), [], agents, [], null, NOW)).toBe('서연이 작업을 나누는 중…');
    expect(progressLine(goal('validating'), [], agents, [], null, NOW)).toBe('하늘이 결과를 확인하는 중…');
  });
  it('shows up to two running tasks and counts the rest by person', () => {
    const tasks = [task('1', 'dev-a', 'running', '로그인 화면'), task('2', 'dev-b', 'running', '저장 기능'), task('3', 'dev-c', 'running'), task('4', 'dev-a', 'pending')];
    expect(progressLine(goal('executing'), tasks, agents, [event(9, 'task_started', 'x')], null, NOW)).toBe('개발자 A가 「로그인 화면」, 개발자 B가 「저장 기능」 작업 중… 외 1명');
  });
  it('says the team is waiting when the engine recorded a wait', () => {
    expect(progressLine(goal('planning'), [], agents, [event(1, 'runtime_progress', '다음 단계 담당이 다른 일을 마칠 때까지 기다립니다: PM busy')], null, NOW)).toBe('다음 담당이 다른 일을 마칠 때까지 기다리는 중');
  });
  it('adds the quiet minutes only as a diagnostic', () => {
    const tasks = [task('1', 'dev-a', 'running', '로그인 화면')];
    expect(progressLine(goal('executing'), tasks, agents, [event(1, 'task_started', 'x', '2026-10-13T23:40:00Z')], null, NOW)).toBe('개발자 A가 「로그인 화면」 작업 중… · 마지막 기록 후 30분');
  });
  it('is empty when nothing runs', () => {
    for (const status of ['new', 'awaiting_approval', 'blocked', 'completed', 'cancelled']) expect(progressLine(goal(status), [], agents, [], null, NOW)).toBeNull();
  });
});

describe('stage lines', () => {
  it('turns known engine stage events into plain lines, oldest first, and skips the rest', () => {
    const lines = stageLines(goal('executing'), [
      event(4, 'goal_stage', 'Implementation tasks delegated'),
      event(3, 'approval_decided', 'project_write: approved'),
      event(2, 'goal_stage', 'CEO approval requested before implementation'),
      event(1, 'goal_started', 'Analysis started by squad lead cto'),
      event(5, 'runtime_progress', '담당 파일을 구현하고 검증하고 있습니다'),
    ]).map(line => line.text);
    expect(lines).toEqual(['팀이 목표를 받아 분석을 시작했어요', 'CEO가 계획을 승인했어요', '계획대로 작업을 시작했어요']);
  });
  it('shows a revision as a new planning pass and a rejection as a cancelled goal', () => {
    const lines = stageLines(goal('cancelled'), [
      event(1, 'approval_decided', 'project_write: revise_requested'),
      event(2, 'goal_stage', 'CEO requested a plan revision; planning lead started'),
      event(3, 'approval_decided', 'project_write: denied'),
      event(4, 'goal_cancelled', 'CEO rejected the plan'),
    ]).map(line => line.text);
    expect(lines).toEqual(['개선 방침으로 계획을 다시 세우고 있어요', '계획을 거부해 목표를 취소했어요']);
  });
});

describe('plan cards', () => {
  const approval = (id: string, status: string, createdAt: string): Approval => ({ id, goalId: 'g', operation: 'project_write', reason: `AI 팀이 계획된 작업을 구현합니다 ${id}\n세부`, status, createdAt });
  it('keeps only the pending card current and counts live tasks, people and files', () => {
    const tasks = [task('1', 'dev-a', 'pending', 'a', ['a.ts', 'b.ts']), task('2', 'dev-b', 'pending', 'b', ['b.ts']), task('3', 'dev-a', 'superseded', 'old', ['z.ts'])];
    const cards = planCards(goal('awaiting_approval'), [approval('new', 'pending', '2026-10-14T00:05:00Z'), approval('old', 'other', '2026-10-14T00:02:00Z'), { ...approval('git', 'pending', 't'), operation: 'git_bootstrap' }], tasks, '마감일 기능을 넣어요');
    expect(cards.map(card => [card.approval.id, card.current])).toEqual([['old', false], ['new', true]]);
    expect(cards[1].counts).toBe('작업 2개 · 팀원 2명 · 파일 2개');
    expect(cards[1].summary).toBe('마감일 기능을 넣어요');
    expect(cards.map(planCardLabel)).toEqual(['계획 · 바뀜', '계획이 나왔어요']);
  });
});
