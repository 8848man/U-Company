import { describe, expect, it } from 'vitest';
import { canResume, conversationOf, conversationState, inProgress, listLabel, objectiveLine } from './conversationObjective';
import type { ConversationObjective, Goal, GoalGuidance } from './types';

const goal = (id: string, status: string): Goal => ({ id, status, kind: 'execution', intent: id, createdAt: id } as Goal);
const link = (from: string, to: string) => ({ goalId: from, status: 'revised', replacementGoalId: to } as GoalGuidance);
const objective = (state: string, remaining: string[] = []): ConversationObjective => ({ conversationId: 'a', projectId: 'p', state, achieved: false, achievedBy: [], evidence: [], remaining, summary: '요약', projectChanges: '', checkedGoalId: 'b', checkedAt: 't' });

describe('conversation objective (ADR-0059)', () => {
  it('keeps a conversation in progress when a small newest pass completed but the objective is open', () => {
    const { rootId, versions, newest } = conversationOf('b', [link('a', 'b')], [goal('a', 'blocked'), goal('b', 'completed')]);
    expect(rootId).toBe('a');
    expect(conversationState(versions, null)).toBe('unchecked');
    expect(conversationState(versions, objective('open', ['화면']))).toBe('open');
    expect(inProgress('open', newest)).toBe(true);
    expect(canResume('open', newest)).toBe(true);
    expect(listLabel('open', objective('open', ['화면', '시험']), '완료')).toBe('진행 중 · 남은 일 2');
  });
  it('closes achieved conversations and a single completed goal of older records', () => {
    expect(conversationState([goal('a', 'completed')], null)).toBe('achieved');
    expect(conversationState([goal('a', 'blocked'), goal('b', 'blocked')], objective('closed_by_other'))).toBe('closed');
    expect(inProgress('closed', goal('b', 'blocked'))).toBe(false);
    expect(canResume('closed', goal('b', 'blocked'))).toBe(false);
    expect(objectiveLine('closed', objective('closed_by_other'))).toBe('다른 목표로 원래 목표가 달성되어 닫았어요 · 요약');
  });
  it('does not offer resume while a version runs', () => {
    const versions = [goal('a', 'completed'), goal('b', 'executing')];
    expect(conversationState(versions, objective('open'))).toBe('running');
    expect(canResume('running', versions[1])).toBe(false);
    expect(conversationState(versions, objective('checking'))).toBe('checking');
  });
});
