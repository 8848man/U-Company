import { useEffect, useState } from 'react';
import type { Agent, Approval, Goal, Task, TokenUsageRecord } from './types';
import { groupGoals, phaseState, phases, roleDialogue, goalTitle, type MemberActivity } from './progress';
import { ContentPreview, ListDisclosure, useLimitedItems } from './ContentDensity';

export function GoalGraph({ goal, tasks }: { goal: Goal; tasks: Task[] }) {
  const owned = tasks.filter(task => task.goalId === goal.id);
  const done = owned.filter(task => task.status === 'completed').length;
  return <div className="work-graph" aria-label={`업무 진행: ${owned.length}개 중 ${done}개 통합 완료`}>
    <div className="phase-graph">{phases.map(phase => <div className={`phase-step ${phaseState(goal, phase.id)}`} key={phase.id}><span className="phase-bar" /><small>{phase.label}</small></div>)}</div>
    <p>{owned.length ? `파일 업무 ${owned.length}개 중 ${done}개 통합 완료` : '업무 계획 전'} · 현재: {goal.status === 'blocked' ? '확인 필요' : goal.status === 'completed' ? '완료' : phases.find(phase => phase.id === goal.status)?.label ?? goal.status}</p>
  </div>;
}

export function OfficeActivity({ agent, activity, tasks, usage, onOpenGoal }: { agent: Agent; activity: MemberActivity; tasks: Task[]; usage: TokenUsageRecord[]; onOpenGoal: (id: string) => void }) {
  const activityLog = useLimitedItems(activity.events);
  const ownedUsage = usage.filter(item => item.agentId === agent.id);
  const active = ownedUsage.filter(item => item.status === 'active').reduce((total, item) => total + item.inputTokens + item.outputTokens, 0);
  const cumulative = ownedUsage.reduce((total, item) => total + item.inputTokens + item.outputTokens, 0);
  return <div className="member-activity">
    <div className="activity-heading"><span className={`activity-dot ${activity.kind}`} /><div><strong>{agent.name}</strong><small>{activity.kind === 'active' ? `작업 중 · ${activity.label}` : activity.kind === 'blocked' ? `확인 필요 · ${activity.label}` : activity.kind === 'waiting' ? activity.label : '현재 대기 중'}</small></div></div>
    <div className="token-summary" aria-live="polite"><span>현재 실행 토큰 추정치 <strong>{active.toLocaleString()}</strong></span><span>프로젝트 누적 <strong>{cumulative.toLocaleString()}</strong></span><small>{ownedUsage.some(item => item.source === 'estimated') ? '일부 추정 포함 · 실행 종료 시 보정' : '제공자 보고 기반'}</small></div>
    {activity.goal && <><div className="activity-assignment"><strong>지금 맡은 일</strong><ContentPreview><p>{activity.task?.title ?? activity.label}</p></ContentPreview>{activity.task?.files.length ? <small>담당 파일: {activity.task.files.join(', ')}</small> : null}<button type="button" className="secondary goal-workspace-open" onClick={() => onOpenGoal(activity.goal!.id)}>{activity.goal.status === 'blocked' ? '중단 원인과 다음 단계 열기 →' : '목표 작업공간 열기 →'}</button></div><GoalGraph goal={activity.goal} tasks={tasks} /></>}
    {activity.kind === 'idle' ? <div className="sample-dialogue"><span className="sample-label">예시 대화 · 실제 AI 응답 아님</span><p className="chat-line ceo">CEO <span>지금은 어떤 일을 하나요?</span></p>{roleDialogue(agent).map((line, index) => <p className="chat-line member" key={index}>{agent.name} <span>{line}</span></p>)}</div> : <div className="activity-log"><div className="activity-log-title"><strong>실제 진행 기록</strong><small>최근 기록부터 표시</small></div>{activity.events.length ? activityLog.visible.map(event => <div className="activity-entry" key={event.id}><time>{new Date(event.at).toLocaleTimeString()}</time><ContentPreview><span>{['goal_blocked', 'task_failed'].includes(event.kind) ? '실행이 중단되었습니다. 위의 목표 작업공간에서 원인과 다음 단계를 확인하세요.' : event.kind === 'agent_handoff' ? '다음 담당자에게 업무를 인계했습니다.' : event.message}</span></ContentPreview></div>) : <p className="activity-empty">아직 세부 실행 기록이 없습니다. 상태가 바뀌면 여기에 표시됩니다.</p>}<ListDisclosure label="최근 진행 기록" total={activityLog.total} hidden={activityLog.hidden} expanded={activityLog.expanded} onToggle={activityLog.toggle} /></div>}
  </div>;
}

