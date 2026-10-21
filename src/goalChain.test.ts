import { describe, expect, it } from 'vitest';
import { conversationGoals, goalChain, latestVersion, versionReason } from './goalChain';
import type { Goal } from './types';

const goal = (id: string) => ({ id } as Goal);
const link = (goalId: string, replacementGoalId: string | null, status = 'revised') => ({ goalId, replacementGoalId, status } as never);

describe('goal versions as one conversation (ADR-0051)', () => {
  const goals = [goal('v1'), goal('v2'), goal('v3'), goal('other')];
  const guidance = [link('v1', 'v2'), link('v2', 'v3'), link('other', null, 'pending')];

  it('joins the versions oldest first from any of them', () => {
    expect(goalChain('v2', guidance, goals)).toEqual(['v1', 'v2', 'v3']);
    expect(goalChain('other', guidance, goals)).toEqual(['other']);
    expect(latestVersion('v1', guidance, goals)).toBe('v3');
  });

  it('lists one row per conversation, its newest version', () => {
    expect(conversationGoals(goals, guidance).map(item => item.id)).toEqual(['v3', 'other']);
    const titled = conversationGoals([{ id: 'v1', intent: '랜딩 문구' } as Goal, { id: 'v2', intent: '랜딩 문구\n\nCEO 변경 지침: x' } as Goal], [link('v1', 'v2')]);
    expect(titled).toEqual([{ id: 'v2', intent: '랜딩 문구' }]);
  });

  it('stops at a version that is not on screen and survives a loop', () => {
    expect(goalChain('v1', guidance, [goal('v1'), goal('v2')])).toEqual(['v1', 'v2']);
    expect(goalChain('a', [link('a', 'b'), link('b', 'a')], [goal('a'), goal('b')])).toEqual(['b', 'a']);
  });

  it('a link that is not revised yet does not join', () => {
    expect(goalChain('v1', [link('v1', 'v2', 'review_required')], goals)).toEqual(['v1']);
  });

  it('says why a version was made', () => {
    expect(versionReason('멈춘 원인을 해결해 주세요.\n멈춘 이유: QA')).toContain('막힌 문제');
    expect(versionReason('버튼 색을 바꿔 주세요')).toContain('방향');
  });
});
