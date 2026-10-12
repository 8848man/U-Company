/**
 * Plain-language projections for the three depths of the UI/UX philosophy (sdd/ux-principles.md,
 * ADR-0042). Everything here is derived from saved records or display-only AI summaries; nothing is
 * used for state, approval or completion.
 */
import type { DecisionPackage, Meeting, MeetingContribution, Snapshot } from './types';
import { shortGoal } from './progress';

/** Snapshot with each goal's display summary attached (meeting topic summary or CTO summary line). */
export function withSummaries(snapshot: Snapshot): Snapshot {
  const meetings = snapshot.meetings ?? [];
  const summaries = snapshot.goalSummaries ?? [];
  return {
    ...snapshot,
    goals: snapshot.goals.map(goal => {
      const summary = goal.kind === 'deliberation'
        ? meetings.find(meeting => meeting.goalId === goal.id)?.topicSummary
        : summaries.find(item => item.goalId === goal.id)?.text;
      return summary?.trim() ? { ...goal, summary: summary.trim() } : goal;
    }),
  };
}

export type Headline = { text: string; source: 'ai' | 'excerpt' | 'full'; raw: string };

/** Summary first (UXP-2): AI summary if any, else a labelled excerpt, else the full short text. */
export function headline(raw: string, aiSummary: string | undefined, limit = 60): Headline {
  if (aiSummary?.trim()) return { text: aiSummary.trim(), source: 'ai', raw };
  const text = shortGoal(raw, limit);
  return { text, source: text === raw.trim().replace(/\s+/g, ' ') ? 'full' : 'excerpt', raw };
}

/** Plain stage names for depths 1–2 (UXP-6). */
export const plainStageLabels: Record<string, string> = {
  planning: '계획 세우기', proposing: '각자 생각 내기', critiquing: '약한 곳 짚기', revising: '고치기', evaluating: '심사하기',
};
export const DEBATE_STAGES = ['planning', 'proposing', 'critiquing', 'revising', 'evaluating'];

const arr = (value: unknown): Record<string, unknown>[] => Array.isArray(value) ? value as Record<string, unknown>[] : [];
const str = (value: unknown): string => typeof value === 'string' ? value : '';

export type StageDigest = { stage: string; label: string; lines: { who: string; text: string; ai: boolean }[]; count: number; done: boolean; current: boolean };

/** One line per stage (depth 2): AI summaries from the replies, else a sentence built from saved counts. */
export function stageDigests(meeting: Meeting, contributions: MeetingContribution[], nameOf: (id: string) => string): StageDigest[] {
  const flowAt = DEBATE_STAGES.indexOf(['awaiting_transfer_approval', 'plan_approved'].includes(meeting.stage) ? meeting.activeStage ?? '' : meeting.stage);
  const finished = ['package_ready', 'decision_recorded'].includes(meeting.stage);
  return DEBATE_STAGES.map((stage, index) => {
    const items = contributions.filter(item => item.meetingId === meeting.id && item.stage === stage);
    const count = stage === 'proposing' ? items.reduce((total, item) => total + arr(item.content.claims).length, 0)
      : stage === 'critiquing' ? items.reduce((total, item) => total + arr(item.content.critiques).length, 0)
      : stage === 'revising' ? items.reduce((total, item) => total + arr(item.content.responses).length, 0)
      : stage === 'evaluating' ? items.reduce((total, item) => total + arr(item.content.assessments).length, 0) : items.length;
    const lines = items.map(item => {
      const ai = str(item.content.summary).trim();
      if (ai) return { who: nameOf(item.slotId), text: ai, ai: true };
      const accepted = arr(item.content.responses).filter(response => str(response.disposition) === 'accept').length;
      const text = stage === 'planning' ? '회의 순서와 참여자를 정했어요'
        : stage === 'proposing' ? `생각 ${arr(item.content.claims).length}개를 냈어요`
        : stage === 'critiquing' ? `약한 곳 ${arr(item.content.critiques).length}개를 짚었어요`
        : stage === 'revising' ? `지적 ${arr(item.content.responses).length}개 중 ${accepted}개를 받아들였어요`
        : `생각 ${arr(item.content.assessments).length}개를 심사하고 제안 ${arr(item.content.recommendations).length}개를 냈어요`;
      return { who: nameOf(item.slotId), text, ai: false };
    });
    return { stage, label: plainStageLabels[stage], lines, count, done: items.length > 0 && (finished || index < flowAt), current: !finished && index === flowAt };
  });
}

export type IssueStatus = '남음' | '확인 필요';
export type Issue = { text: string; ai: boolean; status: IssueStatus; items: DecisionPackage['dissent'] };

/** Open disagreements grouped for depth 1–2: the evaluator's plain issues when given, then any
 * remaining dissent grouped by the claim it targets. Nothing from the package is dropped. */
export function meetingIssues(pkg: DecisionPackage): Issue[] {
  const claimText = (id: string) => pkg.claims.find(claim => claim.id === id)?.text ?? '';
  const statusOf = (items: DecisionPackage['dissent']): IssueStatus => items.length > 0 && items.every(item => item.disposition === 'accept_but_flagged_by_evaluator') ? '확인 필요' : '남음';
  const covered = new Set<string>();
  const issues: Issue[] = [];
  for (const issue of pkg.issues ?? []) {
    const items = pkg.dissent.filter(item => issue.critiqueIds.includes(item.critiqueId));
    items.forEach(item => covered.add(item.critiqueId));
    if (issue.text.trim()) issues.push({ text: issue.text.trim(), ai: true, status: items.length ? statusOf(items) : '확인 필요', items });
  }
  const rest = new Map<string, DecisionPackage['dissent']>();
  for (const item of pkg.dissent.filter(item => !covered.has(item.critiqueId))) rest.set(item.targetClaimId, [...(rest.get(item.targetClaimId) ?? []), item]);
  for (const [claimId, items] of rest) {
    const target = shortGoal(claimText(claimId) || claimId, 40);
    issues.push({ text: `“${target}”에 반대 의견 ${items.length}개`, ai: false, status: statusOf(items), items });
  }
  return issues;
}

export const plainDisposition: Record<string, string> = { accept: '받아들임', rebut: '근거를 들어 반박함', defer: '나중으로 미룸', unanswered: '아직 답하지 않음', accept_but_flagged_by_evaluator: '받아들였지만 심사 담당이 다시 보자고 함' };

/** The always-visible control line (UXP-3). */
export function controlLine(meeting: Meeting): string {
  const parts: string[] = [];
  if (meeting.planApproval?.status === 'approved') parts.push(`계획 승인함${meeting.planApproval.mode === 'per_stage' ? '(단계마다 확인)' : ''}`);
  else if (meeting.plan) parts.push('계획 승인 전');
  parts.push(`AI에게 물어본 횟수 ${meeting.callsUsed}/${meeting.maxCalls}번`);
  parts.push(`보낸 자료 ${meeting.sources.length}개`);
  return parts.join(' · ');
}
