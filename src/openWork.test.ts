import { describe, expect, it } from 'vitest';
import { openWork } from './openWork';
import type { Goal, Snapshot, Task } from './types';

const goal = (id: string, status: string, projectId = 'p'): Goal => ({ id, projectId, intent: id, status, report: '', createdAt: id, resumedFrom: null, reviewLoopsUsed: 0, reviewLoopLimit: 2, workGroup: null });
const task = (id: string, goalId: string, status: string): Task => ({ id, goalId, title: id, instructions: '', files: [], assigneeId: 'dev', status, dependsOn: [], worktree: null, result: '', sessionId: null });
const snapshot: Snapshot = { project: { id: 'p', name: 'Current', path: 'p', kind: 'folder', reviewLoopLimit: 2 }, projects: [{ id: 'p', name: 'Current', path: 'p', kind: 'folder', reviewLoopLimit: 2 }, { id: 'other', name: 'Other', path: 'other', kind: 'folder', reviewLoopLimit: 2 }], agents: [], connections: [], goals: [], goalTurns: [], goalGuidance: [], tasks: [], events: [], approvals: [], usage: [], specProfiles: [], specDrafts: [] };

describe('open work launcher data', () => {
  it('shows cross-project unfinished work and hides completed tasks and superseded goals', () => {
    const result = openWork({ ...snapshot, goals: [goal('a', 'blocked'), goal('b', 'executing', 'other'), goal('c', 'completed'), goal('d', 'blocked')], tasks: [task('a1', 'a', 'failed'), task('a2', 'a', 'completed'), task('b1', 'b', 'running')], goalGuidance: [{ id: 'revision', goalId: 'd', text: '', status: 'revised', createdAt: '', checkpoint: null, replacementGoalId: 'b' }] });
    expect(result.map(item => item.goal.id)).toEqual(['a', 'b']);
    expect(result[0].tasks.map(item => item.id)).toEqual(['a1']);
    expect(result[1].projectName).toBe('Other');
  });
  it('keeps a conversation whose small newest pass completed while its objective is open, and drops closed ones (ADR-0059)', () => {
    const goals = [goal('a', 'blocked'), goal('b', 'completed'), goal('x', 'blocked'), goal('y', 'blocked'), goal('solo', 'completed')];
    const goalGuidance = [{ id: 'g1', goalId: 'a', text: '', status: 'revised' as const, createdAt: '', checkpoint: null, replacementGoalId: 'b' }, { id: 'g2', goalId: 'x', text: '', status: 'revised' as const, createdAt: '', checkpoint: null, replacementGoalId: 'y' }];
    const objective = (conversationId: string, state: string) => ({ conversationId, projectId: 'p', state, achieved: false, achievedBy: [], evidence: [], remaining: [], summary: '', projectChanges: '', checkedGoalId: '', checkedAt: '' });
    expect(openWork({ ...snapshot, goals, goalGuidance }).map(item => item.goal.id).sort()).toEqual(['b', 'y']); // an unchecked longer chain stays; a single completed goal does not
    expect(openWork({ ...snapshot, goals, goalGuidance, conversationObjectives: [objective('a', 'achieved'), objective('x', 'closed_by_other')] }).map(item => item.goal.id)).toEqual([]);
    expect(openWork({ ...snapshot, goals, goalGuidance, conversationObjectives: [objective('x', 'reopened')] }).map(item => item.goal.id).sort()).toEqual(['b', 'y']);
  });
  it('keeps a new goal without tasks and prioritizes pending decisions', () => {
    const result = openWork({ ...snapshot, goals: [goal('z', 'new'), goal('a', 'awaiting_approval')], approvals: [{ id: 'approval', goalId: 'a', operation: 'project_write', reason: '', status: 'pending', createdAt: '' }] });
    expect(result.map(item => item.goal.id)).toEqual(['a', 'z']);
    expect(result[1].tasks).toEqual([]);
  });
  it('keeps a deliberation in open work and prioritizes its packet approval stage', () => {
    const meetingGoal = { ...goal('m', 'deliberating'), kind: 'deliberation' as const };
    const result = openWork({ ...snapshot, goals: [goal('z', 'new'), meetingGoal], meetings: [{ id: 'meeting', goalId: 'm', projectId: 'p', topic: 'Review', stage: 'awaiting_transfer_approval', slots: [], sources: [], callsUsed: 0, maxCalls: 10, package: null, decision: null, createdAt: '', updatedAt: '' }] });
    expect(result.map(item => item.goal.id)).toEqual(['m', 'z']);
    expect(result[0].needsDecision).toBe(true);
  });
});
