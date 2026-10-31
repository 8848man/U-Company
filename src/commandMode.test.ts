import { describe, expect, it } from 'vitest';
import { ALLOW_ALL_RISKS, canTurnOn } from './commandMode';
import type { Approval, Goal } from './types';

const goal = (kind = 'execution') => ({ id: 'g', kind, projectId: 'p', intent: 'x', status: 'awaiting_approval', report: '', createdAt: 't' }) as unknown as Goal;
const approval = (operation: string, status: string, goalId = 'g'): Approval => ({ id: `${operation}-${status}`, goalId, operation, reason: '', status, createdAt: 't' });

describe('ADR-0063 allow-all switch', () => {
  it('is offered only on a pending plan card of an execution goal', () => {
    expect(canTurnOn(goal(), [approval('project_write', 'pending')])).toBe(true);
    expect(canTurnOn(goal(), [approval('project_write', 'approved')])).toBe(false);
    expect(canTurnOn(goal(), [approval('command_permission', 'pending')])).toBe(false);
    expect(canTurnOn(goal(), [approval('project_write', 'pending', 'other')])).toBe(false);
    expect(canTurnOn(goal('deliberation'), [approval('project_write', 'pending')])).toBe(false);
  });

  it('states that push and pull requests stay blocked and that it is not a guarantee', () => {
    const text = ALLOW_ALL_RISKS.join(' ');
    expect(text).toContain('push');
    expect(text).toContain('PR');
    expect(text).toContain('완전히 막지는 못해요');
    expect(text).toContain('언제든 끌 수 있어요');
  });
});
