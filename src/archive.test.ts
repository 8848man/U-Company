import { describe, expect, it } from 'vitest';
import { archiveBlock, formatBytes } from './archive';
import type { Goal, Meeting, Task } from './types';

const goal = (patch: Partial<Goal> = {}): Goal => ({ id: 'g1', kind: 'execution', projectId: null, intent: '목표', status: 'completed', report: '', createdAt: '', resumedFrom: null, reviewLoopsUsed: 0, reviewLoopLimit: null, workGroup: null, ...patch });
const task = (status: string, goalId = 'g1') => ({ id: `t-${status}`, goalId, status }) as Task;

describe('archiveBlock', () => {
  it('allows finished, new or stopped execution goals', () => {
    for (const status of ['completed', 'new', 'blocked']) expect(archiveBlock(goal({ status }), [])).toBeNull();
  });
  it('blocks goals that are running or waiting for approval', () => {
    expect(archiveBlock(goal({ status: 'running' }), [])).toMatch('끝나거나 멈춘 뒤');
    expect(archiveBlock(goal({ status: 'awaiting_approval' }), [])).not.toBeNull();
  });
  it('blocks while a task of this goal is still running', () => {
    expect(archiveBlock(goal({ status: 'blocked' }), [task('running')])).toMatch('실행 중인 업무');
    expect(archiveBlock(goal({ status: 'blocked' }), [task('awaiting_integration')])).not.toBeNull();
    expect(archiveBlock(goal({ status: 'blocked' }), [task('running', 'other')])).toBeNull();
  });
  it('allows meetings only after they stop or record a decision', () => {
    const meeting = (stage: string) => ({ stage }) as Meeting;
    expect(archiveBlock(goal({ kind: 'deliberation', status: 'running' }), [], meeting('proposing'))).toMatch('진행 중인 회의');
    for (const stage of ['decision_recorded', 'legacy_closed', 'blocked', 'stopped']) expect(archiveBlock(goal({ kind: 'deliberation', status: 'running' }), [], meeting(stage))).toBeNull();
  });
});

describe('formatBytes', () => {
  it('uses readable units', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 MB');
    expect(formatBytes(3 * 1024 ** 3)).toBe('3.00 GB');
  });
});