function GroupField({ goal, onSave }: { goal: Goal; onSave: (id: string, group: string) => void }) {
  const [value, setValue] = useState(goal.workGroup ?? '');
  useEffect(() => setValue(goal.workGroup ?? ''), [goal.workGroup]);
  return <div className="goal-group-field"><label htmlFor={`group-${goal.id}`}>같은 일로 묶기</label><input id={`group-${goal.id}`} list="goal-group-options" value={value} maxLength={80} placeholder="묶음 이름 입력 또는 선택" onChange={event => setValue(event.target.value)} />{value.trim() !== (goal.workGroup ?? '') && <button type="button" className="secondary" onClick={() => onSave(goal.id, value)}>저장</button>}</div>;
}

export function BoardProgress({ goals, tasks, agents, approvals, onSaveGroup, onOpenGoal }: { goals: Goal[]; tasks: Task[]; agents: Agent[]; approvals: Approval[]; onSaveGroup: (id: string, group: string) => void; onOpenGoal: (id: string) => void }) {
  const ordered = [...goals].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  const goalWindow = useLimitedItems(ordered);
  if (!goals.length) return null;
  const fullGroups = groupGoals(goals);
  const suggestions = [...new Set(goals.map(goal => goal.workGroup).filter((label): label is string => Boolean(label)))];
  return <div className="board-progress-list"><div className="section-head"><h2>목표별 진행 그래프</h2><span className="count">{goals.length} GOALS</span></div><datalist id="goal-group-options">{suggestions.map(label => <option value={label} key={label} />)}</datalist>{groupGoals(goalWindow.visible).map(group => <section className="board-goal-group" key={group.key}>
    {group.label && <div className="board-group-heading"><strong>▣ {group.label}</strong><small>{fullGroups.find(item => item.key === group.key)?.goals.length ?? group.goals.length}개 목표를 함께 보는 묶음 · 실행과 승인은 목표별로 유지</small></div>}
    {group.goals.map(goal => {
      const ownedTasks = tasks.filter(task => task.goalId === goal.id);
      const members = [...new Set(ownedTasks.map(task => task.assigneeId))].map(id => agents.find(agent => agent.id === id)?.name ?? id);
      const pending = approvals.some(approval => approval.goalId === goal.id && approval.status === 'pending');
      return <article className="card board-progress" key={goal.id}><div className="list-head"><div><small className="board-goal-status">{goal.status === 'completed' ? '완료' : goal.status === 'blocked' ? '확인 필요' : pending ? 'CEO 승인 대기' : goal.status === 'new' ? '시작 전' : '진행 중'}</small><h3 title={goal.intent}>{goalTitle(goal, 140)}</h3>{goalTitle(goal, 140) !== goal.intent && <details className="board-goal-full"><summary>목표 원문 보기</summary><ContentPreview><p>{goal.intent}</p></ContentPreview></details>}</div><small>{new Date(goal.createdAt).toLocaleString()}</small></div><GoalGraph goal={goal} tasks={tasks} />
        <div className="board-goal-meta"><span>참여 개발자: {members.length ? members.join(', ') : '업무 배정 전'}</span>{pending && <strong>결정 필요</strong>}</div>
        <GroupField goal={goal} onSave={onSaveGroup} />
        <button type="button" className="secondary goal-workspace-open" onClick={() => onOpenGoal(goal.id)}>목표 작업공간 열기 →</button>
      </article>;
    })}
  </section>)}<ListDisclosure label="목표 그래프" total={goalWindow.total} hidden={goalWindow.hidden} expanded={goalWindow.expanded} onToggle={goalWindow.toggle} /></div>;
}
