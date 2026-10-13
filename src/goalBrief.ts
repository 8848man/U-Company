import type { Agent, Approval, Event, Goal, GoalGuidance, Task, TokenUsageRecord } from './types';
import { goalRecovery } from './goalRecovery';

const operationalKinds = new Set(['goal_started', 'goal_stage', 'goal_blocked', 'task_delegated', 'task_started', 'task_agent_finished', 'task_failed', 'task_interrupted', 'runtime_progress', 'qa_review', 'review_rework_started', 'approval_requested', 'approval_decided', 'goal_completed', 'goal_interrupted', 'goal_guidance_review_required']);

export function goalStageLabel(status: string): string {
  return ({ new: '시작 전', analyzing: '목표 분석', planning: '업무 계획', awaiting_approval: 'CEO 승인 대기', executing: '구현 중', integrating: '결과 통합', validating: 'QA 검증', deliberating: '다관점 회의', awaiting_decision: '회의 결정 대기', completed: '완료', blocked: '확인 필요', cancelled: '취소됨' } as Record<string, string>)[status] ?? '상태 확인 필요';
}

export function taskStageLabel(status: string): string {
  return ({ pending: '시작 대기', running: '작업 중', awaiting_integration: '통합 대기', completed: '완료', blocked: '확인 필요', failed: '실패' } as Record<string, string>)[status] ?? '상태 확인 필요';
}

export function goalBrief(goal: Goal, tasks: Task[], approvals: Approval[], guidance: GoalGuidance[], agents: Agent[], events: Event[], usage: TokenUsageRecord[]) {
  const owned = tasks.filter(task => task.goalId === goal.id);
  const running = owned.filter(task => task.status === 'running');
  const pendingApproval = approvals.find(item => item.goalId === goal.id && item.status === 'pending');
  const pendingGuidance = guidance.find(item => item.goalId === goal.id && ['pending', 'review_required'].includes(item.status));
  const latestEvent = events.find(event => event.goalId === goal.id && operationalKinds.has(event.kind)) ?? null;
  const goalUsage = usage.filter(item => item.goalId === goal.id);
  const tokenTotal = goalUsage.reduce((sum, item) => sum + item.inputTokens + item.outputTokens, 0);
  const estimated = goalUsage.some(item => item.source === 'estimated');
  const name = (id: string) => agents.find(agent => agent.id === id)?.name ?? id;
  const owners = running.length ? [...new Set(running.map(task => name(task.assigneeId)))]
    : pendingGuidance?.status === 'review_required' || pendingApproval || ['new', 'blocked'].includes(goal.status) ? ['CEO']
      : goal.status === 'completed' ? ['팀 작업 완료']
        : goal.status === 'analyzing' ? [name('cto')]
      : goal.status === 'planning' ? [name('pm')]
        : goal.status === 'validating' ? [name('qa')]
          : [...new Set(owned.filter(task => ['pending', 'awaiting_integration'].includes(task.status)).map(task => name(task.assigneeId)))];
  const nextAction = pendingGuidance ? pendingGuidance.status === 'pending' ? '진행 중인 호출 후 방향 변경 검토' : 'CEO가 새 목표 버전 검토'
    : pendingApproval ? 'CEO가 변경 파일 범위 확인'
      : goal.status === 'new' ? 'CEO가 팀 실행 시작'
        : goal.status === 'completed' ? '결과와 QA 근거 확인'
          : goal.status === 'blocked' || goal.status === 'awaiting_approval' ? goalRecovery(goal, tasks, approvals, guidance)?.label ?? '상태 확인'
              : goal.status === 'validating' ? 'QA 결과 대기'
                : '팀의 다음 실행 기록 대기';
  return { owners, nextAction, latestEvent, owned, pendingApproval, tokenTotal, estimated };
}
