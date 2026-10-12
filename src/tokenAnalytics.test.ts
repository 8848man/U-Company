import { describe, expect, it } from 'vitest';
import { goalTotals, memberTotals, shareOfProject, sortByUsage, sumUsage, usageInsights } from './tokenAnalytics';
import type { Agent, Goal, TokenUsageRecord } from './types';

const goal = (id: string): Goal => ({ id, projectId: 'p', intent: id, status: 'completed', report: '', createdAt: '', resumedFrom: null, reviewLoopsUsed: 0, reviewLoopLimit: 2, workGroup: null });
const agent = (id: string): Agent => ({ id, name: id, role: 'developer', instructions: '', reportsTo: null, connectionId: 'c', modelId: '', x: 0, y: 0, enabled: true });
const row = (id: string, goalId: string, agentId: string, inputTokens: number, outputTokens: number, modelId: string, source: 'reported' | 'estimated' = 'reported'): TokenUsageRecord =>
  ({ id, projectId: 'p', goalId, agentId, taskId: null, connectionId: 'c', connectionName: 'CLI', modelId, inputTokens, outputTokens, source, status: 'completed', updatedAt: '' });

describe('token analytics', () => {
  const rows = [row('1', 'g1', 'a', 60, 20, 'model-a'), row('2', 'g1', 'a', 10, 10, 'model-b', 'estimated'), row('3', 'g2', 'b', 30, 20, '')];
  it('uses one project denominator for goals and teammates', () => {
    const total = sumUsage(rows);
    expect(total).toMatchObject({ input: 100, output: 50, total: 150, estimated: 20 });
    const goals = goalTotals(rows, [goal('g1'), goal('g2')], [agent('a'), agent('b')]);
    expect(goals.map(item => item.total)).toEqual([100, 50]);
    expect(shareOfProject(goals[0].members[0].total, total.total)).toBeCloseTo(66.6667, 3);
    expect(shareOfProject(0, 0)).toBe(0);
    expect(sortByUsage(goals, 'lowest', item => item.goal.intent).map(item => item.goal.id)).toEqual(['g2', 'g1']);
  });
  it('retains per-invocation model identity and does not fill historical unknowns', () => {
    const members = memberTotals(rows, [agent('a'), agent('b')]);
    expect(members.find(item => item.id === 'a')?.models.map(model => model.model)).toEqual(['model-a', 'model-b']);
    expect(members.find(item => item.id === 'b')?.models[0].model).toBe('모델 ID 미확인');
  });
  it('bases insights on recorded usage', () => {
    expect(usageInsights([], [])).toHaveLength(1);
    expect(usageInsights(rows, [goal('g1'), goal('g2')]).some(item => item.includes('추정치'))).toBe(false);
    expect(usageInsights(rows, [goal('g1'), goal('g2')]).some(item => item.includes('입력 토큰'))).toBe(true);
  });
});
