import type { Agent, Event, Goal, Task } from './types';

export type MemberActivity = {
  kind: 'active' | 'waiting' | 'blocked' | 'idle';
  label: string;
  goal: Goal | null;
  task: Task | null;
  events: Event[];
};

const activeGoals = new Set(['analyzing', 'planning', 'executing', 'integrating', 'validating']);

function stageAssignee(agents: Agent[], status: string): Agent | undefined {
  if (status === 'analyzing') return agents.find(agent => agent.enabled && agent.id === 'cto') ?? agents.find(agent => agent.enabled && agent.role.toLowerCase().includes('architect'));
  if (status === 'planning') return agents.find(agent => agent.enabled && agent.id === 'pm') ?? agents.find(agent => agent.enabled && agent.role.toLowerCase().includes('plan'));
  if (status === 'validating') return agents.find(agent => agent.enabled && agent.id === 'qa') ?? agents.find(agent => agent.enabled && agent.role.toLowerCase().includes('review'));
  return undefined;
}

export function memberActivity(agent: Agent, agents: Agent[], goals: Goal[], tasks: Task[], events: Event[]): MemberActivity {
  const recentGoals = goals.slice().sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  const ownTasks = tasks.filter(task => task.assigneeId === agent.id);
  const running = ownTasks.find(task => task.status === 'running');
  const stageGoal = recentGoals.find(goal => activeGoals.has(goal.status) && stageAssignee(agents, goal.status)?.id === agent.id);
  const blockedGoal = recentGoals.find(goal => goal.status === 'blocked' && stageAssignee(agents, goal.resumedFrom ?? '')?.id === agent.id);
  const task = running ?? ownTasks.find(item => ['failed', 'blocked', 'awaiting_integration'].includes(item.status)) ?? null;
  const goal = running ? recentGoals.find(item => item.id === running.goalId) ?? null : stageGoal ?? blockedGoal ?? (task ? recentGoals.find(item => item.id === task.goalId) ?? null : null);
  const relevant = events.filter(event => (event.agentId === agent.id || event.kind === 'goal_blocked' && event.goalId === blockedGoal?.id) && (!goal || event.goalId === goal.id)).slice(0, 24);
  if (running) return { kind: 'active', label: running.title, goal, task: running, events: relevant };
  if (stageGoal) return { kind: 'active', label: stageGoal.status === 'analyzing' ? '목표 분석' : stageGoal.status === 'planning' ? '업무 계획' : '품질 확인', goal: stageGoal, task: null, events: relevant };
  if (blockedGoal) return { kind: 'blocked', label: blockedGoal.resumedFrom === 'analyzing' ? '목표 분석' : blockedGoal.resumedFrom === 'planning' ? '업무 계획' : '품질 확인', goal: blockedGoal, task: null, events: relevant };
  if (task?.status === 'failed' || task?.status === 'blocked') return { kind: 'blocked', label: task.title, goal, task, events: relevant };
  if (task?.status === 'awaiting_integration') return { kind: 'waiting', label: '결과 통합 대기', goal, task, events: relevant };
  return { kind: 'idle', label: '대기 중', goal: null, task: null, events: events.filter(event => event.agentId === agent.id).slice(0, 12) };
}

export const phases = [
  { id: 'analyzing', label: '분석' },
  { id: 'planning', label: '계획' },
  { id: 'awaiting_approval', label: '승인' },
  { id: 'executing', label: '구현' },
  { id: 'integrating', label: '통합' },
  { id: 'validating', label: '검증' },
] as const;

export function phaseState(goal: Goal, phase: string): 'done' | 'current' | 'blocked' | 'pending' {
  if (goal.status === 'completed') return 'done';
  const current = goal.status === 'blocked' ? goal.resumedFrom : goal.status;
  const currentIndex = phases.findIndex(item => item.id === current);
  const index = phases.findIndex(item => item.id === phase);
  if (currentIndex < 0 || index < 0) return 'pending';
  if (index < currentIndex) return 'done';
  if (index === currentIndex) return goal.status === 'blocked' ? 'blocked' : 'current';
  return 'pending';
}

/** A goal's display title (ADR-0042): the AI summary when one was received, else a short excerpt. */
export function goalTitle(goal: Pick<Goal, 'intent' | 'summary'>, limit = 110): string {
  return goal.summary?.trim() ? goal.summary.trim() : shortGoal(goal.intent, limit);
}

export function shortGoal(intent: string, limit = 110): string {
  const text = intent.trim().replace(/\s+/g, ' ');
  if (Array.from(text).length <= limit) return text;
  const firstSentence = text.match(/^.*?[.!?。！？](?=\s|$)/u)?.[0];
  if (firstSentence && Array.from(firstSentence).length <= limit) return firstSentence;
  return `${Array.from(text).slice(0, limit).join('').trimEnd()}…`;
}

export function groupGoals(goals: Goal[]): { key: string; label: string | null; goals: Goal[] }[] {
  const groups = new Map<string, { key: string; label: string | null; goals: Goal[] }>();
  for (const goal of [...goals].sort((a, b) => b.createdAt.localeCompare(a.createdAt))) {
    const label = goal.workGroup?.trim() || null;
    const key = label ? `${goal.projectId ?? ''}:${label.toLocaleLowerCase()}` : `goal:${goal.id}`;
    const existing = groups.get(key);
    if (existing) existing.goals.push(goal);
    else groups.set(key, { key, label, goals: [goal] });
  }
  return [...groups.values()];
}

export function roleDialogue(agent: Agent): string[] {
  const role = agent.role.toLowerCase();
  if (agent.id === 'cto' || role.includes('architect')) return ['목표의 범위와 위험을 먼저 확인해요.', '팀이 같은 기준으로 움직이도록 방향을 정하겠습니다.'];
  if (agent.id === 'pm' || role.includes('plan')) return ['목표를 담당 파일과 완료 기준으로 나눠요.', '서로 겹치지 않는 업무를 배정하겠습니다.'];
  if (agent.id === 'qa' || role.includes('review')) return ['완료된 결과를 독립적으로 확인해요.', '테스트 근거와 남은 위험을 보고하겠습니다.'];
  if (role.includes('implement')) return ['맡은 파일만 작업하고 테스트해요.', '진행 상황은 실제 실행 기록으로 보여드리겠습니다.'];
  return ['제 역할에 맞는 업무를 기다리고 있어요.', '업무가 배정되면 진행 기록이 여기에 나타납니다.'];
}
