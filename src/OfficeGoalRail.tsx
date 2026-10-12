import { GoalGraph } from './OfficeActivity';
import { ListDisclosure, useLimitedItems } from './ContentDensity';
import { goalTitle } from './progress';
import { meetingMembers, type MeetingAttention } from './meetingStatus';
import type { Agent, Approval, Event, Goal, GoalGuidance, Meeting, MeetingPacket, Task } from './types';
import { blockFacts } from './deliverables';

export function OfficeGoalRail({ goals, tasks, agents, approvals, onOpen, onReview, meetingStatus, meetings = [], meetingPackets = [], guidance = [], events = [] }: { goals: Goal[]; tasks: Task[]; agents: Agent[]; approvals: Approval[]; onOpen: (id: string) => void; onReview: (goal: Goal) => void; meetingStatus?: (goal: Goal) => MeetingAttention; meetings?: Meeting[]; meetingPackets?: MeetingPacket[]; guidance?: GoalGuidance[]; events?: Event[] }) {
  const active = goals.filter(goal => goal.status !== 'completed').sort((a, b) => {
    const priority = (goal: Goal) => approvals.some(item => item.goalId === goal.id && item.status === 'pending') ? 0 : goal.status === 'blocked' ? 1 : goal.status === 'new' ? 3 : 2;
    return priority(a) - priority(b) || b.createdAt.localeCompare(a.createdAt);
  });
  const recent = goals.filter(goal => goal.status === 'completed').sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  const window = useLimitedItems(active);
  const history = useLimitedItems(recent);
  function cards(items: Goal[]) {
    return items.map(goal => {
      const assigned = [...new Set(tasks.filter(task => task.goalId === goal.id).map(task => task.assigneeId))].map(id => agents.find(agent => agent.id === id)?.name ?? id);
      const pending = approvals.some(item => item.goalId === goal.id && item.status === 'pending');
      if (goal.kind === 'deliberation') { const status = meetingStatus?.(goal); return <article className="office-goal" key={goal.id}><div className="office-goal-head"><strong title={goal.intent}>{goalTitle(goal, 95)}</strong><span className={`status ${goal.status}`}>다관점 회의 · {status?.label ?? '상태 확인'}</span></div><p>{status ? `${status.detail} · 다음: ${status.next}` : '회의 작업공간에서 상태를 확인하세요'}</p>{(() => { const cast = meetingMembers(meetings.find(item => item.goalId === goal.id), meetingPackets, agents); return cast.chair ? <p className="meeting-note">의장 {cast.chair}{cast.participants.length ? ` · 참여 ${cast.participants.join(', ')}` : ''}{cast.evaluator ? ` · 평가 ${cast.evaluator}` : ''}{cast.calling.length ? ` · 지금 발언: ${cast.calling.join(', ')}` : ''}</p> : null; })()}<div className="button-row"><button type="button" className={status?.needsDecision ? 'primary' : 'secondary'} onClick={() => onOpen(goal.id)}>{status?.needsDecision ? '결정 열기 →' : '회의 열기 →'}</button></div></article>; }
      return <article className="office-goal" key={goal.id}><div className="office-goal-head"><strong title={goal.intent}>{goalTitle(goal, 95)}</strong><span className={`status ${goal.status}`}>{pending ? 'CEO 승인 대기' : goal.status === 'blocked' ? '확인 필요' : goal.status === 'new' ? '시작 전' : goal.status === 'completed' ? '완료' : '진행 중'}</span></div><GoalGraph goal={goal} tasks={tasks} /><p>참여 팀원: {assigned.length ? assigned.join(', ') : '배정 전'} · QA 재작업 {goal.reviewLoopsUsed}/{goal.reviewLoopLimit ?? 2}회</p>{(() => { const facts = blockFacts(goal, tasks, approvals, guidance, agents, events); return facts ? <p className="office-goal-block" title={facts.lastFact}><strong>{facts.kindLabel}</strong> · {facts.reason} · 담당 {facts.owner} · 다음: {facts.next}</p> : null; })()}<div className="button-row"><button type="button" className="secondary" onClick={() => onOpen(goal.id)}>팀과 대화 →</button>{(pending || goal.status === 'blocked') && <button type="button" className="secondary" onClick={() => onReview(goal)}>확인하기 →</button>}</div></article>;
    });
  }
  return <section className="office-goal-rail" aria-label="진행 중인 목표"><div className="section-head"><h2>진행 중인 목표</h2><span className="count">{active.length} GOALS</span></div><div className="office-goal-grid">{cards(window.visible)}</div>{active.length === 0 && <div className="empty-state compact">진행 중인 목표가 없습니다.</div>}<ListDisclosure label="진행 중인 목표" total={window.total} hidden={window.hidden} expanded={window.expanded} onToggle={window.toggle} />{recent.length > 0 && <details className="office-goal-history"><summary>완료된 목표 {recent.length}개 보기</summary><div className="office-goal-grid">{cards(history.visible)}</div><ListDisclosure label="완료된 목표" total={history.total} hidden={history.hidden} expanded={history.expanded} onToggle={history.toggle} /></details>}</section>;
}
