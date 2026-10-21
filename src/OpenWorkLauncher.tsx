import { useEffect, useRef, useState } from 'react';
import { ListDisclosure, useLimitedItems } from './ContentDensity';
import { goalStageLabel, taskStageLabel } from './goalBrief';
import { shortGoal, goalTitle } from './progress';
import type { Agent } from './types';
import type { OpenWork } from './openWork';

function WorkGroup({ item, agents, onSelect }: { item: OpenWork; agents: Agent[]; onSelect: (goalId: string, taskId?: string) => void }) {
  const tasks = useLimitedItems(item.tasks);
  return <article className="open-work-group">
    <button type="button" className="open-work-goal" onClick={() => onSelect(item.goal.id)}>
      <span><strong>{goalTitle(item.goal, 100)}</strong><small>{item.projectName} · {item.meetingLabel ? `다관점 회의 · ${item.meetingLabel}` : goalStageLabel(item.goal.status)}{item.needsDecision ? ' · 확인 필요' : ''}</small></span><span aria-hidden="true">→</span>
    </button>
    {tasks.visible.map(task => <button type="button" className="open-work-task" key={task.id} onClick={() => onSelect(item.goal.id, task.id)}><span><strong>{shortGoal(task.title, 90)}</strong><small>{agents.find(agent => agent.id === task.assigneeId)?.name ?? '담당 팀원 확인 필요'} · {taskStageLabel(task.status)}</small></span><span aria-hidden="true">대화 →</span></button>)}
    <ListDisclosure label="업무" total={tasks.total} hidden={tasks.hidden} expanded={tasks.expanded} onToggle={tasks.toggle} />
  </article>;
}

const PLACE_KEY = 'ucompany.openWorkPlace';
type Place = { right: number; bottom: number };
// Above the conversation composer's send button, which the old corner position covered.
const DEFAULT_PLACE: Place = { right: 24, bottom: 96 };

/** Keep the launcher fully on screen; the place is measured from the bottom-right corner like the default. */
export function clampLauncher(place: Place, viewW: number, viewH: number, size = { w: 150, h: 46 }): Place {
  return { right: Math.round(Math.max(8, Math.min(place.right, viewW - size.w - 8))), bottom: Math.round(Math.max(8, Math.min(place.bottom, viewH - size.h - 8))) };
}
function loadPlace(): Place {
  try { const saved = JSON.parse(window.localStorage.getItem(PLACE_KEY) ?? 'null'); if (saved && Number.isFinite(saved.right) && Number.isFinite(saved.bottom)) return saved; } catch { /* default */ }
  return DEFAULT_PLACE;
}

export function OpenWorkLauncher({ items, agents, onSelect }: { items: OpenWork[]; agents: Agent[]; onSelect: (goalId: string, taskId?: string) => void }) {
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLElement>(null);
  const list = useLimitedItems(items);
  // Drag and drop: a press that moves more than a few pixels repositions the button instead of opening the panel.
  const [place, setPlace] = useState<Place>(() => clampLauncher(loadPlace(), window.innerWidth, window.innerHeight));
  const drag = useRef<{ x: number; y: number; right: number; bottom: number; moved: boolean } | null>(null);
  const moved = useRef(false);
  useEffect(() => { try { window.localStorage.setItem(PLACE_KEY, JSON.stringify(place)); } catch { /* best effort */ } }, [place]);
  useEffect(() => {
    const onResize = () => setPlace(current => clampLauncher(current, window.innerWidth, window.innerHeight));
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);
  function pressDown(event: React.PointerEvent<HTMLButtonElement>) {
    if (event.button !== 0) return;
    drag.current = { x: event.clientX, y: event.clientY, right: place.right, bottom: place.bottom, moved: false };
    moved.current = false;
    event.currentTarget.setPointerCapture(event.pointerId);
  }
  function pressMove(event: React.PointerEvent<HTMLButtonElement>) {
    const start = drag.current;
    if (!start) return;
    const dx = event.clientX - start.x, dy = event.clientY - start.y;
    if (!start.moved && Math.hypot(dx, dy) < 5) return;
    start.moved = true;
    const size = event.currentTarget.getBoundingClientRect();
    const next = clampLauncher({ right: start.right - dx, bottom: start.bottom - dy }, window.innerWidth, window.innerHeight, { w: size.width, h: size.height });
    setPlace(next);
  }
  function pressEnd() {
    moved.current = Boolean(drag.current?.moved);
    drag.current = null;
  }
  const leftHalf = place.right > window.innerWidth / 2;
  const topHalf = place.bottom > window.innerHeight / 2;
  useEffect(() => {
    if (!open) return;
    panel.current?.querySelector('button')?.focus();
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { setOpen(false); trigger.current?.focus(); }
    };
    window.addEventListener('keydown', closeOnEscape);
    return () => window.removeEventListener('keydown', closeOnEscape);
  }, [open]);
  function select(goalId: string, taskId?: string) {
    setOpen(false);
    onSelect(goalId, taskId);
  }
  return <div className={`open-work-launcher${leftHalf ? ' is-left' : ''}${topHalf ? ' is-top' : ''}`} style={{ right: place.right, bottom: place.bottom }}>
    {open && <section id="open-work-panel" ref={panel} className="open-work-panel" role="dialog" aria-label="끝나지 않은 일"><header><div><span className="card-kicker">OPEN WORK</span><h2>끝나지 않은 일</h2><p>업무를 선택해 담당 팀원과 대화하고 다음 단계를 결정하세요.</p></div><button type="button" className="secondary" onClick={() => { setOpen(false); trigger.current?.focus(); }} aria-label="끝나지 않은 일 닫기">닫기</button></header><div className="open-work-list">{list.visible.map(item => <WorkGroup key={item.goal.id} item={item} agents={agents} onSelect={select} />)}{items.length === 0 && <p className="open-work-empty">현재 열려 있는 목표가 없습니다.</p>}<ListDisclosure label="목표" total={list.total} hidden={list.hidden} expanded={list.expanded} onToggle={list.toggle} /></div></section>}
    <button ref={trigger} type="button" className="open-work-trigger" aria-label={`열린 작업 · 미완료 목표 ${items.length}개`} aria-expanded={open} aria-controls={open ? 'open-work-panel' : undefined} title="누르면 열고, 끌면 옮겨요" onPointerDown={pressDown} onPointerMove={pressMove} onPointerUp={pressEnd} onPointerCancel={pressEnd} onClick={() => { if (moved.current) { moved.current = false; return; } setOpen(current => !current); }} onKeyDown={event => { const step = event.shiftKey ? 40 : 10; const delta = { ArrowLeft: [step, 0], ArrowRight: [-step, 0], ArrowUp: [0, step], ArrowDown: [0, -step] }[event.key as 'ArrowLeft']; if (event.altKey && delta) { event.preventDefault(); setPlace(current => clampLauncher({ right: current.right + delta[0], bottom: current.bottom + delta[1] }, window.innerWidth, window.innerHeight)); } }}><span aria-hidden="true">✦</span><span>열린 작업</span><strong>{items.length}</strong></button>
  </div>;
}
