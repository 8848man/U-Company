import { useState } from 'react';
import type { Agent, AiConnection, Squad, SquadSlot, Task } from './types';
import { memberLabel, openSlots, slotLabels, structureLabel, swapCandidates } from './squad';

/** A goal's squad. On the pending approval card (`editable`) the CEO may swap writers and
 * reviewers; leads are fixed because they already analysed and planned (ADR-0041 S2/S3). */
export function SquadPanel({ squad, agents, connections, tasks = [], editable, onSwap, onAdd }: { squad: Squad; agents: Agent[]; connections: AiConnection[]; tasks?: Task[]; editable?: boolean; onSwap?: (from: string, to: string) => void; onAdd?: (slot: 'writer' | 'reviewer', agentId: string, taskId: string | null) => void }) {
  const canEdit = Boolean(editable && onSwap && squad.status === 'proposed');
  const [taskId, setTaskId] = useState('');
  const open = canEdit && onAdd ? openSlots(squad, tasks) : { writer: 0, reviewer: 0, movable: [] as Task[] };
  return <section className="squad-panel" aria-label="이 목표의 스쿼드">
    <div className="squad-head"><strong>스쿼드 · {structureLabel(squad.structure)}</strong><span className={`status ${squad.status === 'locked' ? 'completed' : squad.status === 'released' ? '' : 'awaiting_approval'}`}>{{ draft: '초안', proposed: '승인 대기', locked: '승인됨', released: '종료 · 기록' }[squad.status] ?? squad.status}</span></div>
    {squad.structureReason && <small className="squad-reason">계획자 제안 이유: {squad.structureReason}</small>}
    <ul>{(['lead', 'writer', 'reviewer'] as SquadSlot[]).flatMap(slot => squad.members.filter(member => member.slot === slot).map(member => {
      const candidates = canEdit ? swapCandidates(squad, slot, agents, connections) : [];
      return <li key={`${slot}:${member.agentId}`} className="squad-member">
        <span className="squad-slot">{slotLabels[slot]}</span>
        <span className="squad-name">{memberLabel(member, agents)}</span>
        {member.reason && <small>{member.reason}</small>}
        {canEdit && slot !== 'lead' && <label className="squad-swap">교체
          <select value="" onChange={event => { if (event.target.value) onSwap!(member.agentId, event.target.value); }} disabled={!candidates.length}>
            <option value="">{candidates.length ? '다른 팀원 선택' : '바꿀 수 있는 팀원 없음'}</option>
            {candidates.map(agent => <option key={agent.id} value={agent.id}>{agent.name}{agent.external && agent.external.status !== 'hired' ? ' (외부 인력)' : ''}</option>)}
          </select>
        </label>}
      </li>;
    }))}</ul>
    {squad.shortfall.length > 0 && <small className="squad-shortfall">부족: {squad.shortfall.join(' · ')}</small>}
    {open.writer > 0 && open.movable.length > 0 && <div className="squad-fill"><span>빈 작성자 자리 채우기</span>
      <select aria-label="새 작성자에게 넘길 업무" value={taskId} onChange={event => setTaskId(event.target.value)}><option value="">넘길 업무 선택</option>{open.movable.map(task => <option key={task.id} value={task.id}>{task.deliverable || task.title} ({agents.find(agent => agent.id === task.assigneeId)?.name ?? task.assigneeId})</option>)}</select>
      <select aria-label="새 작성자" value="" disabled={!taskId} onChange={event => { if (event.target.value) { onAdd!('writer', event.target.value, taskId); setTaskId(''); } }}><option value="">맡을 팀원 선택</option>{swapCandidates(squad, 'writer', agents, connections).map(agent => <option key={agent.id} value={agent.id}>{agent.name}{agent.external && agent.external.status !== 'hired' ? ' (외부 인력)' : ''}</option>)}</select>
    </div>}
    {open.reviewer > 0 && <div className="squad-fill"><span>빈 검토자 자리 채우기</span>
      <select aria-label="새 검토자" value="" onChange={event => { if (event.target.value) onAdd!('reviewer', event.target.value, null); }}><option value="">검토자 선택</option>{swapCandidates(squad, 'reviewer', agents, connections).map(agent => <option key={agent.id} value={agent.id}>{agent.name}{agent.external && agent.external.status !== 'hired' ? ' (외부 인력)' : ''}</option>)}</select>
    </div>}
    {squad.changes.length > 0 && <small>CEO 변경 기록: {squad.changes.map(change => { const to = agents.find(agent => agent.id === change.to)?.name ?? change.to; return change.from ? `${agents.find(agent => agent.id === change.from)?.name ?? change.from} → ${to}` : `추가 ${to}`; }).join(', ')}</small>}
    {canEdit && <small className="squad-hint">승인하면 이 구조와 팀원으로 확정됩니다. 외부 인력은 이 목표 동안만 계약됩니다.</small>}
  </section>;
}
