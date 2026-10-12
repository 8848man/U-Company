import type { Snapshot } from './types';

export type AttentionKind = 'approval' | 'blocked';
export type AttentionSignal = { kind: AttentionKind; goalId: string; id: string };
export type AttentionBaseline = { pendingApprovalIds: Set<string>; goalStatuses: Map<string, string> };
export type AttentionPreferences = { system: boolean; sound: boolean };
export const defaultAttentionPreferences: AttentionPreferences = { system: false, sound: false };
export const attentionPreferencesKey = 'utopia.attention.v1';

export function readAttentionPreferences(storage: Pick<Storage, 'getItem'>): AttentionPreferences {
  try {
    const raw = storage.getItem(attentionPreferencesKey);
    if (!raw) return defaultAttentionPreferences;
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return defaultAttentionPreferences;
    const value = parsed as Record<string, unknown>;
    return { system: value.system === true, sound: value.sound === true };
  } catch {
    return defaultAttentionPreferences;
  }
}

export function captureAttentionBaseline(snapshot: Pick<Snapshot, 'approvals' | 'goals'>): AttentionBaseline {
  return {
    pendingApprovalIds: new Set(snapshot.approvals.filter(item => item.status === 'pending').map(item => item.id)),
    goalStatuses: new Map(snapshot.goals.map(goal => [goal.id, goal.status])),
  };
}

export function detectAttentionTransitions(previous: AttentionBaseline, snapshot: Pick<Snapshot, 'approvals' | 'goals'>): AttentionSignal[] {
  const stopped = (status: string | undefined) => status === 'blocked' || status === 'failed';
  const approvals: AttentionSignal[] = snapshot.approvals
    .filter(item => item.status === 'pending' && item.goalId && !previous.pendingApprovalIds.has(item.id))
    .map(item => ({ kind: 'approval', goalId: item.goalId!, id: item.id }));
  const blocked: AttentionSignal[] = snapshot.goals
    .filter(goal => stopped(goal.status) && !stopped(previous.goalStatuses.get(goal.id)))
    .map(goal => ({ kind: 'blocked', goalId: goal.id, id: goal.id }));
  return [...approvals, ...blocked];
}
