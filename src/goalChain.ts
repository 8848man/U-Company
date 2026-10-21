// ADR-0051: a goal and the versions that replaced it are one conversation on screen.
// Storage keeps each version as its own goal (GC-02); only the view joins them.
import type { Goal, GoalGuidance } from './types';

type Link = Pick<GoalGuidance, 'goalId' | 'status' | 'replacementGoalId'>;

/** The version ids of `goalId`'s conversation, oldest first. Only goals in `present` count (archived ones are gone). */
export function goalChain(goalId: string, guidance: Link[], present: { id: string }[]): string[] {
  const exists = (id: string) => present.some(goal => goal.id === id);
  const links = guidance.filter(item => item.status === 'revised' && item.replacementGoalId);
  const chain = [goalId];
  for (let current = goalId; ;) {
    const previous = links.find(item => item.replacementGoalId === current)?.goalId;
    if (!previous || chain.includes(previous) || !exists(previous)) break;
    chain.unshift(previous); current = previous;
  }
  for (let current = goalId; ;) {
    const next = links.find(item => item.goalId === current)?.replacementGoalId ?? null;
    if (!next || chain.includes(next) || !exists(next)) break;
    chain.push(next); current = next;
  }
  return chain;
}

/** The newest version of the conversation `goalId` belongs to. */
export function latestVersion(goalId: string, guidance: Link[], present: { id: string }[]): string {
  const chain = goalChain(goalId, guidance, present);
  return chain[chain.length - 1];
}

/**
 * Goals for a conversation list: one row per conversation, its newest version, titled with the first
 * version's words so the row keeps its name (later versions add the CEO's change to the intent).
 */
export function conversationGoals<T extends Goal>(goals: T[], guidance: Link[]): T[] {
  return goals.filter(goal => latestVersion(goal.id, guidance, goals) === goal.id).map(goal => {
    const first = goals.find(item => item.id === goalChain(goal.id, guidance, goals)[0]);
    return first && first.id !== goal.id ? { ...goal, intent: first.intent } : goal;
  });
}

/** Why a version was made, from the guidance that created it. */
export function versionReason(text: string): string {
  return text.startsWith('멈춘 원인을 해결해 주세요') ? '막힌 문제를 고치려고 다시 계획해요' : '바뀐 방향을 반영해 다시 계획해요';
}
