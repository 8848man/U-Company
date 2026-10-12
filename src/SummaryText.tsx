import { useState } from 'react';
import type { Headline } from './plainText';

/** Summary first, original one action away (UXP-2). Labels AI summaries and excerpts; the original
 * text is shown unchanged on request. */
export function SummaryText({ value, as = 'span', rawLabel = '원문 보기' }: { value: Headline; as?: 'span' | 'h2' | 'h3' | 'strong' | 'p'; rawLabel?: string }) {
  const [open, setOpen] = useState(false);
  const Tag = as;
  const hasRaw = value.source !== 'full' && Boolean(value.raw.trim()) && value.raw.trim() !== value.text;
  return <span className="summary-text">
    <Tag className="summary-main" title={value.source === 'full' ? undefined : value.raw}>{value.text}</Tag>
    {value.source !== 'full' && <small className={`summary-badge ${value.source}`}>{value.source === 'ai' ? 'AI 요약' : '원문 일부'}</small>}
    {hasRaw && <button type="button" className="text-button summary-toggle" aria-expanded={open} onClick={() => setOpen(current => !current)}>{open ? '원문 접기' : rawLabel}</button>}
    {open && <pre className="summary-raw">{value.raw}</pre>}
  </span>;
}
