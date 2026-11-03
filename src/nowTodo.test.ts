import { describe, expect, it } from 'vitest';
import { gitRelevant, nowTodo } from './nowTodo';
import type { ConversationObjective, Goal } from './types';

const goal = (status: string): Goal => ({ id: 'g', status, kind: 'execution', intent: '앱', createdAt: 't' } as Goal);
const objective = (state: string, remaining: string[] = [], ceoActions: string[] = []): ConversationObjective => ({ conversationId: 'a', projectId: 'p', state, achieved: false, achievedBy: [], evidence: [], remaining, ceoActions, summary: '', projectChanges: '', checkedGoalId: 'g', checkedAt: 't' });
const base = { versions: 2, decisionLabel: null, startTitle: '팀 실행 시작', objective: null, git: false };

describe('지금 할 일 (ADR-0061)', () => {
  it('puts start and recovery before the original objective', () => {
    expect(nowTodo({ ...base, goal: goal('new'), state: 'running' })?.kind).toBe('start');
    expect(nowTodo({ ...base, goal: goal('blocked'), state: 'open', decisionLabel: '팀원에게 해결 요청' })).toEqual({ kind: 'decision', title: '팀원에게 해결 요청' });
  });

  it('continues an open objective, or shows only the CEO actions', () => {
    expect(nowTodo({ ...base, goal: goal('completed'), state: 'unchecked' })).toEqual({ kind: 'resume', title: '원래 목표 확인하고 이어서 하기' });
    expect(nowTodo({ ...base, goal: goal('completed'), state: 'open', objective: objective('open', ['화면', '시험']) })?.title).toBe('남은 일 2개 이어서 하기');
    expect(nowTodo({ ...base, goal: goal('completed'), state: 'open', objective: objective('open', [], ['GitHub에 푸시']) })).toEqual({ kind: 'ceo', title: 'CEO가 할 일 1개' });
    expect(nowTodo({ ...base, goal: goal('completed'), state: 'checking' })?.kind).toBe('checking');
  });

  it('offers Git only when nothing else is due and nothing runs, and a closed conversation can be reopened', () => {
    expect(nowTodo({ ...base, goal: goal('completed'), state: 'achieved', versions: 1, git: true })).toEqual({ kind: 'git', title: 'Git 저장소 시작' });
    expect(nowTodo({ ...base, goal: goal('executing'), state: 'running', git: true })).toBeNull();
    expect(nowTodo({ ...base, goal: goal('blocked'), state: 'closed', objective: objective('closed_by_other') })?.kind).toBe('closed');
    expect(nowTodo({ ...base, goal: goal('completed'), state: 'achieved', versions: 1, objective: objective('achieved') })).toBeNull();
  });

  it('finds Git in a stopped goal, the CEO actions or a stuck criterion of a folder project', () => {
    const input = { projectKind: 'folder', goal: goal('completed'), evidence: '', gitTurn: false, objective: null, criteria: [] };
    expect(gitRelevant(input)).toBe(false);
    expect(gitRelevant({ ...input, goal: goal('blocked'), evidence: 'fatal: not a git repository (.git)' })).toBe(true);
    expect(gitRelevant({ ...input, evidence: 'not a git repository' })).toBe(false);
    expect(gitRelevant({ ...input, objective: objective('open', [], ['GitHub 원격 저장소에 푸시']) })).toBe(true);
    expect(gitRelevant({ ...input, criteria: [{ id: 'c', goalId: 'g', stopped: false, createdAt: 't', decisions: [], items: [{ taskId: 't', taskTitle: '', index: 0, criterion: '변경 전 커밋 ID를 적는다', kind: 'precondition', reason: '', evidence: '', proposalAction: '', proposalText: '', proposalReason: '' }] }] })).toBe(true);
    expect(gitRelevant({ ...input, projectKind: 'git', objective: objective('open', [], ['푸시']) })).toBe(false);
  });
});
