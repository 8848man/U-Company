import type { Agent, Approval, BlockKind, DeliverableReview, Event, Goal, GoalGuidance, Task } from './types';
import { goalRecovery } from './goalRecovery';

/** Repair tasks refine the original deliverables; the engine prefixes their titles (ADR-0038). */
export const REPAIR_PREFIX = '재작업 ';

export type DeliverableRow = {
  task: Task;
  title: string;
  owner: string;
  reason: string;
  review: DeliverableReview | null;
};

/** Original deliverable tasks of a goal with their latest per-criterion QA review, if any. */
export function deliverableRows(goal: Goal, tasks: Task[], reviews: DeliverableReview[], agents: Agent[]): DeliverableRow[] {
  return tasks
    .filter(task => task.goalId === goal.id && (task.acceptance?.length ?? 0) > 0 && !task.title.startsWith(REPAIR_PREFIX))
    .map(task => {
      const latest = reviews.filter(item => item.goalId === goal.id && item.taskId === task.id).sort((a, b) => b.round - a.round || b.createdAt.localeCompare(a.createdAt))[0] ?? null;
      return { task, title: task.deliverable || task.title, owner: agents.find(agent => agent.id === task.assigneeId)?.name ?? task.assigneeId, reason: task.assignmentReason ?? '', review: latest };
    });
}

export const blockKindLabels: Record<BlockKind, string> = {
  precall: '호출 전 확인 실패 · AI 호출 없음',
  approval_denied: 'CEO가 변경 범위를 거절함',
  interrupted: '앱 종료로 실행 결과를 알 수 없음',
  integration_conflict: '파일 통합 충돌 · 원본 보존',
  qa_failed: 'QA 실패 · 재작업 한도 도달',
  needs_scope: '재작업에 새 파일 필요 · 범위 승인 필요',
  qa_form: 'QA 판정 형식 오류 · 완료 처리 안 함',
  plan_form: 'PM 계획 형식 오류 · 한 번 고쳐 받았지만 다시 틀림',
  provider: '실행 중 AI 호출 실패',
  other: '실행 중단',
};

export type BlockFacts = { reason: string; kindLabel: string; lastFact: string; owner: string; next: string };

/**
 * One reading of a goal that needs the CEO, shared by the goal card, the goal workspace and Office:
 * reason → last confirmed fact → owner → next action (GC-04, progress-visibility policy).
 */
export function blockFacts(goal: Goal, tasks: Task[], approvals: Approval[], guidance: GoalGuidance[], agents: Agent[], events: Event[]): BlockFacts | null {
  const recovery = goalRecovery(goal, tasks, approvals, guidance);
  if (!recovery) return null;
  const pendingApproval = approvals.find(item => item.goalId === goal.id && item.status === 'pending');
  const kind = (goal.blockKind || 'other') as BlockKind;
  const reason = goal.status === 'blocked'
    ? (goal.report.replace(/^Blocked:\s*/, '').split('\n')[0] || '저장된 중단 사유가 없습니다')
    : pendingApproval ? '팀이 프로젝트 변경 범위의 승인을 요청했습니다' : recovery.explanation;
  const latest = events.find(event => event.goalId === goal.id && event.kind !== 'runtime_progress') ?? events.find(event => event.goalId === goal.id);
  const lastFact = latest ? `${new Date(latest.at).toLocaleString()} · ${latest.message}` : '아직 세부 실행 기록이 없습니다';
  const stuck = tasks.filter(task => task.goalId === goal.id && ['blocked', 'failed'].includes(task.status));
  const owner = stuck.length ? `${[...new Set(stuck.map(task => agents.find(agent => agent.id === task.assigneeId)?.name ?? task.assigneeId))].join(', ')} → CEO 결정` : 'CEO';
  return { reason, kindLabel: goal.status === 'blocked' ? blockKindLabels[kind] ?? blockKindLabels.other : '승인 대기', lastFact, owner, next: recovery.label };
}

/** Minutes since the last confirmed record of a running task: a diagnostic only, never a state. */
export function quietMinutes(goal: Goal, tasks: Task[], events: Event[], now: number = Date.now()): number | null {
  if (!tasks.some(task => task.goalId === goal.id && task.status === 'running')) return null;
  const latest = events.find(event => event.goalId === goal.id);
  if (!latest) return null;
  const minutes = Math.floor((now - new Date(latest.at).getTime()) / 60000);
  return minutes >= 10 ? minutes : null;
}
