import { describe, expect, it } from 'vitest';
import { choiceProblem, decidedLine, decisionsOf, defaultChoices, itemKey } from './criteriaDecision';
import type { CriteriaRequest } from './types';

const request: CriteriaRequest = {
  id: 'c', goalId: 'g', stopped: false, createdAt: 't', decisions: [],
  items: [
    { taskId: 't1', taskTitle: 'README', index: 0, criterion: '커밋 ID를 적는다', kind: 'precondition', reason: 'Git 이력이 없음', evidence: '', proposalAction: 'replace', proposalText: '비교는 미확인으로 표시한다', proposalReason: '' },
    { taskId: 't1', taskTitle: 'README', index: 2, criterion: '푸시를 완료한다', kind: 'precondition', reason: '푸시 불가', evidence: '', proposalAction: 'drop', proposalText: '', proposalReason: '' },
    { taskId: 't2', taskTitle: '측정', index: 1, criterion: '범위를 나눈다', kind: 'no_progress', reason: '', evidence: '', proposalAction: '', proposalText: '', proposalReason: '' },
  ],
};

describe('criteria decision (ADR-0060)', () => {
  it('preselects the planner suggestion and keeps criteria without one', () => {
    const choices = defaultChoices(request);
    expect(choices[itemKey(request.items[0])]).toEqual({ action: 'replace', text: '비교는 미확인으로 표시한다' });
    expect(choices[itemKey(request.items[1])].action).toBe('drop');
    expect(choices[itemKey(request.items[2])].action).toBe('keep');
    expect(decisionsOf(request, choices)).toEqual([
      { taskId: 't1', index: 0, action: 'replace', text: '비교는 미확인으로 표시한다' },
      { taskId: 't1', index: 2, action: 'drop', text: '' },
      { taskId: 't2', index: 1, action: 'keep', text: '' },
    ]);
  });

  it('refuses an empty, multi-line or too long replacement', () => {
    const choices = defaultChoices(request);
    const key = itemKey(request.items[0]);
    expect(choiceProblem(request, choices)).toBeNull();
    expect(choiceProblem(request, { ...choices, [key]: { action: 'replace', text: ' ' } })).toContain('적어');
    expect(choiceProblem(request, { ...choices, [key]: { action: 'replace', text: 'a\nb' } })).toContain('한 줄');
    expect(choiceProblem(request, { ...choices, [key]: { action: 'replace', text: '가'.repeat(301) } })).toContain('300자');
  });

  it('summarizes a decided card', () => {
    expect(decidedLine({ ...request, decisions: [{ taskId: 't1', index: 0, action: 'drop', text: '' }, { taskId: 't2', index: 1, action: 'keep', text: '' }] })).toBe('뺀 기준 1개 · 그대로 1개 · 파일은 그대로 두고 QA가 다시 확인했어요');
    expect(decidedLine({ ...request, stopped: true })).toContain('멈추기');
  });
});
