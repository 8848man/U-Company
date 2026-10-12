import type { Goal, Snapshot, Task } from './types';
import { meetingAttentionFor } from './meetingStatus';

export type OpenWork = { goal: Goal; projectName: string; tasks: Task[]; needsDecision: boolean; meetingLabel?: string };

export function openWork(snapshot: Snapshot): OpenWork[] {
  const superseded = new Set((snapshot.goalGuidance ?? [])
    .filter(item => item.status === 'revised' && item.replacementGoalId)
    .map(item => item.goalId));
  const pendingApprovals = new Set(snapshot.approvals.filter(item => item.status === 'pending').map(item => item.goalId));
  return snapshot.goals
    .filter(goal => goal.status !== 'completed' && !superseded.has(goal.id))
    .map(goal => ({
      goal,
      projectName: snapshot.projects.find(project => project.id === goal.projectId)?.name ?? '프로젝트 확인 필요',
      tasks: snapshot.tasks.filter(task => task.goalId === goal.id && task.status !== 'completed'),
      meetingLabel: goal.kind === 'deliberation' ? meetingAttentionFor(goal, snapshot).label : undefined,
      needsDecision: goal.kind === 'deliberation' ? meetingAttentionFor(goal, snapshot).needsDecision : goal.status === 'blocked' || goal.status === 'awaiting_approval' || pendingApprovals.has(goal.id),
    }))
    .sort((a, b) => Number(b.needsDecision) - Number(a.needsDecision) || b.goal.createdAt.localeCompare(a.goal.createdAt));
}
