// ADR-0059: a conversation (a goal and its versions) is judged by its original objective, not only
// by its newest version. These rules decide what the list, open work and the conversation show.
import { goalChain } from './goalChain';
import type { ConversationObjective, Goal, GoalGuidance } from './types';

export type ConversationState = 'running' | 'checking' | 'open' | 'unchecked' | 'achieved' | 'closed' | 'reopened';
type Link = Pick<GoalGuidance, 'goalId' | 'status' | 'replacementGoalId'>;

export function conversationOf(goalId: string, guidance: Link[], goals: Goal[]) {
  const ids = goalChain(goalId, guidance, goals);
  const versions = ids.map(id => goals.find(goal => goal.id === id)).filter((goal): goal is Goal => Boolean(goal));
  return { rootId: ids[0], versions, newest: versions[versions.length - 1] };
}

export function objectiveFor(rootId: string, objectives: ConversationObjective[] | undefined): ConversationObjective | null {
  return objectives?.find(item => item.conversationId === rootId) ?? null;
}

export function conversationState(versions: Goal[], objective: ConversationObjective | null): ConversationState {
  const newest = versions[versions.length - 1];
  if (objective?.state === 'checking') return 'checking';
  if (objective?.state === 'closed_by_other') return 'closed';
  if (objective?.state === 'achieved') return 'achieved';
  if (newest && newest.status !== 'completed' && newest.status !== 'blocked' && newest.status !== 'cancelled') return 'running';
  if (objective?.state === 'reopened') return 'reopened';
  if (objective?.state === 'open') return 'open';
  // Older records: a single completed version is its own objective; a longer chain was never checked.
  if (newest?.status === 'completed') return versions.length > 1 ? 'unchecked' : 'achieved';
  return 'open';
}

/** Still the CEO's work: running, being checked, open, reopened, or a longer chain not yet checked. */
export function inProgress(state: ConversationState, newest: Goal | undefined): boolean {
  if (state === 'achieved' || state === 'closed') return false;
  return newest?.status !== 'cancelled';
}

/** `이어서 하기` is offered when nothing is running and the objective is not closed. */
export function canResume(state: ConversationState, newest: Goal | undefined): boolean {
  return Boolean(newest) && newest!.kind !== 'deliberation' && (newest!.status === 'completed' || newest!.status === 'blocked') && ['open', 'unchecked', 'reopened'].includes(state);
}

export function objectiveLine(state: ConversationState, objective: ConversationObjective | null): string {
  switch (state) {
    case 'checking': return '원래 목표를 확인하고 있어요';
    case 'closed': return `다른 목표로 원래 목표가 달성되어 닫았어요${objective?.summary ? ` · ${objective.summary}` : ''}`;
    case 'achieved': return objective?.summary ? `원래 목표 달성 · ${objective.summary}` : '원래 목표 달성';
    case 'unchecked': return '원래 목표가 끝났는지 아직 확인하지 않았어요';
    case 'reopened': return '다시 연 대화예요 · 이어서 하면 원래 목표를 다시 확인해요';
    case 'open': return objective?.remaining.length ? `원래 목표에서 남은 일 ${objective.remaining.length}개${objective.summary ? ` · ${objective.summary}` : ''}` : objective?.summary || '원래 목표가 아직 끝나지 않았어요';
    default: return '';
  }
}

/** Short label for the goal list. */
export function listLabel(state: ConversationState, objective: ConversationObjective | null, fallback: string): string {
  if (state === 'closed') return '다른 목표로 달성되어 닫힘';
  if (state === 'achieved') return '원래 목표 달성';
  if (state === 'checking') return '원래 목표 확인 중';
  if (state === 'unchecked') return '진행 중 · 확인 전';
  if ((state === 'open' || state === 'reopened') && objective?.remaining.length && fallback === '완료') return `진행 중 · 남은 일 ${objective.remaining.length}`;
  if ((state === 'open' || state === 'reopened') && fallback === '완료') return '진행 중 · 원래 목표 남음';
  return fallback;
}
