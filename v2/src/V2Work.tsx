import { useState, type ReactNode } from 'react';
import { BoardProgress } from '../../src/OfficeActivity';
import { ContentPreview, ListDisclosure, useLimitedItems } from '../../src/ContentDensity';
import type { Agent, Approval, Event, Goal, Task } from '../../src/types';
import type { MeetingAttention } from '../../src/meetingStatus';
import { headline } from '../../src/plainText';
import { goalTitle } from '../../src/progress';
import { SummaryText } from '../../src/SummaryText';

const plainTaskStatus: Record<string, string> = { pending: '시작 전', running: '하는 중', awaiting_integration: '합치는 중', completed: '끝남', failed: '멈춤', blocked: '멈춤' };

export function V2Work({ goals, tasks, agents, approvals, events, onOpen, onSaveGroup, onApprove, reviewControl, meetingStatus }: { meetingStatus: (goal: Goal) => MeetingAttention; goals: Goal[]; tasks: Task[]; agents: Agent[]; approvals: Approval[]; events: Event[]; onOpen: (id: string) => void; onSaveGroup: (id: string, group: string) => void; onApprove: (id: string) => void; reviewControl: (goal: Goal) => ReactNode }) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const executionGoals = goals.filter(goal => goal.kind !== 'deliberation');
  const selected = executionGoals.find(goal => goal.id === selectedId);
  const ownedTasks = useLimitedItems(tasks.filter(task => task.goalId === selectedId));
  const ownedEvents = useLimitedItems(events.filter(event => event.goalId === selectedId));
  const pending = approvals.filter(item => item.status === 'pending');
  return <div className="page"><div className="v2-heading"><div><div className="eyebrow">TEAM PROGRESS</div><h1>업무</h1><p>목표를 선택하면 그 목표의 담당 업무와 실제 기록을 볼 수 있습니다.</p></div></div>
    {pending.length > 0 && (() => { const goal = goals.find(item => item.id === pending[0].goalId); return <section className="card v2-summary"><div className="v2-row"><div><strong>결정이 필요한 일 {pending.length}건</strong><small>{goal ? `“${goalTitle(goal, 60)}” — ` : ''}팀이 파일을 바꾸기 전에 확인이 필요해요.</small><details className="summary-raw-details"><summary>요청 원문 보기</summary><pre className="summary-raw">{pending[0].reason}</pre></details></div><button className="primary" onClick={() => onApprove(pending[0].id)}>확인 열기</button></div></section>; })()}
    <BoardProgress goals={goals.filter(goal => goal.kind !== 'deliberation')} tasks={tasks} agents={agents} approvals={approvals} onOpenGoal={id => { setSelectedId(id); onOpen(id); }} onSaveGroup={onSaveGroup}/>
    {goals.filter(goal => goal.kind === 'deliberation').map(goal => { const status = meetingStatus(goal); return <section className="card" key={goal.id}><div className="v2-row"><span className={`status ${goal.status}`}>다관점 회의 · {status.label}</span></div><h2><SummaryText value={headline(goal.intent, goal.summary, 80)} /></h2><p>{status.detail} · 다음: {status.next}</p><button className={status.needsDecision ? 'primary' : 'secondary'} onClick={() => onOpen(goal.id)}>{status.needsDecision ? '결정 열기' : '회의 열기'}</button></section>; })}
    {executionGoals.length > 0 && <div className="v2-board-detail"><label>진행 정보 볼 목표 <select value={selectedId ?? ''} onChange={event => setSelectedId(event.target.value || null)}><option value="">목표 선택</option>{executionGoals.map(goal => <option key={goal.id} value={goal.id}>{goalTitle(goal, 60)}</option>)}</select></label>{selected && <section className="card"><h2><SummaryText value={headline(selected.intent, selected.summary, 80)} /></h2><p>담당 업무 {ownedTasks.total}개 · 기록 {ownedEvents.total}건</p>{reviewControl(selected)}<details><summary>담당 업무 {ownedTasks.total}개</summary><div className="v2-stack">{ownedTasks.visible.map(task => <article key={task.id} className="v2-summary"><strong>{task.deliverable || task.title}</strong><small>{agents.find(agent => agent.id === task.assigneeId)?.name ?? task.assigneeId} · {plainTaskStatus[task.status] ?? task.status}</small><details><summary>AI에게 준 지시 보기</summary><ContentPreview><p>{task.instructions}</p></ContentPreview></details>{task.result && <details><summary>결과 원문 보기</summary><ContentPreview><p>{task.result}</p></ContentPreview></details>}</article>)}</div><ListDisclosure label="업무" total={ownedTasks.total} hidden={ownedTasks.hidden} expanded={ownedTasks.expanded} onToggle={ownedTasks.toggle}/></details><details><summary>실제 활동 기록 {ownedEvents.total}건</summary>{ownedEvents.visible.map(event => <p key={event.id}><small>{new Date(event.at).toLocaleString()}</small> · {event.message}</p>)}<ListDisclosure label="활동 기록" total={ownedEvents.total} hidden={ownedEvents.hidden} expanded={ownedEvents.expanded} onToggle={ownedEvents.toggle}/></details><button className="secondary" onClick={() => onOpen(selected.id)}>팀과 대화</button></section>}</div>}
    {goals.length === 0 && <div className="empty-state compact">목표를 전달하면 업무 현황을 볼 수 있습니다.</div>}
  </div>;
}
