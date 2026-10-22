import type { Approval, Goal, Squad, Task, TokenUsageRecord } from './types';

/**
 * ADR-0054: what approving a plan sets in motion, before the CEO presses the button. Counts come from
 * the saved plan (tasks, reviewers, the SDD note the engine wrote into the approval); the token figure
 * is this project's own average per call, never a cost or a quota. Nothing here calls an AI.
 */
export type PlanEstimate = { calls: number; text: string; tokens: string; rework: string };

const MIN_RECORDS = 3;

export function formatTokens(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${Math.round(value / 100) / 10}k`;
  return String(Math.round(value));
}

export function planEstimate(goal: Goal, approval: Approval, tasks: Task[], squad: Squad | null, usage: TokenUsageRecord[]): PlanEstimate {
  const work = tasks.filter(task => task.goalId === goal.id && task.status !== 'superseded' && task.status !== 'completed').length;
  const reviewers = Math.max(1, squad?.goalId === goal.id ? squad.members.filter(member => member.slot === 'reviewer').length : 0);
  const sdd = /\nSDD: /.test(approval.reason) ? 2 : 0;
  const calls = work + reviewers + sdd;
  const parts = [`작업 ${work}`, `QA ${reviewers}`, ...(sdd ? [`SDD ${sdd}`] : [])].join(' · ');
  const text = `승인하면 AI를 약 ${calls}번 불러요 (${parts})`;
  const counted = usage.filter(item => item.projectId === goal.projectId && !item.meetingId && item.inputTokens + item.outputTokens > 0);
  const average = counted.length ? counted.reduce((sum, item) => sum + item.inputTokens + item.outputTokens, 0) / counted.length : 0;
  const tokens = counted.length >= MIN_RECORDS
    ? `이 프로젝트의 호출 평균(${formatTokens(average)})으로 보면 토큰 약 ${formatTokens(average * calls)}`
    : '아직 이 프로젝트의 호출 기록이 적어 토큰은 어림하지 않아요';
  const limit = goal.reviewLoopLimit ?? 2;
  const rework = limit > 0 ? `QA에 떨어지면 최대 ${limit}번 다시 고쳐요 · 한 번마다 호출이 더 들어요` : 'QA에 떨어지면 다시 고치지 않고 멈춰요';
  return { calls, text, tokens, rework };
}
