import { useState } from 'react';
import { chipLabel, curationLabel } from './referenceAttach';
import type { Reference, Task } from './types';

/** ADR-0049: the goal's reference material and which tasks use it (the index the team reads). */
export function ReferenceButton({ references, tasks }: { references: Reference[]; tasks: Task[] }) {
  const [open, setOpen] = useState(false);
  if (!references.length) return null;
  const usedBy = (reference: Reference) => tasks.filter(task => (task.evidence ?? []).some(path => path.toLowerCase() === reference.path.toLowerCase() || reference.sections.some(section => section.file.toLowerCase() === path.toLowerCase())));
  return <>
    <button type="button" className="secondary" onClick={() => setOpen(true)} title="이 목표에 첨부한 참조 자료">참조 자료 {references.length}</button>
    {open && <div className="dialog-backdrop" onClick={() => setOpen(false)} onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); setOpen(false); } }}>
      <section className="reason-dialog reference-dialog" role="dialog" aria-modal="true" aria-label="참조 자료" onClick={event => event.stopPropagation()}>
        <button type="button" className="text-button gc-todo-close" autoFocus onClick={() => setOpen(false)}>닫기</button>
        <span className="card-kicker">참조 자료</span>
        <h3>팀은 이 목록을 보고 필요한 자료만 열어요</h3>
        <ul className="reference-index">{references.map(reference => <li key={reference.id} className={reference.status === 'missing' ? 'is-missing' : undefined}>
          <strong>{chipLabel(reference)}</strong>
          <small>{reference.status === 'missing' ? '파일 없음 · 이 자료를 지정한 계획은 멈춰요' : curationLabel(reference)} · <code>{reference.path}</code></small>
          {(reference.caption || reference.summary) && <p>{reference.caption || reference.summary}</p>}
          {reference.sections.length > 0 && <ol>{reference.sections.map(section => <li key={section.n}>§{section.n} {section.title} <small>{section.chars.toLocaleString('ko-KR')}자</small></li>)}</ol>}
          {usedBy(reference).length > 0 && <small>쓰는 업무: {usedBy(reference).map(task => task.title).join(', ')}</small>}
        </li>)}</ul>
      </section>
    </div>}
  </>;
}
