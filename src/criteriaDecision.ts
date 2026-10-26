// ADR-0060: the CEO's choices on a criteria card. The planner's suggestion is preselected but nothing
// is decided until the CEO presses a button; a replacement is one line of at most 300 characters.
import type { CriteriaDecision, CriteriaRequest, StuckCriterion } from './types';

export type Choice = { action: 'drop' | 'replace' | 'keep'; text: string };
export const MAX_CRITERION = 300;

export const itemKey = (item: Pick<StuckCriterion, 'taskId' | 'index'>) => `${item.taskId}:${item.index}`;

export function kindLabel(kind: string): string {
  if (kind === 'precondition') return '팀이 채울 수 없음';
  if (kind === 'no_progress') return '재작업해도 그대로';
  return 'PM 수정 제안';
}

export function defaultChoices(request: CriteriaRequest): Record<string, Choice> {
  return Object.fromEntries(request.items.map(item => [itemKey(item), item.proposalAction === 'drop' ? { action: 'drop', text: '' } : item.proposalAction === 'replace' ? { action: 'replace', text: item.proposalText } : { action: 'keep', text: '' }]));
}

/** Why the choices cannot be sent yet, or `null`. */
export function choiceProblem(request: CriteriaRequest, choices: Record<string, Choice>): string | null {
  for (const item of request.items) {
    const choice = choices[itemKey(item)];
    if (choice?.action !== 'replace') continue;
    const text = choice.text.trim();
    if (!text) return '바꿀 기준을 적어 주세요';
    if (text.includes('\n')) return '바꿀 기준은 한 줄로 적어 주세요';
    if ([...text].length > MAX_CRITERION) return `바꿀 기준은 ${MAX_CRITERION}자 이하로 적어 주세요`;
  }
  return null;
}

export function decisionsOf(request: CriteriaRequest, choices: Record<string, Choice>): CriteriaDecision[] {
  return request.items.map(item => {
    const choice = choices[itemKey(item)] ?? { action: 'keep', text: '' };
    return { taskId: item.taskId, index: item.index, action: choice.action, text: choice.action === 'replace' ? choice.text.trim() : '' };
  });
}

/** One line summarizing what the CEO decided, for a decided card. */
export function decidedLine(request: CriteriaRequest | undefined): string {
  if (!request) return '';
  if (request.stopped) return '멈추기를 골랐어요. 해결 요청으로 다음 계획 버전에서 기준을 다시 세워요.';
  const count = (action: string) => request.decisions.filter(item => item.action === action).length;
  const parts = [count('drop') && `뺀 기준 ${count('drop')}개`, count('replace') && `바꾼 기준 ${count('replace')}개`, count('keep') && `그대로 ${count('keep')}개`].filter(Boolean);
  return parts.length ? `${parts.join(' · ')} · 파일은 그대로 두고 QA가 다시 확인했어요` : '';
}
