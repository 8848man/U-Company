import { describe, expect, it } from 'vitest';
import { goalRecovery } from './goalRecovery';
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
