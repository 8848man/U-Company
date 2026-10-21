import { describe, expect, it } from 'vitest';
import { clampPlace, memberRows, stageStrip } from './officeWindowState';
import type { MemberActivity } from './progress';
import type { Agent, Goal, Squad, Task } from './types';

const goal = (status: string, resumedFrom: string | null = null): Goal => ({ id: 'g', projectId: 'p', intent: 'x', status, report: '', createdAt: 't', resumedFrom, reviewLoopsUsed: 0, reviewLoopLimit: 2, workGroup: null });
const agent = (id: string, name: string): Agent => ({ id, name, role: id, instructions: '', reportsTo: null, connectionId: 'c', modelId: '', x: 0, y: 0, enabled: true });
const task = (id: string, assigneeId: string, status: string): Task => ({ id, goalId: 'g', title: `업무 ${id}`, assigneeId, status, files: [], dependsOn: [], result: '' } as unknown as Task);

describe('stage strip', () => {
  it('marks done, current and pending, and tells waiting for the CEO apart', () => {
    expect(stageStrip(goal('executing')).map(stage => stage.state)).toEqual(['done', 'done', 'done', 'current', 'pending', 'pending', 'pending']);
    expect(stageStrip(goal('awaiting_approval')).find(stage => stage.label === '승인')?.state).toBe('waiting');
    expect(stageStrip(goal('completed')).every(stage => stage.state === 'done')).toBe(true);
  });
  it('shows a stop where it happened and a cancelled goal at the approval', () => {
    expect(stageStrip(goal('blocked', 'planning')).find(stage => stage.label === '계획')?.state).toBe('blocked');
    expect(stageStrip(goal('cancelled')).map(stage => stage.state)).toEqual(['done', 'done', 'blocked', 'pending', 'pending', 'pending', 'pending']);
  });
});

describe('member rows', () => {
  const agents = [agent('cto', '민준'), agent('pm', '서연'), agent('dev-a', '개발자 A'), agent('dev-b', '개발자 B'), agent('qa', '하늘'), agent('other', '다른 팀원')];
  const idle = (): MemberActivity => ({ kind: 'idle', label: '대기 중', goal: null, task: null, events: [] });
  it('lists squad members first with their work and state, and never people outside the goal', () => {
    const squad = { goalId: 'g', members: [{ agentId: 'cto', slot: 'lead', duty: 'analysis' }, { agentId: 'dev-a', slot: 'writer', duty: '' }, { agentId: 'qa', slot: 'reviewer', duty: '' }] } as unknown as Squad;
    const tasks = [task('1', 'dev-a', 'running'), task('2', 'dev-b', 'completed')];
    const activities = Object.fromEntries(agents.map(item => [item.id, idle()]));
    const rows = memberRows(goal('executing'), agents, tasks, squad, activities);
    expect(rows.map(row => [row.name, row.work, row.state])).toEqual([['민준', '목표 분석', '대기'], ['개발자 A', '업무 1', '작업 중'], ['하늘', '결과 검토', '대기'], ['개발자 B', '업무 2', '끝남']]);
  });
  it('shows a lead as working only for this goal', () => {
    const activities: Record<string, MemberActivity> = { pm: { kind: 'active', label: '업무 계획', goal: goal('planning'), task: null, events: [] } };
    const squad = { goalId: 'g', members: [{ agentId: 'pm', slot: 'lead', duty: 'planning' }] } as unknown as Squad;
    expect(memberRows(goal('planning'), agents, [], squad, activities)[0].state).toBe('작업 중');
    const elsewhere: Record<string, MemberActivity> = { pm: { ...activities.pm, goal: { ...goal('planning'), id: 'other-goal' } } };
    expect(memberRows(goal('planning'), agents, [], squad, elsewhere)[0].state).toBe('대기');
  });
});

describe('window place', () => {
  it('keeps the title bar inside the app and the window no larger than it', () => {
    expect(clampPlace({ x: -1, y: 96, w: 440, h: 560, mode: 'floating' }, 1280, 800)).toMatchObject({ x: 816, w: 440, h: 560 });
    expect(clampPlace({ x: 5000, y: 5000, w: 3000, h: 3000, mode: 'floating' }, 1280, 800)).toMatchObject({ x: 1160, y: 756, w: 1264, h: 784 });
    expect(clampPlace({ x: 10, y: 10, w: 100, h: 100, mode: 'floating' }, 1280, 800)).toMatchObject({ w: 320, h: 240 });
  });
});
