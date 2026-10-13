import SddOriginCard from './SddOriginCard';
import { connectionGap, type GoalOrigin } from './sddOrigin';
import type { Project, Squad } from './types';

type Props = {
  goalId: string | null | undefined;
  goalOrigins: GoalOrigin[] | undefined;
  projects: Project[];
  squads: Squad[] | undefined;
  onOpenConnections: () => void;
  onError: (message: string) => void;
};

/**
 * ADR-0045: the plan approval card asks about the project structure (development goals only) and about
 * missing AI connections together. Each answer is separate from plan approval; nothing here approves the plan.
 */
export function PlanningOriginPanel({ goalId, goalOrigins, projects, squads, onOpenConnections, onError }: Props) {
  if (!goalId) return null;
  const origin = goalOrigins?.find(item => item.goalId === goalId);
  const project = origin && projects.find(item => item.id === origin.projectId);
  const gap = connectionGap(squads?.find(item => item.goalId === goalId)?.shortfall);
  const askStructure = Boolean(origin?.offered && project);
  if (!askStructure && !gap) return null;
  return <>
    {askStructure && origin && project && <SddOriginCard key={`${goalId}:${origin.decision ?? ''}`} project={project} blocked={false} onError={onError} goalOrigin={origin} />}
    {gap && <div className="spec-preview sdd-origin"><h3>AI 연결</h3><p>{gap}</p><div className="button-row"><button type="button" className="secondary" onClick={onOpenConnections}>AI 연결 열기</button></div></div>}
  </>;
}
