import { describe, expect, it } from 'vitest';
import { goalBrief } from './goalBrief';
import type { Agent, Approval, Event, Goal, GoalGuidance, Task } from './types';

const goal: Goal = { id: 'g', projectId: 'p', intent: 'Make an accessible page', status: 'executing', report: '', createdAt: '2026-10-03', resumedFrom: null, reviewLoopsUsed: 0, reviewLoopLimit: 2, workGroup: null };
const task: Task = { id: 't', goalId: 'g', title: 'Page', instructions: '', files: ['index.html'], assigneeId: 'dev', status: 'running', dependsOn: [], worktree: null, result: '', sessionId: null };
const agent: Agent = { id: 'dev', name: 'Developer', role: 'implementation', instructions: '', reportsTo: null, connectionId: '', modelId: '', x: 0, y: 0, enabled: true };
const approval: Approval = { id: 'a', goalId: 'g', operation: 'project_write', reason: 'index.html', status: 'pending', createdAt: '2026-10-03' };
const guidance: GoalGuidance = { id: 'change', goalId: 'g', text: 'Change scope', status: 'pending', createdAt: '2026-10-03', checkpoint: null, replacementGoalId: null };
const events: Event[] = [
  { id: 3, at: '2026-10-03', kind: 'goal_question', goalId: 'g', taskId: null, agentId: null, message: 'Question received' },
  { id: 2, at: '2026-10-03', kind: 'task_started', goalId: 'g', taskId: 't', agentId: 'dev', message: 'Started work' },
];

describe('goal workspace brief', () => {
  it('prioritizes CEO direction over old approval and ignores conversation events as progress', () => {
    const brief = goalBrief(goal, [task], [approval], [guidance], [agent], events, []);
    expect(brief.nextAction).toContain('방향 변경');
    expect(brief.owners).toEqual(['Developer']);
    expect(brief.latestEvent?.message).toBe('Started work');
    expect(brief.pendingApproval?.reason).toBe('index.html');
  });
  it('uses goal-scoped usage with provenance', () => {
    const brief = goalBrief(goal, [], [], [], [], [], [
      { id: 'one', projectId: 'p', goalId: 'g', agentId: 'cto', taskId: null, connectionId: 'c', connectionName: 'AI', modelId: 'm', inputTokens: 10, outputTokens: 3, source: 'reported', status: 'completed', updatedAt: '2026-10-03' },
      { id: 'two', projectId: 'p', goalId: 'other', agentId: 'cto', taskId: null, connectionId: 'c', connectionName: 'AI', modelId: 'm', inputTokens: 90, outputTokens: 10, source: 'estimated', status: 'completed', updatedAt: '2026-10-03' },
    ]);
    expect(brief.tokenTotal).toBe(13);
    expect(brief.estimated).toBe(false);
  });
});
