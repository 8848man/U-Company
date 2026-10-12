import { describe, expect, it } from 'vitest';
import { captureAttentionBaseline, detectAttentionTransitions, readAttentionPreferences } from './attentionNotifications';
import type { Approval, Goal } from './types';

const goal = (id: string, status: string): Goal => ({ id, projectId: 'p', intent: 'Test goal', status, report: '', createdAt: '', resumedFrom: null, reviewLoopsUsed: 0, reviewLoopLimit: 2, workGroup: null });
const approval = (id: string, status: string): Approval => ({ id, goalId: 'g1', operation: 'project_write', reason: 'scope', status, createdAt: '' });

describe('attention transitions', () => {
  it('does not replay approvals and blocks present at startup or after repeated refresh', () => {
    const snapshot = { goals: [goal('g1', 'blocked')], approvals: [approval('a1', 'pending')] };
    const first = captureAttentionBaseline(snapshot);
    expect(detectAttentionTransitions(first, snapshot)).toEqual([]);
    expect(detectAttentionTransitions(captureAttentionBaseline(snapshot), snapshot)).toEqual([]);
  });

  it('signals a new approval and newly blocked goal once from saved states', () => {
    const before = { goals: [goal('g1', 'planning'), goal('g2', 'executing')], approvals: [] };
    const after = { goals: [goal('g1', 'awaiting_approval'), goal('g2', 'blocked')], approvals: [approval('a1', 'pending')] };
    expect(detectAttentionTransitions(captureAttentionBaseline(before), after)).toEqual([
      { kind: 'approval', goalId: 'g1', id: 'a1' },
      { kind: 'blocked', goalId: 'g2', id: 'g2' },
    ]);
    expect(detectAttentionTransitions(captureAttentionBaseline(after), after)).toEqual([]);
  });

  it('ignores decided approvals and nonblocked goal changes', () => {
    const before = { goals: [goal('g1', 'planning')], approvals: [] };
    const after = { goals: [goal('g1', 'executing')], approvals: [approval('a1', 'approved')] };
    expect(detectAttentionTransitions(captureAttentionBaseline(before), after)).toEqual([]);
  });

  it('alerts for a failed goal once and does not repeat when it becomes blocked', () => {
    const before = { goals: [goal('g1', 'executing')], approvals: [] };
    const failed = { goals: [goal('g1', 'failed')], approvals: [] };
    const blocked = { goals: [goal('g1', 'blocked')], approvals: [] };
    expect(detectAttentionTransitions(captureAttentionBaseline(before), failed)).toEqual([{ kind: 'blocked', goalId: 'g1', id: 'g1' }]);
    expect(detectAttentionTransitions(captureAttentionBaseline(failed), blocked)).toEqual([]);
  });
});

it('defaults notification preferences off when storage is missing or malformed', () => {
  expect(readAttentionPreferences({ getItem: () => null })).toEqual({ system: false, sound: false });
  expect(readAttentionPreferences({ getItem: () => '{' })).toEqual({ system: false, sound: false });
  expect(readAttentionPreferences({ getItem: () => '{"system":true,"sound":false}' })).toEqual({ system: true, sound: false });
});
