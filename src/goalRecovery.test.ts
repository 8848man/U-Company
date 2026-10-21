import { describe, expect, it } from 'vitest';
import { fixReceiver, goalRecovery, teamFixRoute } from './goalRecovery';
import type { Approval, Goal, GoalGuidance, Task } from './types';

const goal: Goal = { id: 'g', projectId: 'p', intent: 'Build the page', status: 'blocked', report: 'Provider error', createdAt: '2026-10-08', resumedFrom: 'analyzing', reviewLoopsUsed: 0, reviewLoopLimit: 2, workGroup: null };
const task: Task = { id: 't', goalId: 'g', title: 'Page', instructions: '', files: ['page.html'], assigneeId: 'dev', status: 'pending', dependsOn: [], worktree: null, result: '', sessionId: null };
const approval: Approval = { id: 'a', goalId: 'g', operation: 'project_write', reason: 'page.html', status: 'approved', createdAt: '2026-10-08' };
const guidance: GoalGuidance = { id: 'c', goalId: 'g', text: 'Change the outcome', status: 'review_required', createdAt: '2026-10-08', checkpoint: 'after error', replacementGoalId: null };

describe('blocked goal recovery routes', () => {
  it('retries only before tasks and approval exist', () => {
    expect(goalRecovery(goal, [], [], [])?.kind).toBe('retry');
    expect(goalRecovery(goal, [task], [], [])?.kind).toBe('revision');
  });
  it('prioritizes the exact pending approval and resumes only an untouched approved plan', () => {
    const atApproval = { ...goal, resumedFrom: 'awaiting_approval' };
    expect(goalRecovery(atApproval, [task], [{ ...approval, status: 'pending' }], [])?.kind).toBe('approval');
    expect(goalRecovery(atApproval, [task], [approval], [])?.kind).toBe('resume');
    expect(goalRecovery(atApproval, [{ ...task, status: 'awaiting_integration' }], [approval], [])?.kind).toBe('revision');
  });
  it('routes execution failure to CEO revision, and an already revised goal to its replacement', () => {
    const executing = { ...goal, resumedFrom: 'executing', report: 'Task produced no file changes' };
    expect(goalRecovery(executing, [{ ...task, status: 'failed' }], [approval], [])?.kind).toBe('revision');
    expect(goalRecovery(executing, [], [], [guidance])?.kind).toBe('guidance');
    expect(goalRecovery(executing, [], [], [{ ...guidance, status: 'revised', replacementGoalId: 'g2' }])?.kind).toBe('replacement');
  });
});

describe('one-click fix request (ADR-0048)', () => {
  const reviewed: Goal = { ...goal, resumedFrom: 'validating', blockKind: 'other', report: "Blocked: 산출물 'README'의 수용 기준은 8개 이하여야 합니다" };
  const done = { ...task, status: 'completed' };
  it('repairs a review-stage block on the same goal and labels the card as a fix request', () => {
    expect(goalRecovery(reviewed, [done], [approval], [])?.label).toBe('팀원에게 해결 요청');
    expect(teamFixRoute(reviewed, [done], [approval], []).route).toBe('repair');
    expect(teamFixRoute({ ...reviewed, blockKind: 'plan_form' }, [done], [approval], []).note).toContain('새 승인은 필요 없어요');
    expect(teamFixRoute({ ...reviewed, reviewLoopsUsed: 2 }, [done], [approval], []).note).toContain('1회 추가');
  });
  it('falls back to a new version, and leaves connections and CEO denials to the CEO', () => {
    expect(teamFixRoute(reviewed, [{ ...done, status: 'running' }], [approval], []).route).toBe('revision');
    expect(teamFixRoute({ ...reviewed, blockKind: 'integration_conflict', resumedFrom: 'executing' }, [done], [approval], []).route).toBe('revision');
    expect(teamFixRoute({ ...reviewed, blockKind: 'provider' }, [done], [approval], []).route).toBe('connections');
    expect(goalRecovery({ ...reviewed, blockKind: 'approval_denied' }, [done], [approval], [])?.label).toBe('수정 지침으로 새 목표 만들기');
    expect(teamFixRoute({ ...reviewed, blockKind: 'approval_denied' }, [done], [approval], []).route).toBe('direct');
  });
  it('sends the request to the goal planning lead', () => {
    const agents = [{ id: 'pm', name: '서연', enabled: true }, { id: 'pm2', name: '도윤', enabled: true }];
    expect(fixReceiver('g', [{ goalId: 'g', members: [{ slot: 'lead', duty: 'planning', agentId: 'pm2' }] }], agents)).toBe('도윤');
    expect(fixReceiver('g', [], agents)).toBe('서연');
  });
});
