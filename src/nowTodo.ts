// ADR-0061: '지금 할 일' is the one place for what the CEO does next in a goal conversation:
// start, recovery, continuing the original objective, the CEO's own actions, starting Git, or
// reopening a closed conversation. The highest-priority item names the bar; the dialog shows the rest.
import { canResume, type ConversationState } from './conversationObjective';
import type { ConversationObjective, CriteriaRequest, Goal } from './types';

export type NowTodoKind = 'start' | 'decision' | 'checking' | 'resume' | 'ceo' | 'git' | 'closed';
export type NowTodo = { kind: NowTodoKind; title: string };

const ACTIVE = ['analyzing', 'planning', 'awaiting_approval', 'executing', 'integrating', 'validating'];
const GIT_TEXT = /\.git\b|git init|git repository|git 저장소|깃 저장소|첫 커밋|초기 커밋|커밋 (id|이력)|commit (id|history)|git 이력|push|푸시|원격 저장소/i;

/** Whether starting Git is part of what this conversation is about (a folder project only). */
export function gitRelevant(input: { projectKind: string; goal: Goal; evidence: string; gitTurn: boolean; objective: ConversationObjective | null; criteria: CriteriaRequest[] }): boolean {
  if (input.projectKind !== 'folder' || input.goal.kind === 'deliberation') return false;
  if (input.gitTurn) return true;
  if (input.goal.status === 'blocked' && GIT_TEXT.test(input.evidence)) return true;
  if ((input.objective?.ceoActions ?? []).some(item => GIT_TEXT.test(item))) return true;
  return input.criteria.some(request => request.goalId === input.goal.id && request.items.some(item => GIT_TEXT.test(`${item.criterion} ${item.reason}`)));
}

export function ceoOnly(objective: ConversationObjective | null): boolean {
  return Boolean(objective?.ceoActions?.length) && !objective?.remaining.length;
}

export function nowTodo(input: { goal: Goal; versions: number; decisionLabel: string | null; startTitle: string; state: ConversationState; objective: ConversationObjective | null; git: boolean }): NowTodo | null {
  const { goal, state, objective } = input;
  if (goal.status === 'new') return { kind: 'start', title: input.startTitle };
  if (input.decisionLabel) return { kind: 'decision', title: input.decisionLabel };
  if (state === 'checking') return { kind: 'checking', title: '원래 목표를 확인하고 있어요' };
  if (canResume(state, goal)) {
    if (ceoOnly(objective)) return { kind: 'ceo', title: `CEO가 할 일 ${objective!.ceoActions!.length}개` };
    if (state === 'unchecked') return { kind: 'resume', title: '원래 목표 확인하고 이어서 하기' };
    return { kind: 'resume', title: objective?.remaining.length ? `남은 일 ${objective.remaining.length}개 이어서 하기` : '원래 목표 이어서 하기' };
  }
  if (input.git && !ACTIVE.includes(goal.status)) return { kind: 'git', title: 'Git 저장소 시작' };
  if (state === 'closed') return { kind: 'closed', title: '다른 목표로 달성되어 닫힌 대화' };
  if (state === 'achieved' && objective && input.versions > 1) return { kind: 'closed', title: '원래 목표 달성 · 닫힌 대화' };
  return null;
}
