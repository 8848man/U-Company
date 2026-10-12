import { describe, expect, it } from 'vitest';
import { groupGoals, memberActivity, phaseState, shortGoal } from './progress';
import type { Agent, Event, Goal, Task } from './types';

const agent = (id: string, role: string): Agent => ({ id, name: id, role, instructions: '', reportsTo: null, connectionId: 'codex', modelId: '', x: 50, y: 50, enabled: true });
const goal = (status: string, resumedFrom: string | null = null): Goal => ({ id: 'g', projectId: 'p', intent: 'Build a page', status, report: '', createdAt: '2026-10-01T00:00:00Z', resumedFrom, reviewLoopsUsed: 0, reviewLoopLimit: 2, workGroup: null });

describe('shared progress display policy', () => {
  it('shortens CEO goals deterministically without changing saved text', () => {
    const original = '첫 문장으로 요청합니다. ' + '상세 조건 '.repeat(30);
    expect(shortGoal(original)).toBe('첫 문장으로 요청합니다.');
    expect(original).toContain('상세 조건');
    expect(shortGoal('짧은 목표')).toBe('짧은 목표');
  });

  it('groups only explicitly labeled goals in the same project', () => {
    const first = { ...goal('executing'), id: 'g1', workGroup: '출시 준비' };
    const second = { ...goal('validating'), id: 'g2', workGroup: '출시 준비' };
    const otherProject = { ...goal('new'), id: 'g3', projectId: 'other', workGroup: '출시 준비' };
    const separate = { ...goal('new'), id: 'g4' };
    expect(groupGoals([first, second, otherProject, separate]).map(group => group.goals.map(item => item.id))).toEqual([['g1', 'g2'], ['g3'], ['g4']]);
  });
  it('marks only the engine-selected manager as active', () => {
    const agents = [agent('cto', 'architecture'), agent('cto-2', 'architecture')];
    expect(memberActivity(agents[0], agents, [goal('analyzing')], [], []).kind).toBe('active');
    expect(memberActivity(agents[1], agents, [goal('analyzing')], [], []).kind).toBe('idle');
  });

  it('links a running developer to its real task and agent events', () => {
    const developer = agent('developer-a', 'implementation');
    const task: Task = { id: 't', goalId: 'g', title: 'Create team.html', instructions: '', files: ['team.html'], assigneeId: developer.id, status: 'running', dependsOn: [], worktree: null, result: '', sessionId: null };
    const event: Event = { id: 1, at: '2026-10-01T00:00:01Z', kind: 'runtime_progress', goalId: 'g', taskId: 't', agentId: developer.id, message: '파일 변경 중' };
    const activity = memberActivity(developer, [developer], [goal('executing')], [task], [event]);
    expect(activity.kind).toBe('active');
    expect(activity.task?.files).toEqual(['team.html']);
    expect(activity.events.map(item => item.message)).toEqual(['파일 변경 중']);
  });

  it('keeps the newest operational event first for a member', () => {
    const developer = agent('developer-a', 'implementation');
    const older: Event = { id: 1, at: '2026-10-01T00:00:01Z', kind: 'runtime_progress', goalId: 'g', taskId: null, agentId: developer.id, message: 'older' };
    const newer: Event = { ...older, id: 2, at: '2026-10-01T00:00:02Z', message: 'newer' };
    expect(memberActivity(developer, [developer], [], [], [newer, older]).events.map(event => event.message)).toEqual(['newer', 'older']);
  });

  it('does not count blocked or awaiting approval stages as done', () => {
    expect(phaseState(goal('awaiting_approval'), 'awaiting_approval')).toBe('current');
    expect(phaseState(goal('awaiting_approval'), 'executing')).toBe('pending');
    expect(phaseState(goal('blocked', 'planning'), 'planning')).toBe('blocked');
    expect(phaseState(goal('blocked', 'planning'), 'analyzing')).toBe('done');
  });
});
