import { useEffect, useRef, useState } from 'react';
import type { PointerEvent } from 'react';
import type { Agent, AiConnection, Event } from '../types';
import { mapOffset } from './projection';
import PixelCharacter from './PixelCharacter';
import { appearanceFor } from './metrocity';
import type { MemberActivity } from '../progress';

type Props = {
  agents: Agent[];
  connections: AiConnection[];
  activities: Record<string, MemberActivity>;
  events: Event[];
  /** Squad slot / external staff label per character, from saved squads (ADR-0041). */
  badges?: Record<string, string>;
  selectedId: string | null;
  /** ADR-0046 Office window: members of the watched goal stay clear, everyone else is dimmed. */
  focusIds?: string[];
  onSelect: (id: string) => void;
  onMove: (id: string, x: number, y: number) => void;
};
type Position = { x: number; y: number };
type Drag = { id: string; startX: number; startY: number; moved: boolean };

const clamp = (value: number) => Math.max(0, Math.min(100, Math.round(value)));

export default function OfficeCanvas({ agents, connections, activities, events, badges, selectedId, focusIds, onSelect, onMove }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const stageRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef<Drag | null>(null);
  const [preview, setPreview] = useState<Record<string, Position>>({});
  // Where the floor map is drawn inside the stage (CSS px). Characters are placed on this map, not on the stage box,
  // so the Office page and the smaller Office window put everyone on the same floor spot.
  const [mapRect, setMapRect] = useState<{ left: number; top: number; width: number; height: number } | null>(null);
  const seenHandoff = useRef(0);
  const handoffTimers = useRef<number[]>([]);
  const [interaction, setInteraction] = useState<{ from: string; to: string; moving: boolean } | null>(null);

  useEffect(() => () => handoffTimers.current.forEach(window.clearTimeout), []);

  useEffect(() => {
    const handoff = events.find(event => event.kind === 'agent_handoff');
    if (!handoff || handoff.id <= seenHandoff.current) return;
    seenHandoff.current = handoff.id;
    if (Date.now() - new Date(handoff.at).getTime() > 8_000 || !handoff.agentId || !agents.some(agent => agent.id === handoff.agentId) || !agents.some(agent => agent.id === handoff.message)) return;
    handoffTimers.current.forEach(window.clearTimeout);
    setInteraction({ from: handoff.agentId, to: handoff.message, moving: false });
    const approach = window.setTimeout(() => setInteraction({ from: handoff.agentId!, to: handoff.message, moving: true }), 40);
    const returnHome = window.setTimeout(() => setInteraction(current => current?.from === handoff.agentId ? { ...current, moving: false } : current), 1700);
    const clear = window.setTimeout(() => setInteraction(current => current?.from === handoff.agentId ? null : current), 2800);
    handoffTimers.current = [approach, returnHome, clear];
  }, [events, agents]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const draw = () => {
      const context = canvas.getContext('2d');
      if (!context) return;
      const bounds = canvas.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.round(bounds.width * dpr);
      canvas.height = Math.round(bounds.height * dpr);
      context.imageSmoothingEnabled = false;
      context.fillStyle = '#14251e';
      context.fillRect(0, 0, canvas.width, canvas.height);
      const cols = 36;
      const rows = 22;
      const tile = 16;
      // Whole-number zoom keeps pixel art sharp; a stage smaller than the map scales down instead of cropping it.
      const fit = Math.min(canvas.width / (cols * tile), canvas.height / (rows * tile));
      const zoom = fit >= 1 ? Math.floor(fit) : fit;
      const { offsetX, offsetY } = mapOffset(canvas.width, canvas.height, cols, rows, zoom, 0, 0);
      const box = (x: number, y: number, width: number, height: number, color: string) => {
        context.fillStyle = color;
        context.fillRect(Math.round(offsetX + x * zoom), Math.round(offsetY + y * zoom), width * zoom, height * zoom);
      };
      for (let row = 0; row < rows; row++) {
        for (let col = 0; col < cols; col++) box(col * tile, row * tile, tile - 1, tile - 1, (col + row) % 2 ? '#284433' : '#2b4836');
      }
      box(0, 0, cols * tile, 8, '#385845');
      box(0, 0, 8, rows * tile, '#385845');
      box(15 * tile, 8 * tile, 6 * tile, 4 * tile, '#3d2c26');
      box(15 * tile + 5, 8 * tile + 5, 6 * tile - 10, 4 * tile - 10, '#805c3c');
      const next = { left: offsetX / dpr, top: offsetY / dpr, width: (cols * tile * zoom) / dpr, height: (rows * tile * zoom) / dpr };
      setMapRect(current => current && Math.abs(current.left - next.left) < 0.5 && Math.abs(current.top - next.top) < 0.5 && Math.abs(current.width - next.width) < 0.5 && Math.abs(current.height - next.height) < 0.5 ? current : next);
    };
    draw();
    const observer = new ResizeObserver(draw);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, []);

  function positionFromPointer(event: PointerEvent<HTMLButtonElement>): Position {
    const stage = stageRef.current!.getBoundingClientRect();
    const map = mapRect ?? { left: 0, top: 0, width: stage.width, height: stage.height };
    return {
      x: clamp((((event.clientX - stage.left - map.left) / map.width) * 100 - 8) / 0.84),
      y: clamp((((event.clientY - stage.top - map.top) / map.height) * 100 - 10) / 0.78),
    };
  }
  /** Saved 0–100 coordinates → a spot on the drawn floor map. */
  function placeOnMap(x: number, y: number) {
    if (!mapRect) return { left: `${8 + x * 0.84}%`, top: `${10 + y * 0.78}%` };
    return { left: `${mapRect.left + ((8 + x * 0.84) / 100) * mapRect.width}px`, top: `${mapRect.top + ((10 + y * 0.78) / 100) * mapRect.height}px` };
  }

  function beginDrag(event: PointerEvent<HTMLButtonElement>, id: string) {
    dragRef.current = { id, startX: event.clientX, startY: event.clientY, moved: false };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function moveDrag(event: PointerEvent<HTMLButtonElement>) {
    const drag = dragRef.current;
    if (!drag) return;
    if (Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY) > 4) drag.moved = true;
    if (drag.moved) setPreview(current => ({ ...current, [drag.id]: positionFromPointer(event) }));
  }

  function endDrag(event: PointerEvent<HTMLButtonElement>) {
    const drag = dragRef.current;
    if (!drag) return;
    dragRef.current = null;
    if (drag.moved) {
      const position = positionFromPointer(event);
      onMove(drag.id, position.x, position.y);
      setPreview(current => { const next = { ...current }; delete next[drag.id]; return next; });
    } else onSelect(drag.id);
  }

  return <div className="office-stage" ref={stageRef}>
    <canvas ref={canvasRef} className="office-canvas" aria-hidden="true" />
    {agents.filter(agent => agent.enabled).map((agent, index) => {
      const activity = activities[agent.id];
      const working = activity?.kind === 'active';
      const blocked = activity?.kind === 'blocked';
      const position = preview[agent.id] ?? agent;
      const recipient = interaction?.from === agent.id ? agents.find(item => item.id === interaction.to) : null;
      const visualX = recipient && interaction?.moving && !preview[agent.id] ? agent.x + (recipient.x - agent.x) * 0.7 : position.x;
      const visualY = recipient && interaction?.moving && !preview[agent.id] ? agent.y + (recipient.y - agent.y) * 0.7 : position.y;
      const state = working ? `작업 중: ${activity.label}` : blocked ? '확인 필요' : connections.find(item => item.id === agent.connectionId)?.name ?? 'AI 미배정';
      return <button
        type="button"
        key={agent.id}
        className={`office-avatar ${selectedId === agent.id ? 'selected' : ''} ${working ? 'working' : ''} ${blocked ? 'blocked' : ''} ${recipient ? 'interacting' : ''} ${focusIds && !focusIds.includes(agent.id) ? 'dimmed' : ''}`}
        style={placeOnMap(visualX, visualY)}
        title={`${agent.name}: 클릭해 활동 보기, 드래그해 위치 변경`}
        aria-label={`${agent.name}, ${state}${badges?.[agent.id] ? `, ${badges[agent.id]}` : ''}. 클릭해 활동을 보거나 드래그해 이동`}
        onPointerDown={event => beginDrag(event, agent.id)}
        onPointerMove={moveDrag}
        onPointerUp={endDrag}
        onPointerCancel={() => { dragRef.current = null; setPreview({}); }}
        onKeyDown={event => {
          if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); onSelect(agent.id); }
          const delta = { ArrowLeft: [-5, 0], ArrowRight: [5, 0], ArrowUp: [0, -5], ArrowDown: [0, 5] }[event.key];
          if (delta) { event.preventDefault(); onMove(agent.id, clamp(agent.x + delta[0]), clamp(agent.y + delta[1])); }
        }}
      >
        {recipient ? <span className="activity-bubble" aria-hidden="true"><span>💬</span><b>↗</b></span> : working && <span className="activity-bubble" aria-hidden="true"><span>💬</span><b>…</b></span>}
        <PixelCharacter index={index} working={working} appearance={appearanceFor(agent)} />
        <span className="avatar-caption"><strong>{agent.name}</strong><small>{state}</small>{badges?.[agent.id] && <span className={`squad-badge ${agent.external && agent.external.status !== 'hired' ? 'external' : ''}`}>{badges[agent.id]}</span>}</span>
      </button>;
    })}
  </div>;
}
