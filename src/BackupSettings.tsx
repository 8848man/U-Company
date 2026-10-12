import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { ArchivedGoal } from './types';
import { formatBytes } from './archive';
import { ListDisclosure, useLimitedItems } from './ContentDensity';

type Props = { archived: ArchivedGoal[]; busy: boolean; onRestore: (goalId: string) => Promise<boolean>; onPurge: (goalId: string) => Promise<boolean> };

/** 설정 › 백업 삭제 (ADR-0043): one compact card; the archived goals open in a dialog that shows only
 * a summary of each, with restore and permanent deletion. */
export function BackupSettings({ archived, busy, onRestore, onPurge }: Props) {
  const [open, setOpen] = useState(false);
  return <section className="card v2-settings-narrow backup-settings">
    <div className="card-kicker">BACKUP</div>
    <h2>백업 삭제</h2>
    <p>보관한 목표와 회의는 화면에서 숨겨져 있지만 이 PC에 기록이 남아 있어요. 공간이 필요하면 여기서 영구 삭제할 수 있어요.</p>
    <div className="v2-row"><strong>보관한 항목 {archived.length}개</strong><button type="button" className="primary" disabled={archived.length === 0} onClick={() => setOpen(true)}>{archived.length ? '보관함 열기' : '보관한 항목 없음'}</button></div>
    {open && <BackupDialog archived={archived} busy={busy} onRestore={onRestore} onPurge={onPurge} onClose={() => setOpen(false)} />}
  </section>;
}

function BackupDialog({ archived, busy, onRestore, onPurge, onClose }: Props & { onClose: () => void }) {
  const [sizes, setSizes] = useState<Record<string, number>>({});
  const [sizeError, setSizeError] = useState('');
  const [confirming, setConfirming] = useState<string | null>(null);
  const [checked, setChecked] = useState(false);
  const rows = useLimitedItems(archived);
  useEffect(() => {
    let live = true;
    invoke<{ goalId: string; bytes: number }[]>('backup_usage').then(items => { if (live) setSizes(Object.fromEntries(items.map(item => [item.goalId, item.bytes]))); }).catch(error => { if (live) setSizeError(String(error)); });
    return () => { live = false; };
  }, [archived.length]);
  useEffect(() => { if (archived.length === 0) onClose(); }, [archived.length, onClose]);
  const total = Object.values(sizes).reduce((sum, value) => sum + value, 0);
  return <div className="dialog-backdrop" onClick={onClose}><section className="reason-dialog backup-dialog" role="dialog" aria-modal="true" aria-labelledby="backup-title" onClick={event => event.stopPropagation()}>
    <div className="v2-row"><h2 id="backup-title">보관함</h2><button type="button" className="secondary" onClick={onClose}>닫기</button></div>
    <p>{archived.length}개 · {sizeError ? '크기를 계산하지 못했어요' : Object.keys(sizes).length ? `약 ${formatBytes(total)}` : '크기 계산 중…'}</p>
    <ul className="backup-list">{rows.visible.map(item => <li key={item.goalId}>
      <div className="backup-row-main"><strong>{item.title}</strong><small>{item.kind === 'deliberation' ? '회의' : '목표'} · 보관 {new Date(item.archivedAt).toLocaleDateString()} · {sizes[item.goalId] !== undefined ? formatBytes(sizes[item.goalId]) : '…'}</small></div>
      {confirming === item.goalId
        ? <div className="backup-confirm"><label><input type="checkbox" checked={checked} onChange={event => setChecked(event.target.checked)} /> 되돌릴 수 없다는 것을 확인했어요</label><div className="button-row"><button type="button" className="secondary" onClick={() => { setConfirming(null); setChecked(false); }}>취소</button><button type="button" className="danger" disabled={!checked || busy} onClick={async () => { if (await onPurge(item.goalId)) { setConfirming(null); setChecked(false); } }}>영구 삭제</button></div></div>
        : <div className="button-row"><button type="button" className="secondary" disabled={busy} onClick={() => void onRestore(item.goalId)}>되돌리기</button><button type="button" className="secondary danger-text" disabled={busy} onClick={() => { setConfirming(item.goalId); setChecked(false); }}>영구 삭제…</button></div>}
    </li>)}</ul>
    <ListDisclosure label="보관한 항목" total={rows.total} hidden={rows.hidden} expanded={rows.expanded} onToggle={rows.toggle} />
    <small>영구 삭제하면 그 목표의 업무·대화·회의 기록·작업 폴더가 지워져요. 토큰 사용량 숫자는 프로젝트 합계를 위해 남아요.</small>
  </section></div>;
}
