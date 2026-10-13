import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ListDisclosure, useLimitedItems } from './ContentDensity';
import { ORIGIN_CONTROL_LINE, PLANNING_CONTROL_LINE, answerLabel, changePreviews, confirmBody, confirmTitle, originChoices, originMessage, planRows, planSummary, planningLead, type GoalOrigin, type OriginAction, type OriginPlan, type OriginStatus } from './sddOrigin';
import type { Project } from './types';

type Props = {
  project: Project;
  blocked: boolean;
  onError: (message: string) => void;
  /** ADR-0045: set on the plan approval card; the answer is recorded for this goal only. */
  goalOrigin?: GoalOrigin;
};

/**
 * What rules this project follows, and the consent-gated structure actions (SI-03).
 * Settings: first card of 설정 › 프로젝트 명세. Planning: the structure section of a development goal's plan approval card.
 */
export default function SddOriginCard({ project, blocked, onError, goalOrigin }: Props) {
  const planning = Boolean(goalOrigin);
  const [status, setStatus] = useState<OriginStatus | null>(null);
  const [unavailable, setUnavailable] = useState<string | null>(null);
  const [plan, setPlan] = useState<OriginPlan | null>(null);
  const [working, setWorking] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const rows = useLimitedItems(plan ? planRows(plan) : []);
  const title = planning ? '작업 구조' : '이 프로젝트의 작업 규칙';

  async function load() {
    try {
      setStatus(await invoke<OriginStatus>('get_sdd_origin_status', { projectId: project.id }));
      setUnavailable(null);
    } catch (error) {
      const text = String(error);
      setStatus(null);
      setUnavailable(/not found|unknown command/i.test(text) ? '앱을 최신 버전으로 다시 시작하면 이 프로젝트의 작업 규칙을 확인할 수 있어요.' : `작업 규칙을 확인하지 못했어요: ${text}`);
    }
  }

  useEffect(() => { setPlan(null); setDone(null); void load(); }, [project.id]);

  async function preview(action: OriginAction) {
    if (blocked) { onError('진행 중인 작업이 끝나거나 앱을 다시 시작한 뒤 시도해 주세요.'); return; }
    setWorking(true);
    try { setPlan(await invoke<OriginPlan>('plan_sdd_origin', { projectId: project.id, action })); setDone(null); }
    catch (error) { onError(String(error)); await load(); }
    finally { setWorking(false); }
  }

  async function apply() {
    if (!plan) return;
    setWorking(true);
    try {
      setStatus(await invoke<OriginStatus>('apply_sdd_origin', { projectId: project.id, action: plan.action, digest: plan.digest, confirmed: true, goalId: goalOrigin?.goalId ?? null }));
      setDone(`완료했어요: ${planSummary(plan)}`);
      setPlan(null);
    } catch (error) { onError(String(error)); setPlan(null); await load(); }
    finally { setWorking(false); }
  }

  async function answer(decision: 'declined' | 'muted') {
    if (!goalOrigin) return;
    setWorking(true);
    try { await invoke('decide_goal_origin', { goalId: goalOrigin.goalId, decision }); }
    catch (error) { onError(String(error)); }
    finally { setWorking(false); }
  }

  async function unmute() {
    setWorking(true);
    try { await invoke('set_sdd_origin_mute', { projectId: project.id, muted: false }); await load(); }
    catch (error) { onError(String(error)); }
    finally { setWorking(false); }
  }

  const answered = answerLabel(goalOrigin?.decision);
  if (planning && answered) return <div className="spec-preview sdd-origin"><h3>{title}</h3><p>{answered}</p></div>;
  if (unavailable) return <div className="spec-preview sdd-origin"><h3>{title}</h3><p className="muted">{unavailable}</p></div>;
  if (!status) return <div className="spec-preview sdd-origin"><h3>{title}</h3><p className="muted">확인하는 중이에요…</p></div>;

  const message = originMessage(status);
  const choices = originChoices(status);
  const previews = plan ? changePreviews(plan) : [];
  return <div className="spec-preview sdd-origin">
    <h3>{title}</h3>
    {planning && goalOrigin && <p>{planningLead(goalOrigin)}</p>}
    <p><strong>{message.headline}</strong></p>
    {message.detail && <p>{message.detail}</p>}
    {!planning && message.notes.map(note => <p key={note} className="muted">{note}</p>)}
    {planning && goalOrigin?.advice && <p className="muted">CTO 의견: {goalOrigin.advice}</p>}
    {!planning && status.muted && <p className="muted">이 폴더는 계획 단계에서 구조를 묻지 않아요. <button type="button" className="text-button" disabled={working} onClick={() => void unmute()}>다시 묻기</button></p>}
    {done && <p role="status">{done}</p>}
    <small className="muted">{planning ? PLANNING_CONTROL_LINE : ORIGIN_CONTROL_LINE}</small>
    <div className="button-row">
      {/* On the plan approval card the plan's approval stays the only primary button; the recommended choice is named instead. */}
      {choices.map(choice => <button key={choice.action} type="button" className={choice.primary && !planning ? 'primary' : 'secondary'} disabled={working} onClick={() => void preview(choice.action)}>{choice.label}{planning && choice.primary ? ' (권장)' : ''}</button>)}
      {planning && <button type="button" className="secondary" disabled={working} onClick={() => void answer('declined')}>이번엔 건너뛰기</button>}
      {planning && <button type="button" className="text-button" disabled={working} onClick={() => void answer('muted')}>이 폴더는 묻지 않기</button>}
      {!planning && <button type="button" className="text-button" disabled={working} onClick={() => void load()}>다시 확인</button>}
    </div>
    {plan && <div className="dialog-backdrop" onClick={() => setPlan(null)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="sdd-origin-title" onClick={event => event.stopPropagation()}>
      <div className="dialog-mark">?</div>
      <h2 id="sdd-origin-title">{confirmTitle(plan.action)}</h2>
      <p>{project.name} 폴더 · {planSummary(plan)}</p>
      <p>{confirmBody(plan.action)}</p>
      <ul className="sdd-origin-files">{rows.visible.map(row => <li key={`${row.label}:${row.path}`}><span>{row.label}</span> <code>{row.path}</code></li>)}</ul>
      <ListDisclosure label="파일" total={rows.total} hidden={rows.hidden} expanded={rows.expanded} onToggle={rows.toggle} />
      {previews.map(item => <details key={item.path} className="sdd-origin-diff">
        <summary>{item.path} 바뀌는 내용 보기 ({item.label})</summary>
        <div className="sdd-origin-diff-grid">
          <div><b>지금</b><pre>{item.before || '(비어 있음)'}</pre></div>
          <div><b>바뀐 뒤</b><pre>{item.after}</pre></div>
        </div>
      </details>)}
      <p className="muted">미리본 뒤 파일이 바뀌었으면 쓰지 않고 다시 미리보라고 알려 드려요.</p>
      <div className="dialog-actions"><button type="button" className="secondary" onClick={() => setPlan(null)}>취소</button><button type="button" className="primary" autoFocus disabled={working} onClick={() => void apply()}>{choices.find(choice => choice.action === plan.action)?.label ?? '확인'}</button></div>
    </section></div>}
  </div>;
}
