import { useState } from 'react';
import { ALLOW_ALL_RISKS } from './commandMode';
import './commandMode.css';

type Props = { allowAll: boolean; working: boolean; onChange: (allowAll: boolean, confirmed: boolean) => Promise<boolean> };

/** ADR-0063: the always-visible control line while allow-all is on (UXP-3). */
export function CommandModeBanner({ allowAll, working, onChange }: Props) {
  if (!allowAll) return null;
  return <div className="gc-allow-all-banner" role="status">
    <span className="gc-allow-all-dot" aria-hidden="true" />
    <strong>명령 전부 허용 중</strong>
    <span>팀원이 묻지 않고 명령을 실행해요 · push·PR·배포는 막혀요</span>
    <button type="button" className="secondary" disabled={working} onClick={() => void onChange(false, false)}>끄기</button>
  </div>;
}

/** ADR-0063: the switch on a pending plan card. Turning it on always goes through the confirmation dialog. */
export function CommandModeToggle({ allowAll, working, onChange }: Props) {
  const [confirming, setConfirming] = useState(false);
  if (allowAll) return <p className="gc-allow-all-plan is-on"><strong>명령 전부 허용 켜짐</strong> · 이 대화의 팀원은 명령 카드 없이 실행해요 <button type="button" className="text-button" disabled={working} onClick={() => void onChange(false, false)}>끄기</button></p>;
  return <>
    <p className="gc-allow-all-plan"><span>명령마다 묻지 않고 진행하려면</span> <button type="button" className="text-button" disabled={working} onClick={() => setConfirming(true)}>명령 전부 허용 켜기…</button></p>
    {confirming && <div className="dialog-backdrop" onClick={() => setConfirming(false)} onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); setConfirming(false); } }}>
      <section className="reason-dialog gc-allow-all-dialog" role="dialog" aria-modal="true" aria-labelledby="allow-all-title" onClick={event => event.stopPropagation()}>
        <h2 id="allow-all-title">명령 전부 허용을 켤까요?</h2>
        <ul>{ALLOW_ALL_RISKS.map(line => <li key={line}>{line}</li>)}</ul>
        <div className="button-row">
          <button type="button" className="danger" disabled={working} onClick={() => void onChange(true, true).then(done => { if (done) setConfirming(false); })}>위험을 알고 켜기</button>
          <button type="button" className="secondary" autoFocus onClick={() => setConfirming(false)}>취소</button>
        </div>
      </section>
    </div>}
  </>;
}
