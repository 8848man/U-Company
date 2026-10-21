// In-app Office window (ADR-0046 decision 7): what the stage strip and member list show, and where the window sits.
// Saved state only; the window's place is a per-computer display preference.
import type { MemberActivity } from './progress';
import { phaseState } from './progress';
import type { Agent, Goal, Squad, Task } from './types';

export const STAGES = [
  { id: 'analyzing', label: '분석' },
  { id: 'planning', label: '계획' },
  { id: 'awaiting_approval', label: '승인' },
  { id: 'executing', label: '작업' },
  { id: 'integrating', label: '합치기' },
  { id: 'validating', label: '검토' },
  { id: 'completed', label: '완료' },
] as const;

export type StageState = 'done' | 'current' | 'waiting' | 'blocked' | 'pending';

/** One state per stage. Waiting for the CEO is told apart from work in progress, and a stop from completion. */
export function stageStrip(goal: Goal): { id: string; label: string; state: StageState }[] {
  return STAGES.map(stage => {
    if (stage.id === 'completed') return { ...stage, state: goal.status === 'completed' ? 'done' : 'pending' };
    if (goal.status === 'cancelled') return { ...stage, state: stage.id === 'awaiting_approval' ? 'blocked' : ['analyzing', 'planning'].includes(stage.id) ? 'done' : 'pending' };
    const state = phaseState(goal, stage.id);
    return { ...stage, state: state === 'current' && stage.id === 'awaiting_approval' ? 'waiting' : state };
  });
}

export type MemberRow = { id: string; name: string; role: string; work: string; state: string; kind: MemberActivity['kind'] | 'done' };

const DUTY: Record<string, string> = { analysis: '목표 분석', planning: '작업 계획' };

/** The goal's people: squad members first (leads, writers, reviewers), else anyone with a task in it. */
export function memberRows(goal: Goal, agents: Agent[], tasks: Task[], squad: Squad | null, activities: Record<string, MemberActivity>): MemberRow[] {
  const owned = tasks.filter(task => task.goalId === goal.id);
  const ids = squad ? squad.members.map(member => member.agentId) : [];
  for (const task of owned) if (!ids.includes(task.assigneeId)) ids.push(task.assigneeId);
  return [...new Set(ids)].flatMap(id => {
    const agent = agents.find(item => item.id === id);
    if (!agent) return [];
    const member = squad?.members.find(item => item.agentId === id);
    const mine = owned.filter(task => task.assigneeId === id);
    const running = mine.find(task => task.status === 'running');
    const activity = activities[id];
    const forThisGoal = activity?.goal?.id === goal.id;
    const work = running?.title ?? (member?.slot === 'lead' ? DUTY[member.duty] ?? '팀 리드' : member?.slot === 'reviewer' ? '결과 검토' : mine[0]?.title ?? '대기');
    const finished = mine.length > 0 && mine.every(task => task.status === 'completed');
    const kind: MemberRow['kind'] = running || (forThisGoal && activity.kind === 'active') ? 'active' : forThisGoal && activity.kind === 'blocked' ? 'blocked' : finished ? 'done' : 'idle';
    const state = { active: '작업 중', blocked: '멈춤', done: '끝남', idle: '대기', waiting: '대기' }[kind];
    return [{ id, name: agent.name, role: agent.role, work, state, kind }];
  });
}

export type WindowMode = 'floating' | 'docked' | 'minimized';
export type WindowPlace = { x: number; y: number; w: number; h: number; mode: WindowMode };
export const DEFAULT_PLACE: WindowPlace = { x: -1, y: 96, w: 440, h: 560, mode: 'floating' };
const KEY = 'ucompany.officeWindow';

/** Keep the window reachable: at least its title bar stays inside the app, and it is never larger than the app. */
export function clampPlace(place: WindowPlace, viewW: number, viewH: number): WindowPlace {
  const w = Math.max(320, Math.min(place.w, viewW - 16));
  const h = Math.max(240, Math.min(place.h, viewH - 16));
  const x = place.x < 0 ? viewW - w - 24 : Math.max(8 - w + 120, Math.min(place.x, viewW - 120));
  const y = Math.max(8, Math.min(place.y, viewH - 44));
  return { ...place, w, h, x, y };
}

export function loadPlace(): WindowPlace {
  try {
    const saved = JSON.parse(window.localStorage.getItem(KEY) ?? 'null');
    if (saved && ['floating', 'docked', 'minimized'].includes(saved.mode) && [saved.x, saved.y, saved.w, saved.h].every(Number.isFinite)) return saved;
  } catch { /* storage unavailable: use the default */ }
  return DEFAULT_PLACE;
}

export function savePlace(place: WindowPlace) {
  try { window.localStorage.setItem(KEY, JSON.stringify(place)); } catch { /* best effort */ }
}

/** Mini chat in the Office window: why this teammate cannot be asked right now, or null when they can. */
export function miniChatBlock(goal: Goal | null, agent: { id: string; connectionId?: string | null } | undefined): string | null {
  if (!goal) return '목표를 고르면 이 팀원과 대화할 수 있어요.';
  if (goal.kind === 'deliberation') return '회의 목표에서는 회의 화면에서 이야기해 주세요.';
  if (!goal.projectId) return '프로젝트가 연결된 목표에서만 대화할 수 있어요.';
  if (!agent) return '팀원을 찾을 수 없어요.';
  if (!agent.connectionId) return '이 팀원에게 연결된 AI가 없어요. 설정 › 팀에서 AI를 배정해 주세요.';
  return null;
}

/** The latest turns this teammate had in this goal's conversation, oldest first (the window shows a short session). */
export function miniChatTurns<T extends { agentId: string; askedAt: string }>(turns: T[], agentId: string, limit = 6): T[] {
  return turns.filter(turn => turn.agentId === agentId).sort((a, b) => a.askedAt.localeCompare(b.askedAt)).slice(-limit);
}
