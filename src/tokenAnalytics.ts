import type { Agent, Goal, TokenUsageRecord } from './types';

export type TokenTotals = { input: number; output: number; total: number; estimated: number; active: number };
export type ModelTotals = TokenTotals & { key: string; connection: string; model: string };
export type MemberTotals = TokenTotals & { id: string; name: string; models: ModelTotals[] };
export type GoalTotals = TokenTotals & { goal: Goal; members: MemberTotals[] };
export type UsageSort = 'highest' | 'lowest' | 'name';

export function sumUsage(rows: TokenUsageRecord[]): TokenTotals {
  return rows.reduce((sum, row) => {
    const input = Math.max(0, row.inputTokens || 0);
    const output = Math.max(0, row.outputTokens || 0);
    const total = input + output;
    sum.input += input;
    sum.output += output;
    sum.total += total;
    if (row.source === 'estimated') sum.estimated += total;
    if (row.status === 'active') sum.active += total;
    return sum;
  }, { input: 0, output: 0, total: 0, estimated: 0, active: 0 });
}

export function shareOfProject(tokens: number, projectTotal: number): number {
  return projectTotal > 0 ? Math.min(100, Math.max(0, tokens / projectTotal * 100)) : 0;
}

export function sortByUsage<T extends { total: number }>(items: T[], order: UsageSort, name: (item: T) => string): T[] {
  return [...items].sort((a, b) => order === 'name' ? name(a).localeCompare(name(b), 'ko') :
    (order === 'highest' ? b.total - a.total : a.total - b.total) || name(a).localeCompare(name(b), 'ko'));
}

export function memberTotals(rows: TokenUsageRecord[], agents: Agent[]): MemberTotals[] {
  const groups = new Map<string, TokenUsageRecord[]>();
  for (const row of rows) groups.set(row.agentId, [...(groups.get(row.agentId) ?? []), row]);
  return [...groups.entries()].map(([id, owned]) => {
    const modelGroups = new Map<string, TokenUsageRecord[]>();
    for (const row of owned) {
      const key = `${row.connectionId || ''}\u0000${row.modelId || ''}`;
      modelGroups.set(key, [...(modelGroups.get(key) ?? []), row]);
    }
    const models = [...modelGroups.entries()].map(([key, modelRows]) => ({
      key, connection: modelRows[0].connectionName || '연결 정보 미확인',
      model: modelRows[0].modelId || '모델 ID 미확인', ...sumUsage(modelRows),
    })).sort((a, b) => b.total - a.total);
    return { id, name: agents.find(agent => agent.id === id)?.name ?? id, ...sumUsage(owned), models };
  });
}

export function goalTotals(rows: TokenUsageRecord[], goals: Goal[], agents: Agent[]): GoalTotals[] {
  return goals.map(goal => {
    const owned = rows.filter(row => row.goalId === goal.id);
    return { goal, ...sumUsage(owned), members: memberTotals(owned, agents) };
  });
}

export function usageInsights(rows: TokenUsageRecord[], goals: Goal[]): string[] {
  const total = sumUsage(rows);
  if (!total.total) return ['아직 사용 기록이 없습니다. 목표가 실행되면 입력·출력 토큰과 모델별 기록을 바탕으로 절감 힌트를 보여줍니다.'];
  const result: string[] = [];
  const estimatedShare = shareOfProject(total.estimated, total.total);
  if (estimatedShare >= 20) result.push(`전체의 ${estimatedShare.toFixed(1)}%가 추정치입니다. 모델별 수치를 비교할 때 제공자 보고 값과 구분해 보세요.`);
  if (total.input >= total.output * 2) result.push('입력 토큰 비중이 큽니다. 반복되는 프로젝트 설명과 긴 첨부 문맥을 줄일 수 있는지 먼저 살펴보세요.');
  else if (total.output > total.input) result.push('출력 토큰 비중이 큽니다. 필요한 산출물 형식과 분량을 목표에 명확히 적어 보세요.');
  const ranked = goalTotals(rows, goals, []).sort((a, b) => b.total - a.total);
  if (ranked[0] && ranked[0].total > total.total * 0.4) result.push(`가장 많은 토큰을 쓴 목표가 전체의 ${shareOfProject(ranked[0].total, total.total).toFixed(1)}%를 차지합니다. 이 목표의 팀원·모델 상세에서 재작업이나 긴 입력이 있었는지 확인하세요.`);
  return result.length ? result : ['사용량이 여러 목표에 분산되어 있습니다. 목표별 상세에서 큰 입력·출력 구간을 확인해 보세요.'];
}
