import { useState } from 'react';
import { approveAllKeys, groupRules, ruleKey, ruleLabel } from './commandPermission';
import type { Approval, CommandRequest } from './types';

type Props = { approval: Approval; request: CommandRequest | undefined; working: boolean; onDecide: (keys: string[]) => void };

/** ADR-0058: a teammate was refused commands. `모두 승인` grants the safe ones (and ticked risky ones) for this goal conversation only. */
export function CommandPermissionCard({ approval, request, working, onDecide }: Props) {
  const [chosen, setChosen] = useState<string[]>([]);
  const who = approval.reason.split('이(가)')[0] || '팀원';
  if (!request) return <section className="gc-plan is-permission" aria-label="명령 실행 승인"><h3>명령 실행 승인이 필요해요</h3><p className="muted">요청 기록을 불러오는 중이에요.</p></section>;
  const { safe, individual, blocked } = groupRules(request);
  const keys = approveAllKeys(request, chosen);
  const toggle = (key: string) => setChosen(list => list.includes(key) ? list.filter(item => item !== key) : [...list, key]);
  return <section className="gc-plan is-permission" aria-label="명령 실행 승인">
    <h3>명령 실행 승인이 필요해요</h3>
    <p>{who}이(가) 아래 명령을 실행하려다 막혔어요. 승인하면 <b>이 목표 대화에서만</b> 허용하고, 막힌 작업을 같은 자리에서 이어서 해요.</p>
    {safe.length > 0 && <ul className="gc-commands">{safe.map(rule => <li key={ruleKey(rule)}><code>{ruleLabel(rule)}</code></li>)}</ul>}
    {individual.length > 0 && <><p className="gc-commands-head">따로 확인할 명령</p><ul className="gc-commands is-individual">{individual.map(rule => { const key = ruleKey(rule); const on = chosen.includes(key); return <li key={key}><code>{ruleLabel(rule)}</code><small>{rule.reason}</small><button type="button" className={on ? 'primary' : 'secondary'} aria-pressed={on} disabled={working} onClick={() => toggle(key)}>{on ? '함께 승인함' : '이것도 승인'}</button></li>; })}</ul></>}
    {blocked.length > 0 && <><p className="gc-commands-head">앱이 허용하지 않는 요청</p><ul className="gc-commands is-blocked">{blocked.map(rule => <li key={ruleKey(rule)}><code>{ruleLabel(rule)}</code><small>{rule.reason}</small></li>)}</ul></>}
    <small className="muted">승인한 명령은 내 컴퓨터 권한으로 실행돼요(Windows에서는 격리되지 않아요) · 승인은 이 버튼으로만 해요</small>
    <div className="gc-plan-actions">
      <button type="button" className="primary" disabled={working || keys.length === 0} onClick={() => onDecide(keys)}>{individual.length && !safe.length ? '선택한 명령 승인' : '모두 승인'}{keys.length ? ` (${keys.length})` : ''}</button>
      <button type="button" className="secondary" disabled={working} onClick={() => onDecide([])}>승인하지 않음</button>
    </div>
    <small className="muted">승인하지 않으면 지금 결과 그대로 진행하고 QA가 판정해요.</small>
  </section>;
}
