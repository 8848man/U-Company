import type { ReactNode } from 'react';
import { ListDisclosure, useLimitedItems } from '../../src/ContentDensity';
import { goalTitle } from '../../src/progress';
import type { Goal } from '../../src/types';

type Props = {
  goals: Goal[];
  selectedId: string | null;
  /** Goals that need the CEO now (approval, recovery, meeting decision); listed first and marked. */
  needsDecision: (goal: Goal) => boolean;
  stageLabel: (goal: Goal) => string;
  onSelect: (id: string) => void;
  onNew: () => void;
  children: ReactNode;
};

/** v3 home (ADR-0046 decision 5): goal list on the left, the selected goal's conversation or a new goal in the center. */
export function V3ChatHome({ goals, selectedId, needsDecision, stageLabel, onSelect, onNew, children }: Props) {
  const ordered = [...goals].sort((a, b) => Number(needsDecision(b)) - Number(needsDecision(a)) || b.createdAt.localeCompare(a.createdAt));
  const list = useLimitedItems(ordered, 12);
  return <div className="v3-chat-home">
    <aside className="v3-goal-list" aria-label="목표 목록">
      <button type="button" className={selectedId ? 'secondary v3-new-goal' : 'primary v3-new-goal'} onClick={onNew}>+ 새 목표</button>
      <nav>{list.visible.map(goal => <button type="button" key={goal.id} className={`v3-goal-item${goal.id === selectedId ? ' is-selected' : ''}`} aria-current={goal.id === selectedId ? 'true' : undefined} onClick={() => onSelect(goal.id)}>
        <span className="v3-goal-item-title" title={goal.intent}>{goalTitle(goal, 48)}</span>
        <small>{needsDecision(goal) && <b className="v3-goal-flag">결정 필요</b>}{stageLabel(goal)}</small>
      </button>)}</nav>
      {!goals.length && <p className="muted">아직 목표가 없어요.</p>}
      <ListDisclosure label="목표" total={list.total} hidden={list.hidden} expanded={list.expanded} onToggle={list.toggle} />
    </aside>
    <div className="v3-chat-main">{children}</div>
  </div>;
}
