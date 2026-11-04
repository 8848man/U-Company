import { useState } from 'react';
import { ContentPreview } from './ContentDensity';
import { choiceProblem, decisionsOf, defaultChoices, itemKey, kindLabel, MAX_CRITERION, type Choice } from './criteriaDecision';
import type { Approval, CriteriaDecision, CriteriaRequest } from './types';

type Props = { approval: Approval; request: CriteriaRequest | undefined; working: boolean; onDecide: (decisions: CriteriaDecision[], stop: boolean) => void };

const ACTIONS: { action: Choice['action']; label: string }[] = [{ action: 'drop', label: '빼기' }, { action: 'replace', label: '바꾸기' }, { action: 'keep', label: '그대로' }];

/** ADR-0060: criteria the team cannot meet stop here instead of spending rework rounds. */
export function CriteriaDecisionCard({ approval, request, working, onDecide }: Props) {
  const [choices, setChoices] = useState<Record<string, Choice> | null>(null);
  if (!request) return <section className="gc-plan is-criteria" aria-label="완료 기준 결정"><h3>완료 기준을 정해 주세요</h3><p className="muted">요청 기록을 불러오는 중이에요.</p></section>;
  const current = choices ?? defaultChoices(request);
  const set = (key: string, choice: Choice) => setChoices({ ...current, [key]: choice });
  const problem = choiceProblem(request, current);
  return <section className="gc-plan is-criteria" aria-label="완료 기준 결정">
    <h3>완료 기준을 정해 주세요</h3>
    <p>QA가 아래 기준을 통과시키지 못했는데, 팀이 채울 수 없거나 재작업해도 그대로예요. 재작업을 더 쓰지 않고 멈췄어요. 기준마다 고르면 <b>파일은 그대로 두고 QA만 다시</b> 확인해요.</p>
    <ol className="gc-criteria">{request.items.map(item => {
      const key = itemKey(item);
      const choice = current[key] ?? { action: 'keep', text: '' };
      return <li key={key}>
        <div className="gc-criteria-head"><span className={`gc-criteria-kind is-${item.kind}`}>{kindLabel(item.kind)}</span><small className="muted">{item.taskTitle}</small></div>
        <p className="gc-criteria-text">{item.criterion}</p>
        {item.reason && <p className="gc-criteria-reason">{item.reason}</p>}
        {(item.evidence || item.proposalReason) && <details><summary>QA 근거{item.proposalAction ? '와 PM 제안 이유' : ''}</summary><ContentPreview><p className="muted">{item.evidence}</p>{item.proposalReason && <p className="muted">PM: {item.proposalReason}</p>}</ContentPreview></details>}
        <div className="gc-criteria-actions" role="group" aria-label="이 기준을">
          {ACTIONS.map(option => <button type="button" key={option.action} className={choice.action === option.action ? 'primary' : 'secondary'} aria-pressed={choice.action === option.action} disabled={working} onClick={() => set(key, { action: option.action, text: option.action === 'replace' ? (choice.text || item.proposalText || item.criterion) : '' })}>{option.label}{item.proposalAction === option.action ? ' · PM 제안' : ''}</button>)}
        </div>
        {choice.action === 'replace' && <label className="gc-criteria-edit"><span>바꿀 기준</span><input value={choice.text} maxLength={MAX_CRITERION} disabled={working} onChange={event => set(key, { action: 'replace', text: event.target.value })} /></label>}
      </li>;
    })}</ol>
    {problem && <small className="gc-criteria-problem">{problem}</small>}
    <small className="muted">기준을 바꾸거나 빼는 건 CEO만 할 수 있어요 · PM 제안과 QA 근거는 AI가 쓴 글이에요 · 푸시·배포처럼 팀이 못 하는 일은 직접 해 주세요</small>
    <div className="gc-plan-actions">
      <button type="button" className="primary" disabled={working || Boolean(problem)} onClick={() => onDecide(decisionsOf(request, current), false)}>결정대로 다시 확인</button>
      <button type="button" className="secondary" disabled={working} onClick={() => onDecide([], true)}>멈추기</button>
    </div>
    <small className="muted">{approval.status === 'pending' ? '멈추면 목표가 멈추고, 해결 요청으로 다음 계획 버전에서 기준을 다시 세워요.' : ''}</small>
  </section>;
}
