import { describe, expect, it } from 'vitest';
import { meetingAttention, meetingMembers } from './meetingStatus';
import { openWork } from './openWork';
import type { Goal, Meeting, MeetingPacket, Snapshot } from './types';

const goal: Goal = { id: 'g', kind: 'deliberation', projectId: 'p', intent: 'Which improvement first?', status: 'deliberating', report: '', createdAt: '2026-10-09T00:00:00Z', resumedFrom: null, reviewLoopsUsed: 0, reviewLoopLimit: null, workGroup: null };
const meeting = (stage: string, extra: Partial<Meeting> = {}): Meeting => ({ id: 'm', goalId: 'g', projectId: 'p', topic: 't', stage, slots: [], sources: [], callsUsed: 3, maxCalls: 10, package: null, decision: null, createdAt: '', updatedAt: '', activeStage: 'critiquing', blockReason: '', ...extra });
const packet = (status: string): MeetingPacket => ({ id: status, meetingId: 'm', stage: 'critiquing', slotId: status, payload: 'x', payloadSha256: 'h', byteLength: 1, sourceSnapshotIds: [], previewRevision: 1, destinationFingerprint: 'f', status, createdAt: '' });

describe('meetingAttention', () => {
  it('asks for packet review, then dispatch, ignoring superseded revisions', () => {
    expect(meetingAttention(goal, meeting('awaiting_transfer_approval'), [packet('pending'), packet('approved'), packet('superseded')])).toMatchObject({ label: '보낼 내용 확인 필요', next: '보낼 내용 1개 확인', needsDecision: true });
    expect(meetingAttention(goal, meeting('awaiting_transfer_approval'), [packet('approved'), packet('outcome_unknown_superseded')])).toMatchObject({ label: '보내기 대기', needsDecision: true });
  });
  it('treats active calls as progress and blocked/stopped/package as CEO decisions', () => {
    expect(meetingAttention(goal, meeting('revising'))).toMatchObject({ needsDecision: false, active: true });
    expect(meetingAttention(goal, meeting('blocked', { blockReason: 'denied' }))).toMatchObject({ detail: 'denied', needsDecision: true });
    expect(meetingAttention(goal, meeting('package_ready')).needsDecision).toBe(true);
    expect(meetingAttention(goal, meeting('decision_recorded')).needsDecision).toBe(false);
    expect(meetingAttention(goal, undefined).needsDecision).toBe(true);
  });
  it('plan review is a CEO decision, planning is progress, and legacy meetings are read-only history', () => {
    const slots = [
      { id: 'pm', kind: 'chair', agentName: '민지 PM', viewpoint: '', instructions: '', connectionId: 'c', connectionFingerprint: 'f', modelId: '' },
      { id: 'cto', kind: 'participant', agentName: '도윤 CTO', viewpoint: '', instructions: '', connectionId: 'c', connectionFingerprint: 'f', modelId: '' },
      { id: 'qa', kind: 'evaluator', agentName: '서연 QA', viewpoint: '', instructions: '', connectionId: 'c', connectionFingerprint: 'f', modelId: '' },
    ] as Meeting['slots'];
    const plan = { revision: 2, digest: 'd', objective: 'o', constraints: [], participants: [{ agentId: 'cto', stance: '' }], evaluatorAgentId: 'qa', protocol: 'propose_critique_revise', rounds: 1, maxCalls: 6, sources: [], successCriteria: [], origin: 'ceo', errors: [], createdAt: '' };
    expect(meetingAttention(goal, meeting('planning', { slots: slots.slice(0, 1) }))).toMatchObject({ needsDecision: false, active: true });
    expect(meetingAttention(goal, meeting('planning', { slots: slots.slice(0, 1) })).detail).toContain('민지 PM');
    const review = meetingAttention(goal, meeting('plan_review', { slots, plan }));
    expect(review).toMatchObject({ label: '회의 계획 확인 필요', needsDecision: true });
    expect(review.detail).toContain('도윤 CTO');
    expect(meetingAttention(goal, meeting('plan_review', { slots, plan: { ...plan, errors: ['x'] } })).next).toBe('계획 고치기');
    expect(meetingAttention(goal, meeting('legacy_closed'))).toMatchObject({ needsDecision: false, active: false });
    const calling = { ...packet('dispatching'), slotId: 'cto', stage: 'proposing' };
    expect(meetingMembers(meeting('proposing', { slots, plan }), [calling])).toEqual({ chair: '민지 PM', participants: ['도윤 CTO'], evaluator: '서연 QA', calling: ['도윤 CTO'] });
    expect(meetingAttention(goal, meeting('proposing', { slots, plan }), [calling]).detail).toContain('도윤 CTO 응답 대기');
  });
  it('open work uses the same rule for meeting goals', () => {
    const snapshot = { goals: [goal], projects: [{ id: 'p', name: 'Pilot', path: '', kind: 'folder', reviewLoopLimit: 2 }], tasks: [], approvals: [], goalGuidance: [], meetings: [meeting('evaluating')], meetingPackets: [] } as unknown as Snapshot;
    expect(openWork(snapshot)[0].needsDecision).toBe(false);
    snapshot.meetings = [meeting('package_ready')];
    expect(openWork(snapshot)[0].needsDecision).toBe(true);
  });
});
