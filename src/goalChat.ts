// v3 one-conversation goal flow (ADR-0046): what the stream shows and the progress line above the composer.
// Pure functions over saved state only; no AI call, no guess about completion.
import { quietMinutes } from './deliverables';
import type { Agent, Approval, Event, Goal, Squad, Task } from './types';

/** Subject particle for a name: 이 after a final consonant, 가 otherwise (non-Hangul endings read as 가). */
export function subject(name: string): string {
  const last = name.trim().slice(-1);
  const code = last.charCodeAt(0);
  if (code >= 0xac00 && code <= 0xd7a3) return `${name}${(code - 0xac00) % 28 ? '이' : '가'}`;
  return `${name}가`;
}

function shorten(text: string, max: number): string {
  const line = text.replace(/\s+/g, ' ').trim();
  return line.length > max ? `${line.slice(0, max - 1)}…` : line;
}

function leadName(duty: 'analysis' | 'planning' | 'reviewer', agents: Agent[], squad: Squad | null): string {
  const member = squad?.members.find(item => duty === 'reviewer' ? item.slot === 'reviewer' : item.slot === 'lead' && item.duty === duty);
  const fallbackId = duty === 'analysis' ? 'cto' : duty === 'planning' ? 'pm' : 'qa';
  const agent = agents.find(item => item.id === (member?.agentId ?? fallbackId));
  return agent?.name ?? (duty === 'analysis' ? 'CTO' : duty === 'planning' ? 'PM' : 'QA');
}

const WAITING = /^(다음 단계 )?담당이 다른 일을 마칠 때까지 기다립니다/;

/**
 * One line above the composer while the team works, replaced in place (progress-visibility "대화 진행 줄").
 * Null when nothing runs: a new, waiting-for-CEO, blocked, cancelled or completed goal shows no line.
 */
export function progressLine(goal: Goal, tasks: Task[], agents: Agent[], events: Event[], squad: Squad | null, now: number = Date.now()): string | null {
  const latest = events.find(event => event.goalId === goal.id && event.kind === 'runtime_progress');
  const waiting = latest && WAITING.test(latest.message) ? '다음 담당이 다른 일을 마칠 때까지 기다리는 중' : null;
  let line: string | null = null;
  if (goal.status === 'analyzing') line = waiting ?? `${subject(leadName('analysis', agents, squad))} 목표와 프로젝트를 살펴보는 중…`;
  else if (goal.status === 'planning') line = waiting ?? `${subject(leadName('planning', agents, squad))} 작업을 나누는 중…`;
  else if (goal.status === 'integrating') line = '작업 결과를 프로젝트에 합치는 중…';
  else if (goal.status === 'validating') line = waiting ?? `${subject(leadName('reviewer', agents, squad))} 결과를 확인하는 중…`;
  else if (goal.status === 'executing') {
    const running = tasks.filter(task => task.goalId === goal.id && task.status === 'running');
    if (running.length === 0) line = waiting ?? '다음 작업을 준비하는 중…';
    else {
      const name = (id: string) => agents.find(agent => agent.id === id)?.name ?? '팀원';
      const shown = running.slice(0, 2).map(task => `${subject(name(task.assigneeId))} 「${shorten(task.title, 24)}」`);
      const rest = new Set(running.slice(2).map(task => task.assigneeId)).size;
      line = `${shown.join(', ')} 작업 중…${rest ? ` 외 ${rest}명` : ''}`;
    }
  }
  if (!line) return null;
  const quiet = quietMinutes(goal, tasks, events, now);
  return quiet ? `${line} · 마지막 기록 후 ${quiet}분` : line;
}

/** Engine stage messages (English, fixed strings) → one plain Korean line in the stream. Unknown ones are skipped. */
const STAGE_LINES: Record<string, string> = {
  'CTO analysis completed; planning lead started': '분석을 마치고 계획을 세우기 시작했어요',
  'Implementation tasks delegated': '계획대로 작업을 시작했어요',
  'All implementation tasks completed; integration started': '작업을 마치고 결과를 합치고 있어요',
  'Changes integrated; QA review started': '결과를 합쳤어요. 검토를 시작했어요',
  'QA repair tasks delegated': '검토에서 고칠 점이 나와 다시 작업해요',
};

export type StreamLine = { id: string; at: string; text: string };

/** One line per saved stage change for this goal, oldest first. Plan cards come from approvals, not from here. */
export function stageLines(goal: Goal, events: Event[]): StreamLine[] {
  const lines: StreamLine[] = [];
  for (const event of events) {
    if (event.goalId !== goal.id) continue;
    let text: string | null = null;
    if (event.kind === 'goal_started') text = '팀이 목표를 받아 분석을 시작했어요';
    else if (event.kind === 'goal_stage') text = STAGE_LINES[event.message] ?? null;
    else if (event.kind === 'goal_completed') text = '검토를 통과했어요. 결과를 확인해 주세요';
    else if (event.kind === 'approval_decided' && /^project_write: /.test(event.message)) text = event.message.endsWith('approved') ? 'CEO가 계획을 승인했어요' : event.message.endsWith('denied') ? 'CEO가 계획을 거부했어요' : null;
    if (text) lines.push({ id: `event:${event.id}`, at: event.at, text });
  }
  return lines.sort((a, b) => a.at.localeCompare(b.at) || a.id.localeCompare(b.id));
}

export type PlanCard = { approval: Approval; current: boolean; status: 'pending' | 'approved' | 'denied' | 'other'; summary: string; counts: string; firstLine: string };

/** Plan cards for this goal's project-write approvals, oldest first. Only the pending one is current. */
export function planCards(goal: Goal, approvals: Approval[], tasks: Task[], summary: string | null): PlanCard[] {
  const owned = tasks.filter(task => task.goalId === goal.id && task.status !== 'superseded');
  const assignees = new Set(owned.map(task => task.assigneeId)).size;
  const files = new Set(owned.flatMap(task => task.files)).size;
  const counts = owned.length ? `작업 ${owned.length}개 · 팀원 ${assignees}명 · 파일 ${files}개` : '';
  return approvals
    .filter(item => item.goalId === goal.id && item.operation === 'project_write')
    .sort((a, b) => a.createdAt.localeCompare(b.createdAt))
    .map(approval => {
      const status = approval.status === 'pending' || approval.status === 'approved' || approval.status === 'denied' ? approval.status : 'other';
      return { approval, current: status === 'pending', status, summary: summary ?? '', counts: status === 'pending' || status === 'approved' ? counts : '', firstLine: approval.reason.split('\n')[0] };
    });
}

/** Collapsed label of a decided plan card. */
export function planCardLabel(card: PlanCard): string {
  return card.status === 'pending' ? '계획이 나왔어요' : card.status === 'approved' ? '계획 · CEO가 승인함' : card.status === 'denied' ? '계획 · CEO가 거부함' : '계획 · 바뀜';
}
