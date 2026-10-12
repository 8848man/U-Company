import type { Agent, AiConnection, Goal, Squad, SquadMember, SquadSlot, Task } from './types';

/** Shared reading of goal squads and external staff (ADR-0041). The engine validates every change;
 * these helpers only decide what the UI offers and how it is labelled. */
export const structureLabels: Record<string, string> = { standard: '기본 팀', launch_prep: '출시 준비 팀' };
export const slotLabels: Record<SquadSlot, string> = { lead: '리드', writer: '작성', reviewer: '검토' };

export function structureLabel(structure: string): string { return structureLabels[structure] ?? '기본 팀'; }

export function squadFor(goalId: string | null | undefined, squads: Squad[] | undefined): Squad | null {
  return goalId ? (squads ?? []).find(squad => squad.goalId === goalId) ?? null : null;
}

export function isExternal(agent: Agent | undefined): boolean {
  return Boolean(agent?.external && agent.external.status !== 'hired');
}

export function displayName(agent: Agent | undefined, id: string): string {
  if (!agent) return id;
  return isExternal(agent) ? `${agent.name}(외부)` : agent.name;
}

export function memberLabel(member: SquadMember, agents: Agent[]): string {
  const name = displayName(agents.find(agent => agent.id === member.agentId), member.agentId);
  return member.duty ? `${name}(${member.duty === 'analysis' ? '분석' : '계획'})` : name;
}

/** "리드 … · 작성 … · 검토 …" in one line for cards. */
export function squadSummary(squad: Squad, agents: Agent[]): string {
  return (['lead', 'writer', 'reviewer'] as SquadSlot[]).map(slot => {
    const names = squad.members.filter(member => member.slot === slot).map(member => memberLabel(member, agents));
    return names.length ? `${slotLabels[slot]} ${names.join(', ')}` : '';
  }).filter(Boolean).join(' · ');
}

const unusable = ['missing', 'failed', 'needs_setup'];
const isWriterRole = (agent: Agent) => !['cto', 'pm', 'qa'].includes(agent.id) && agent.role.toLowerCase().includes('implement');
const isReviewerRole = (agent: Agent) => agent.id === 'qa' || /review|validation/i.test(agent.role);

/** Characters the CEO may pick to replace `slot` on the approval card: same role, usable
 * connection (file-capable for writers), not already in the squad, or standby external staff whose
 * character file is for that slot. Busy state is re-checked by the engine. */
export function swapCandidates(squad: Squad, slot: SquadSlot, agents: Agent[], connections: AiConnection[]): Agent[] {
  if (slot === 'lead') return [];
  const members = new Set(squad.members.map(member => member.agentId));
  return agents.filter(agent => {
    if (!agent.enabled || members.has(agent.id)) return false;
    const connection = connections.find(item => item.id === agent.connectionId);
    if (!connection || unusable.includes(connection.status)) return false;
    if (slot === 'writer' && connection.kind === 'http') return false;
    if (agent.external && agent.external.status !== 'hired') return agent.external.status === 'standby' && agent.external.slot === slot;
    return slot === 'writer' ? isWriterRole(agent) : isReviewerRole(agent);
  });
}

/** Office badges: the slot a character holds in an unfinished goal's squad, and external staff
 * status. Derived from saved squads only. */
export function officeBadges(squads: Squad[] | undefined, goals: Goal[], agents: Agent[]): Record<string, string> {
  const badges: Record<string, string> = {};
  const active = (squads ?? []).filter(squad => ['proposed', 'locked'].includes(squad.status) && goals.some(goal => goal.id === squad.goalId && goal.status !== 'completed'));
  for (const squad of active) {
    const goal = goals.find(item => item.id === squad.goalId);
    const short = goal ? goal.intent.slice(0, 12).trim() : '';
    for (const member of squad.members) {
      const label = `${slotLabels[member.slot]} · ${short}${goal && goal.intent.length > 12 ? '…' : ''}`;
      badges[member.agentId] = badges[member.agentId] ? `${badges[member.agentId]} / ${label}` : label;
    }
  }
  for (const agent of agents) {
    if (!isExternal(agent)) continue;
    const status = agent.external!.status;
    const prefix = status === 'standby' ? '외부 · 대기' : status === 'contracted' ? '외부 · 계약' : '외부 · 종료';
    badges[agent.id] = badges[agent.id] ? `${prefix} · ${badges[agent.id]}` : prefix;
  }
  return badges;
}

export const externalStatusLabels: Record<string, string> = { standby: '대기 · 승인 카드에서 스쿼드에 넣을 수 있음', contracted: '이 목표 동안만 계약', released: '계약 종료 · 기록만 남음', hired: '정식 채용됨' };

const slotCounts: Record<string, [number, number]> = { standard: [2, 1], launch_prep: [3, 2] };

/** Empty writer/reviewer places in the proposed structure, and the tasks a new writer could take
 * (only from writers holding two or more). Mirrors the engine's add rule; the engine re-checks. */
export function openSlots(squad: Squad, tasks: Task[]): { writer: number; reviewer: number; movable: Task[] } {
  const [writers, reviewers] = slotCounts[squad.structure] ?? slotCounts.standard;
  const own = tasks.filter(task => task.goalId === squad.goalId);
  const count = (slot: SquadSlot) => squad.members.filter(member => member.slot === slot).length;
  const load = (id: string) => own.filter(task => task.assigneeId === id).length;
  return {
    writer: Math.max(0, Math.min(writers, own.length) - count('writer')),
    reviewer: Math.max(0, reviewers - count('reviewer')),
    movable: own.filter(task => load(task.assigneeId) >= 2),
  };
}
