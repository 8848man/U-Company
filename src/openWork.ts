import type { Goal, Snapshot, Task } from './types';
import { meetingAttentionFor } from './meetingStatus';
import { conversationOf, conversationState, inProgress, objectiveFor } from './conversationObjective';

export type OpenWork = { goal: Goal; projectName: string; tasks: Task[]; needsDecision: boolean; meetingLabel?: string };

/** The launcher badge always uses the selected project; the expanded panel may opt into all projects. */
export function currentProjectWork(items: OpenWork[], projectId: string | null): OpenWork[] {
  return items.filter(item => item.goal.projectId === projectId && projectId !== null);
}

export function openWork(snapshot: Snapshot): OpenWork[] {
  const superseded = new Set((snapshot.goalGuidance ?? [])
    .filter(item => item.status === 'revised' && item.replacementGoalId)
    .map(item => item.goalId));
  const pendingApprovals = new Set(snapshot.approvals.filter(item => item.status === 'pending').map(item => item.goalId));
  // ADR-0059: one entry per conversation (its newest version). A conversation whose newest pass
  // completed stays while its original objective is open; a closed or achieved one leaves.
  const stillOpen = (goal: Goal) => {
    if (goal.kind === 'deliberation') return goal.status !== 'completed';
    const { rootId, versions } = conversationOf(goal.id, snapshot.goalGuidance ?? [], snapshot.goals);
    const state = conversationState(versions, objectiveFor(rootId, snapshot.conversationObjectives));
    return inProgress(state, goal) && (goal.status !== 'completed' || ['open', 'unchecked', 'reopened', 'checking'].includes(state));
  };
  return snapshot.goals
    .filter(goal => !superseded.has(goal.id) && stillOpen(goal))
    .map(goal => ({
      goal,
      projectName: snapshot.projects.find(project => project.id === goal.projectId)?.name ?? '프로젝트 확인 필요',
      tasks: snapshot.tasks.filter(task => task.goalId === goal.id && task.status !== 'completed'),
      meetingLabel: goal.kind === 'deliberation' ? meetingAttentionFor(goal, snapshot).label : undefined,
      needsDecision: goal.kind === 'deliberation' ? meetingAttentionFor(goal, snapshot).needsDecision : ['blocked', 'paused', 'awaiting_approval'].includes(goal.status) || pendingApprovals.has(goal.id),
    }))
    .sort((a, b) => Number(b.needsDecision) - Number(a.needsDecision) || b.goal.createdAt.localeCompare(a.goal.createdAt));
}
