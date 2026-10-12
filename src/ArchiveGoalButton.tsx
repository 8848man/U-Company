import { useState } from 'react';
import type { Goal, Meeting, Task } from './types';
import { archiveBlock } from './archive';

/** "목표 보관": hides the goal from every screen without deleting it (ADR-0043). Two steps so it is
 * never one accidental click; the backup dialog in 설정 restores or permanently deletes it. */
export function ArchiveGoalButton({ goal, tasks, meeting, busy, onArchive }: { goal: Goal; tasks: Task[]; meeting?: Meeting | null; busy?: boolean; onArchive: () => Promise<boolean> }) {
  const [asking, setAsking] = useState(false);
  const blocked = archiveBlock(goal, tasks, meeting);
  const noun = goal.kind === 'deliberation' ? '회의' : '목표';
  if (!asking) return <button type="button" className="secondary archive-goal" title={blocked ?? `${noun}를 화면에서 숨겨요`} onClick={() => setAsking(true)}>{noun} 보관</button>;
  return <div className="archive-confirm" role="alertdialog" aria-label={`${noun} 보관 확인`}>
    {blocked ? <p>{blocked}</p> : <p>이 {noun}를 모든 화면에서 숨겨요. 기록은 지우지 않아요. <b>설정 › 백업 삭제</b>에서 되돌리거나 영구 삭제할 수 있어요.</p>}
    <div className="button-row"><button type="button" className="secondary" onClick={() => setAsking(false)}>취소</button>{!blocked && <button type="button" className="primary" disabled={busy} onClick={async () => { if (await onArchive()) setAsking(false); }}>보관하기</button>}</div>
  </div>;
}
