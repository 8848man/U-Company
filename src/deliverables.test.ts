import { describe, expect, it } from 'vitest';
import { blockFacts, deliverableRows, quietMinutes } from './deliverables';
import { goalRecovery } from './goalRecovery';
import type { Agent, Approval, DeliverableReview, Event, Goal, Task } from './types';

const goal: Goal = { id: 'g', projectId: 'p', intent: 'Launch docs', status: 'blocked', report: 'Blocked: developer-b: AI 연결 상태가 failed입니다', createdAt: '2026-10-10', resumedFrom: 'executing', reviewLoopsUsed: 0, reviewLoopLimit: 2, workGroup: null, blockKind: 'precall' };
const base: Task = { id: 't1', goalId: 'g', title: 'Hypothesis', instructions: '', files: ['launch/hypothesis.md'], assigneeId: 'developer-a', status: 'completed', dependsOn: [], worktree: null, result: '', sessionId: null, deliverable: '고객·문제 가설', acceptance: ['Names the customer', 'Separates facts'], evidence: ['notes/idea.md'], capability: 'document', assignmentReason: 'document 작업 가능' };
const tasks: Task[] = [base, { ...base, id: 't2', title: 'Message', deliverable: '제안·메시지 초안', assigneeId: 'developer-b', status: 'pending' }, { ...base, id: 't3', title: '재작업 1: Hypothesis', status: 'pending' }];
const approval: Approval = { id: 'a', goalId: 'g', operation: 'project_write', reason: 'scope', status: 'approved', createdAt: '2026-10-10' };
const agents = [{ id: 'developer-a', name: 'Dev A' }, { id: 'developer-b', name: 'Dev B' }] as Agent[];
const events: Event[] = [{ id: 2, at: '2026-10-10T01:00:00Z', kind: 'goal_blocked', goalId: 'g', taskId: null, agentId: null, message: 'developer-b: AI 연결 상태가 failed입니다' }];

describe('deliverable rows', () => {
  it('lists original deliverables with owner, reason and the latest review, excluding repair tasks', () => {
    const reviews: DeliverableReview[] = [
      { id: 'r0', goalId: 'g', taskId: 't1', round: 0, verdict: 'fail', criteria: [], unverified: [], reviewerId: 'qa', createdAt: '1' },
      { id: 'r1', goalId: 'g', taskId: 't1', round: 1, verdict: 'pass', criteria: [{ criterion: 'Names the customer', met: true, evidence: 'hypothesis.md §1' }], unverified: ['market size'], reviewerId: 'qa', createdAt: '2' },
    ];
    const rows = deliverableRows(goal, tasks, reviews, agents);
    expect(rows.map(row => row.title)).toEqual(['고객·문제 가설', '제안·메시지 초안']);
    expect(rows[0]).toMatchObject({ owner: 'Dev A', reason: 'document 작업 가능' });
    expect(rows[0].review?.round).toBe(1);
    expect(rows[1].review).toBeNull();
    expect(deliverableRows(goal, [{ ...base, acceptance: [] }], [], agents)).toEqual([]);
  });
});

describe('blocked goal facts and recovery', () => {
  it('reads reason → last fact → owner → next action from saved state', () => {
    const facts = blockFacts(goal, tasks.slice(0, 2), [approval], [], agents, events);
    expect(facts).toMatchObject({ reason: 'developer-b: AI 연결 상태가 failed입니다', kindLabel: '호출 전 확인 실패 · AI 호출 없음', owner: 'CEO', next: '승인된 계획 이어서 실행' });
    expect(facts?.lastFact).toContain('failed');
  });
  it('resumes only never-started work under an approved plan, and explains every other block kind', () => {
    expect(goalRecovery(goal, tasks.slice(0, 2), [approval], [])?.kind).toBe('resume');
    expect(goalRecovery(goal, tasks.slice(0, 2), [{ ...approval, status: 'denied' }], [])?.kind).toBe('revision');
    for (const status of ['running', 'blocked', 'failed', 'awaiting_integration']) expect(goalRecovery(goal, [base, { ...tasks[1], status }], [approval], [])?.kind).toBe('revision');
    const interrupted = { ...goal, blockKind: 'interrupted' as const };
    const stuck = [base, { ...tasks[1], status: 'blocked' }];
    expect(goalRecovery(interrupted, stuck, [approval], [])?.explanation).toContain('자동으로 다시 실행하거나 통합하지 않습니다');
    expect(blockFacts(interrupted, stuck, [approval], [], agents, events)?.owner).toBe('Dev B → CEO 결정');
    expect(goalRecovery({ ...goal, blockKind: 'approval_denied' }, [], [{ ...approval, status: 'denied' }], [])?.explanation).toContain('자동으로 다시 요청하지 않습니다');
    expect(goalRecovery({ ...goal, blockKind: 'qa_form' }, [base], [approval], [])?.explanation).toContain('재작업 횟수는 쓰지 않았습니다');
  });
  it('treats a quiet runtime as a diagnostic only', () => {
    const running = [{ ...base, status: 'running' }];
    const at = new Date('2026-10-10T01:00:00Z').getTime();
    expect(quietMinutes({ ...goal, status: 'executing' }, running, events, at + 5 * 60000)).toBeNull();
    expect(quietMinutes({ ...goal, status: 'executing' }, running, events, at + 25 * 60000)).toBe(25);
    expect(quietMinutes({ ...goal, status: 'executing' }, [base], events, at + 25 * 60000)).toBeNull();
  });
});
