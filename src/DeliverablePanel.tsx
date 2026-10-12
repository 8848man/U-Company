import type { DeliverableRow, BlockFacts } from './deliverables';

const capabilityLabel: Record<string, string> = { document: '문서', code: '코드' };

/** Deliverables, assignees and acceptance evidence of one goal; used by the approval dialog and
 * the goal workspace so both show the same saved plan (progress-visibility policy). */
export function DeliverablePanel({ rows, mode }: { rows: DeliverableRow[]; mode: 'plan' | 'result' }) {
  if (!rows.length) return null;
  return <section className="deliverable-panel" aria-label={mode === 'plan' ? '산출물 계획' : '산출물과 QA 근거'}>
    <strong>{mode === 'plan' ? `산출물 ${rows.length}개 · 담당과 수용 기준` : `산출물 ${rows.length}개 · QA 판정`}</strong>
    <ol>{rows.map(row => <li key={row.task.id} className="deliverable-row">
      <div className="deliverable-head"><span>{row.title}</span>{row.review && <span className={`status ${row.review.verdict === 'pass' ? 'completed' : 'blocked'}`}>{row.review.verdict === 'pass' ? 'QA 통과' : 'QA 실패'}{row.review.round > 0 ? ` · 재작업 ${row.review.round}회 후` : ''}</span>}</div>
      <small>담당 {row.owner}{row.task.capability ? ` · ${capabilityLabel[row.task.capability] ?? row.task.capability} 작업` : ''} · 파일 {row.task.files.join(', ')}</small>
      {row.reason && <small className="deliverable-reason">배정 이유: {row.reason}</small>}
      <small>근거 자료: {row.task.evidence?.length ? row.task.evidence.join(', ') : '없음 (근거 없는 내용은 미확인으로 남겨야 함)'}</small>
      <ul className="deliverable-criteria">{(row.task.acceptance ?? []).map((criterion, index) => { const result = row.review?.criteria[index]; return <li key={index} className={result ? (result.met && result.evidence ? 'met' : 'unmet') : ''}>
        <span>{result ? (result.met && result.evidence ? '✓ ' : '✗ ') : '· '}{criterion}</span>{result?.evidence && <small> — 근거: {result.evidence}</small>}
      </li>; })}</ul>
      {row.review && row.review.unverified.length > 0 && <small className="deliverable-unverified">미확인: {row.review.unverified.join(' · ')}</small>}
    </li>)}</ol>
  </section>;
}

/** Reason → last confirmed fact → owner → next action, in that order (GC-04). */
export function BlockFactsView({ facts, quiet }: { facts: BlockFacts; quiet?: number | null }) {
  return <dl className="block-facts">
    <div><dt>이유</dt><dd>{facts.reason}<small> · {facts.kindLabel}</small></dd></div>
    <div><dt>마지막 확인</dt><dd>{facts.lastFact}</dd></div>
    <div><dt>담당</dt><dd>{facts.owner}</dd></div>
    <div><dt>다음 행동</dt><dd>{facts.next}</dd></div>
    {quiet ? <div><dt>진단</dt><dd>마지막 기록 후 {quiet}분 · 상태를 바꾸지 않는 참고 정보입니다</dd></div> : null}
  </dl>;
}
