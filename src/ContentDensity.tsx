import { useEffect, useRef, useState, type ReactNode } from 'react';

export const DEFAULT_VISIBLE_ITEMS = 5;
export const CONTENT_PREVIEW_HEIGHT = 160;

export function limitItems<T>(items: T[], expanded: boolean, count = DEFAULT_VISIBLE_ITEMS): T[] {
  return expanded ? items : items.slice(0, count);
}

export function useLimitedItems<T>(items: T[], count = DEFAULT_VISIBLE_ITEMS) {
  const [expanded, setExpanded] = useState(false);
  return { visible: limitItems(items, expanded, count), expanded, hidden: Math.max(0, items.length - count), total: items.length, toggle: () => setExpanded(value => !value) };
}

export function ListDisclosure({ label, total, hidden, expanded, onToggle }: { label: string; total: number; hidden: number; expanded: boolean; onToggle: () => void }) {
  if (!hidden) return null;
  return <div className="list-disclosure"><span>{expanded ? `${label} 전체 ${total}개 표시` : `${label} ${total}개 중 ${total - hidden}개 표시 · ${hidden}개 더 있음`}</span><button type="button" className="secondary" aria-expanded={expanded} onClick={onToggle}>{expanded ? '접기 ↑' : `모두 보기 (${total}개) ↓`}</button></div>;
}

export function ContentPreview({ children, className = '', maxHeight = CONTENT_PREVIEW_HEIGHT }: { children: ReactNode; className?: string; maxHeight?: number }) {
  const inner = useRef<HTMLDivElement>(null);
  const [expanded, setExpanded] = useState(false);
  const [tall, setTall] = useState(false);
  useEffect(() => {
    const node = inner.current;
    if (!node) return;
    const measure = () => setTall(node.scrollHeight > maxHeight + 1);
    measure();
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', measure);
      return () => window.removeEventListener('resize', measure);
    }
    const observer = new ResizeObserver(measure);
    observer.observe(node);
    return () => observer.disconnect();
  }, [children, maxHeight]);
  return <div className={`content-preview ${className}`}><div className="content-preview-viewport" style={!expanded ? { maxHeight } : undefined}><div ref={inner}>{children}</div></div>{tall && <button type="button" className="text-button content-preview-toggle" aria-expanded={expanded} onClick={() => setExpanded(value => !value)}>{expanded ? '접기 ↑' : '전체 보기 ↓'}</button>}</div>;
}
