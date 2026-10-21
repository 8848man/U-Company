import { describe, expect, it } from 'vitest';
import { formatTokens, planEstimate } from './planEstimate';
import type { Approval, Goal, Squad, Task, TokenUsageRecord } from './types';

const goal = { id: 'g', projectId: 'p', reviewLoopLimit: 2 } as Goal;
const approval = (reason: string) => ({ id: 'a', goalId: 'g', operation: 'project_write', reason, status: 'pending', createdAt: '' }) as Approval;
const task = (id: string, status = 'pending') => ({ id, goalId: 'g', status }) as Task;
const squad = (reviewers: number) => ({ goalId: 'g', members: [{ agentId: 'cto', slot: 'lead' }, ...Array.from({ length: reviewers }, (_, n) => ({ agentId: `qa${n}`, slot: 'reviewer' }))] }) as unknown as Squad;
const record = (tokens: number, extra: Partial<TokenUsageRecord> = {}) => ({ projectId: 'p', inputTokens: tokens, outputTokens: 0, ...extra }) as TokenUsageRecord;

describe('plan estimate', () => {
  it('counts calls from the saved plan: open tasks, reviewers and the SDD steps', () => {
    const plain = planEstimate(goal, approval('AI 팀이 …'), [task('1'), task('2'), task('3', 'superseded')], squad(1), []);
    expect(plain.calls).toBe(3);
    expect(plain.text).toBe('승인하면 AI를 약 3번 불러요 (작업 2 · QA 1)');
    const withSdd = planEstimate(goal, approval('AI 팀이 …\nSDD: 승인하면 PM이 …'), [task('1')], squad(2), []);
    expect(withSdd.text).toBe('승인하면 AI를 약 5번 불러요 (작업 1 · QA 2 · SDD 2)');
  });

  it('uses this project\'s own average per call only when there are enough records', () => {
    expect(planEstimate(goal, approval(''), [task('1')], null, [record(1000)]).tokens).toContain('어림하지 않아요');
    const many = [record(1000), record(2000), record(3000), record(99999, { projectId: 'other' }), record(50000, { meetingId: 'm' })];
    expect(planEstimate(goal, approval(''), [task('1')], null, many).tokens).toBe('이 프로젝트의 호출 평균(2k)으로 보면 토큰 약 4k');
  });

  it('says what a QA failure costs in plain words', () => {
    expect(planEstimate(goal, approval(''), [], null, []).rework).toBe('QA에 떨어지면 최대 2번 다시 고쳐요 · 한 번마다 호출이 더 들어요');
    expect(planEstimate({ ...goal, reviewLoopLimit: 0 }, approval(''), [], null, []).rework).toBe('QA에 떨어지면 다시 고치지 않고 멈춰요');
  });

  it('formats token counts compactly', () => {
    expect([formatTokens(950), formatTokens(12_345), formatTokens(2_500_000)]).toEqual(['950', '12.3k', '2.5M']);
  });
});
