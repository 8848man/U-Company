import { describe, expect, it } from 'vitest';
import { controlLine, headline, meetingIssues, stageDigests, withSummaries } from './plainText';
import type { DecisionPackage, Meeting, MeetingContribution, Snapshot } from './types';

const pkg = (patch: Partial<DecisionPackage> = {}): DecisionPackage => ({ id: 'p', sha256: 'x', partial: false, terminationReason: '한 라운드 완료', recommendations: [], claims: [{ id: 'a.C1', slotId: 'a', text: '첫 달은 9,900원으로 시작한다', revised: false, sourceStatus: 'none', assumptions: [], unknowns: [], evaluatorVerdict: 'uncertain', evaluatorNote: '' }, { id: 'b.C1', slotId: 'b', text: '경쟁사 가격 자료를 먼저 확인한다', revised: false, sourceStatus: 'none', assumptions: [], unknowns: [], evaluatorVerdict: 'uncertain', evaluatorNote: '' }], alternatives: [], options: [], dissent: [
  { critiqueId: 'b.K1', criticSlotId: 'b', targetClaimId: 'a.C1', text: '너무 싸면 손해', disposition: 'rebut', response: '근거 있음' },
  { critiqueId: 'c.K1', criticSlotId: 'c', targetClaimId: 'a.C1', text: '첫 달만이면 괜찮다', disposition: 'unanswered', response: '' },
  { critiqueId: 'a.K1', criticSlotId: 'a', targetClaimId: 'b.C1', text: '자료가 오래됨', disposition: 'accept_but_flagged_by_evaluator', response: '받아들임' },
], provenance: [], usage: [], independenceNote: '', createdAt: 't', ...patch });

describe('plain text projections (ADR-0042)', () => {
  it('labels AI summaries, excerpts and short originals', () => {
    expect(headline('긴 원문입니다. 두 번째 문장은 아주 길게 이어집니다 '.repeat(5), '가격을 정하는 회의')).toMatchObject({ text: '가격을 정하는 회의', source: 'ai' });
    expect(headline('짧은 안건', undefined)).toMatchObject({ text: '짧은 안건', source: 'full' });
    expect(headline('첫 문장입니다. ' + '뒤 내용 '.repeat(40), undefined).source).toBe('excerpt');
  });
  it('groups dissent by target claim when the evaluator gave no issues, keeping every item', () => {
    const issues = meetingIssues(pkg());
    expect(issues.map(issue => [issue.text, issue.status, issue.items.length])).toEqual([['“첫 달은 9,900원으로 시작한다”에 반대 의견 2개', '남음', 2], ['“경쟁사 가격 자료를 먼저 확인한다”에 반대 의견 1개', '확인 필요', 1]]);
  });
  it('prefers the evaluator issues and still shows dissent they did not cover', () => {
    const issues = meetingIssues(pkg({ issues: [{ text: '너무 싸게 팔면 손해일까?', critiqueIds: ['b.K1', 'c.K1'] }] }));
    expect(issues[0]).toMatchObject({ text: '너무 싸게 팔면 손해일까?', ai: true, status: '남음' });
    expect(issues[0].items).toHaveLength(2);
    expect(issues[1].items.map(item => item.critiqueId)).toEqual(['a.K1']);
  });
  it('builds one line per stage from AI summaries or saved counts, and a plain control line', () => {
    const meeting = { id: 'm', stage: 'package_ready', activeStage: '', callsUsed: 7, maxCalls: 9, sources: [{ id: 'S1' }], plan: { revision: 1 }, planApproval: { status: 'approved', mode: 'plan' }, slots: [] } as unknown as Meeting;
    const contribution = (stage: string, slotId: string, content: Record<string, unknown>) => ({ id: `${stage}-${slotId}`, meetingId: 'm', stage, slotId, content } as unknown as MeetingContribution);
    const digests = stageDigests(meeting, [contribution('proposing', 'a', { claims: [{}, {}], summary: '두 가지 생각을 냈어요' }), contribution('critiquing', 'b', { critiques: [{}] })], id => id.toUpperCase());
    expect(digests.find(item => item.stage === 'proposing')?.lines).toEqual([{ who: 'A', text: '두 가지 생각을 냈어요', ai: true }]);
    expect(digests.find(item => item.stage === 'critiquing')?.lines).toEqual([{ who: 'B', text: '약한 곳 1개를 짚었어요', ai: false }]);
    expect(digests.every(item => !item.current)).toBe(true);
    expect(controlLine(meeting)).toBe('계획 승인함 · AI에게 물어본 횟수 7/9번 · 보낸 자료 1개');
  });
  it('attaches display summaries to goals without changing their intent', () => {
    const snapshot = { goals: [{ id: 'g', kind: 'deliberation', intent: '원문' }, { id: 'h', kind: 'execution', intent: '원문2' }], meetings: [{ goalId: 'g', topicSummary: '회의 요약' }], goalSummaries: [{ goalId: 'h', text: '목표 요약', createdAt: 't' }] } as unknown as Snapshot;
    expect(withSummaries(snapshot).goals.map(goal => [goal.intent, goal.summary])).toEqual([['원문', '회의 요약'], ['원문2', '목표 요약']]);
  });
});
