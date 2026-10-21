import { useEffect, useRef, useState } from 'react';
import OfficeCanvas from './office/OfficeCanvas';
import { OfficeMiniChat } from './OfficeMiniChat';
import { goalTitle } from './progress';
import type { MemberActivity } from './progress';
import { clampPlace, loadPlace, memberRows, savePlace, stageStrip, type WindowPlace } from './officeWindowState';
import type { Agent, AiConnection, Event, Goal, GoalTurn, Squad, Task } from './types';

type Props = {
  goal: Goal | null;
  agents: Agent[]; connections: AiConnection[]; tasks: Task[]; events: Event[];
  squad: Squad | null; activities: Record<string, MemberActivity>; badges: Record<string, string>;
  /** The conversation's progress line, shown as the window's status so both say the same thing. */
  progress: string | null;
  selectedId: string | null; onSelect: (id: string) => void; onMove: (id: string, x: number, y: number) => void;
  /** This goal's conversation turns; the mini chat shows the selected teammate's. */
  turns: GoalTurn[];
  onAsk: (agentId: string, question: string) => Promise<boolean>;
  onClose: () => void;
};

/** In-app Office window (ADR-0046 decision 7): move, resize, dock right, minimize, close; place remembered on this computer. */
export function OfficeWindow({ goal, agents, connections, tasks, events, squad, activities, badges, progress, selectedId, onSelect, onMove, turns, onAsk, onClose }: Props) {
  // Clicking a character (or a member row) opens a short session with that teammate inside the window.
  const [chatId, setChatId] = useState<string | null>(null);
  const openChat = (id: string) => { onSelect(id); setChatId(id); };
  const [place, setPlace] = useState<WindowPlace>(() => clampPlace(loadPlace(), window.innerWidth, window.innerHeight));
  const frameRef = useRef<HTMLElement>(null);
  const dragRef = useRef<{ dx: number; dy: number } | null>(null);
  const rows = goal ? memberRows(goal, agents, tasks, squad, activities) : [];
  const strip = goal ? stageStrip(goal) : [];

  useEffect(() => { savePlace(place); }, [place]);
  // Docked, the window takes the right edge and the app narrows instead of hiding the conversation under it.
  useEffect(() => {
    document.body.classList.toggle('office-docked', place.mode === 'docked');
    return () => document.body.classList.remove('office-docked');
  }, [place.mode]);
  useEffect(() => {
    const onResize = () => setPlace(current => clampPlace(current, window.innerWidth, window.innerHeight));
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);
  // Native CSS resize on the floating frame; remember the size it ends at.
  useEffect(() => {
    const frame = frameRef.current;
    if (!frame || place.mode !== 'floating') return;
    const observer = new ResizeObserver(() => {
      const { width, height } = frame.getBoundingClientRect();
      setPlace(current => Math.abs(current.w - width) < 2 && Math.abs(current.h - height) < 2 ? current : { ...current, w: Math.round(width), h: Math.round(height) });
    });
    observer.observe(frame);
    return () => observer.disconnect();
  }, [place.mode]);

  // Only a primary-button press on the bar moves the window; right clicks and other buttons never start a drag.
  function startDrag(event: React.PointerEvent) {
    if (event.button !== 0 || place.mode !== 'floating' || (event.target as HTMLElement).closest('button')) return;
    dragRef.current = { dx: event.clientX - place.x, dy: event.clientY - place.y };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }
  function moveDrag(event: React.PointerEvent) {
    const drag = dragRef.current;
    if (!drag) return;
    // Compute the target now: the drag may end before React runs a queued updater, so the updater must not read the ref.
    const x = event.clientX - drag.dx, y = event.clientY - drag.dy;
    setPlace(current => clampPlace({ ...current, x, y }, window.innerWidth, window.innerHeight));
  }
  function endDrag() { dragRef.current = null; }
  const mode = (next: WindowPlace['mode']) => setPlace(current => ({ ...current, mode: current.mode === next ? 'floating' : next }));
  const style = place.mode === 'floating' ? { left: place.x, top: place.y, width: place.w, height: place.h } : undefined;
  const title = goal ? `Office · ${goalTitle(goal, 28)}` : 'Office';

  return <section ref={frameRef} className={`office-window is-${place.mode}`} style={style} role="dialog" aria-label={title}>
    <header className="office-window-bar" onPointerDown={startDrag} onPointerMove={moveDrag} onPointerUp={endDrag} onPointerCancel={endDrag} onLostPointerCapture={endDrag} onDoubleClick={() => mode('docked')}>
      <strong title={goal?.intent}>{title}</strong>
      <div className="office-window-controls">
        <button type="button" aria-label={place.mode === 'minimized' ? '창 펼치기' : '최소화'} title={place.mode === 'minimized' ? '펼치기' : '최소화'} onClick={() => mode('minimized')}>{place.mode === 'minimized' ? '▢' : '–'}</button>
        <button type="button" aria-label={place.mode === 'docked' ? '떠 있는 창으로' : '오른쪽에 붙이기'} title={place.mode === 'docked' ? '떼기' : '오른쪽에 붙이기'} onClick={() => mode('docked')}>{place.mode === 'docked' ? '⧉' : '▐'}</button>
        <button type="button" aria-label="Office 창 닫기" title="닫기" onClick={onClose}>×</button>
      </div>
    </header>
    {place.mode !== 'minimized' && <div className="office-window-body">
      {goal && <ol className="office-stages" aria-label="단계">{strip.map(stage => <li key={stage.id} className={`is-${stage.state}`} aria-current={stage.state === 'current' || stage.state === 'waiting' ? 'step' : undefined}>{stage.label}</li>)}</ol>}
      {progress && <p className="office-window-status" role="status">{progress}</p>}
      <div className="office-window-room"><OfficeCanvas agents={agents} connections={connections} activities={activities} events={events} badges={badges} selectedId={selectedId} focusIds={goal ? rows.map(row => row.id) : undefined} onSelect={openChat} onMove={onMove} />
        {chatId && <OfficeMiniChat goal={goal} agent={agents.find(agent => agent.id === chatId)} turns={turns} onAsk={onAsk} onClose={() => setChatId(null)} />}</div>
      {goal && <ul className="office-members" aria-label="이 목표의 팀원">{rows.map(row => <li key={row.id} className={`is-${row.kind}${row.id === chatId ? ' is-chatting' : ''}`}><button type="button" title={`${row.name}과 대화`} onClick={() => openChat(row.id)}><strong>{row.name}</strong><span>{row.work}</span><small>{row.state}</small></button></li>)}{rows.length === 0 && <li className="is-idle"><span>아직 맡은 팀원이 없어요</span></li>}</ul>}
    </div>}
  </section>;
}
