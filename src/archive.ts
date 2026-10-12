import type { Goal, Meeting, Task } from './types';

/** Mirrors the engine rule (ADR-0043): only goals with nothing running or waiting can be archived.
 * Returns the reason it cannot, or null. The engine re-checks. */
export function archiveBlock(goal: Goal, tasks: Task[], meeting?: Meeting | null): string | null {
  if (goal.kind === 'deliberation') {
    const stage = meeting?.stage ?? 'decision_recorded';
    return goal.status === 'completed' || ['decision_recorded', 'legacy_closed', 'blocked', 'stopped'].includes(stage) ? null : '진행 중인 회의는 중단하거나 결정을 기록한 뒤 보관할 수 있어요.';
  }
  if (!['new', 'completed', 'blocked'].includes(goal.status)) return '진행 중이거나 승인을 기다리는 목표는 끝나거나 멈춘 뒤 보관할 수 있어요.';
  if (tasks.some(task => task.goalId === goal.id && ['running', 'awaiting_integration'].includes(task.status))) return '실행 중인 업무가 끝난 뒤 보관할 수 있어요.';
  return null;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
