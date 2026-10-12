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

export function OpenWorkLauncher({ items, agents, onSelect }: { items: OpenWork[]; agents: Agent[]; onSelect: (goalId: string, taskId?: string) => void }) {
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const panel = useRef<HTMLElement>(null);
  const list = useLimitedItems(items);
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
  return <div className="open-work-launcher">
    {open && <section id="open-work-panel" ref={panel} className="open-work-panel" role="dialog" aria-label="끝나지 않은 일"><header><div><span className="card-kicker">OPEN WORK</span><h2>끝나지 않은 일</h2><p>업무를 선택해 담당 팀원과 대화하고 다음 단계를 결정하세요.</p></div><button type="button" className="secondary" onClick={() => { setOpen(false); trigger.current?.focus(); }} aria-label="끝나지 않은 일 닫기">닫기</button></header><div className="open-work-list">{list.visible.map(item => <WorkGroup key={item.goal.id} item={item} agents={agents} onSelect={select} />)}{items.length === 0 && <p className="open-work-empty">현재 열려 있는 목표가 없습니다.</p>}<ListDisclosure label="목표" total={list.total} hidden={list.hidden} expanded={list.expanded} onToggle={list.toggle} /></div></section>}
    <button ref={trigger} type="button" className="open-work-trigger" aria-label={`열린 작업 · 미완료 목표 ${items.length}개`} aria-expanded={open} aria-controls={open ? 'open-work-panel' : undefined} onClick={() => setOpen(current => !current)}><span aria-hidden="true">✦</span><span>열린 작업</span><strong>{items.length}</strong></button>
  </div>;
}
