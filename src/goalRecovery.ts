import type { Approval, Goal, GoalGuidance, Task } from './types';

const revisionExplanation: Record<string, string> = {
  approval_denied: '거절은 기록되었고 같은 범위를 자동으로 다시 요청하지 않습니다. 범위를 바꾸려면 CEO 지침으로 새 목표 버전을 만들어 계획과 승인을 다시 받습니다.',
  interrupted: '앱이 실행 중에 종료되어 작업 결과를 알 수 없습니다. 격리 작업공간에 일부 변경이 남았을 수 있어 자동으로 다시 실행하거나 통합하지 않습니다. 기록을 확인한 뒤 새 목표 버전으로 이어 갑니다.',
  integration_conflict: '원래 프로젝트 파일은 덮어쓰지 않았습니다. 충돌한 파일과 기존 통합 기록을 확인한 뒤 새 목표 버전에서 범위를 다시 승인받습니다.',
  qa_failed: 'QA 근거와 실패한 산출물은 보존했습니다. 재작업 한도를 넘었거나 새 파일이 필요해 CEO의 결정이 필요합니다.',
  qa_form: 'QA가 산출물별 판정을 형식에 맞게 내지 않아 완료로 처리하지 않았습니다. 재작업 횟수는 쓰지 않았습니다. 새 목표 버전에서 QA를 다시 받습니다.',
  plan_form: 'PM의 계획이 앱의 형식 검사를 두 번 통과하지 못했습니다. 재작업 횟수는 쓰지 않았습니다. 해결을 요청하면 PM이 같은 목표에서 승인된 파일 안의 재작업을 다시 계획합니다.',
  provider: '실행 중 AI 호출이 실패했습니다. 연결을 확인한 뒤 기존 결과를 보존한 새 목표 버전으로 이어 갑니다.',
};

export type RecoveryKind = 'guidance' | 'replacement' | 'approval' | 'retry' | 'resume' | 'revision';

export function goalRecovery(goal: Goal, tasks: Task[], approvals: Approval[], guidance: GoalGuidance[]): { kind: RecoveryKind; label: string; explanation: string } | null {
  if (goal.status !== 'blocked' && goal.status !== 'awaiting_approval') return null;
  const ownedTasks = tasks.filter(task => task.goalId === goal.id);
  const ownedApprovals = approvals.filter(approval => approval.goalId === goal.id);
  const activeGuidance = guidance.find(item => item.goalId === goal.id && ['pending', 'review_required'].includes(item.status));
  if (activeGuidance) return { kind: 'guidance', label: activeGuidance.status === 'review_required' ? '새 목표 버전 검토' : '방향 변경 대기', explanation: activeGuidance.status === 'review_required' ? '저장된 변경 지침을 확인하고 새 목표 버전을 만들 수 있습니다.' : '진행 중인 호출이 끝나면 안전 지점에서 변경 지침을 검토합니다.' };
  if (guidance.some(item => item.goalId === goal.id && item.status === 'revised' && item.replacementGoalId)) return { kind: 'replacement', label: '새 목표 버전 열기', explanation: '이 목표의 방향 변경은 새 목표 버전으로 이어졌습니다. 새 버전에서 계획과 승인을 진행해 주세요.' };
  if (ownedApprovals.some(approval => approval.status === 'pending')) return { kind: 'approval', label: '승인 범위 확인', explanation: '팀이 요청한 프로젝트 변경 범위를 확인하고 승인 또는 거부해 주세요.' };
  if (goal.status === 'blocked' && ['analyzing', 'planning'].includes(goal.resumedFrom ?? '') && ownedTasks.length === 0 && ownedApprovals.length === 0)
    return { kind: 'retry', label: '문제 해결 후 다시 실행', explanation: '파일 작업과 승인이 시작되지 않아 기존 목표의 분석을 다시 시작할 수 있습니다.' };
  // ADR-0038 §4: an approved plan whose unfinished tasks never started may resume; the engine
  // re-checks connection, busy state and approval before any call.
  if (goal.status === 'blocked' && ownedTasks.some(task => task.status === 'pending') && ownedTasks.every(task => task.status === 'pending' || task.status === 'completed') && ownedApprovals.some(approval => approval.operation === 'project_write' && approval.status === 'approved'))
    return { kind: 'resume', label: '승인된 계획 이어서 실행', explanation: goal.blockKind === 'precall' ? '호출 전 확인에서 멈춰 AI 호출은 없었습니다. 원인(연결·팀원 상태)을 고친 뒤 승인된 범위의 미시작 업무만 실행합니다. 엔진이 시작 전에 다시 확인합니다.' : '승인된 파일 범위와 아직 시작하지 않은 업무를 그대로 실행합니다. 통합된 업무는 다시 실행하지 않습니다.' };
  // ADR-0048: the card's main action is the one-click fix request, except where only the CEO or a connection can help.
  const label = goal.blockKind === 'approval_denied' ? '수정 지침으로 새 목표 만들기' : goal.blockKind === 'provider' || goal.blockKind === 'precall' ? 'AI 연결 확인' : '팀원에게 해결 요청';
  return { kind: 'revision', label, explanation: revisionExplanation[goal.blockKind ?? ''] ?? '이 단계의 기존 AI 세션은 자동 재개할 수 없습니다. 기존 작업과 오류를 검토한 뒤 CEO 지침으로 새 목표 버전을 만들고 계획·승인을 다시 받습니다.' };
}

/**
 * ADR-0048: what the recovery card's one-click `해결 요청` will do. Mirrors `team_fix::route` in the
 * engine, which decides again on the click; this only labels the button.
 */
export type TeamFix = { route: 'repair' | 'revision'; note: string } | { route: 'connections' | 'direct'; note: string };

export function teamFixRoute(goal: Goal, tasks: Task[], approvals: Approval[], guidance: GoalGuidance[]): TeamFix {
  const kind = goal.blockKind ?? '';
  if (kind === 'provider' || kind === 'precall') return { route: 'connections', note: 'AI 연결이나 팀원 상태 문제라 팀원이 고칠 수 없어요. 연결을 확인해 주세요.' };
  if (kind === 'approval_denied') return { route: 'direct', note: '거절한 범위예요. 무엇을 바꿀지 직접 적어 주세요.' };
  const owned = tasks.filter(task => task.goalId === goal.id);
  const approved = approvals.some(item => item.goalId === goal.id && item.operation === 'project_write' && item.status === 'approved');
  const repair = ['plan_form', 'qa_failed', 'qa_form', 'other'].includes(kind) && goal.resumedFrom === 'validating' && approved
    && !guidance.some(item => item.goalId === goal.id) && owned.length > 0 && owned.every(task => task.status === 'completed');
  if (repair) {
    const limit = goal.reviewLoopLimit ?? 2;
    const extra = goal.reviewLoopsUsed >= limit ? ` 재작업 ${goal.reviewLoopsUsed}/${limit}회를 썼어요. 누르면 1회 추가돼요.` : '';
    return { route: 'repair', note: `같은 목표에서 승인된 파일 안의 재작업을 계획하고 바로 진행해요. 새 승인은 필요 없어요.${extra}` };
  }
  return { route: 'revision', note: '기존 작업을 보존한 새 목표 버전을 만들어 다시 계획해요. 새 계획은 계획 카드에서 승인해 주세요.' };
}

/** Who receives a fix request: the goal's planning lead, else the PM. */
export function fixReceiver(goalId: string, squads: { goalId: string; members: { slot: string; duty: string; agentId: string }[] }[] | undefined, agents: { id: string; name: string; enabled: boolean }[]): string {
  const lead = squads?.find(squad => squad.goalId === goalId)?.members.find(member => member.slot === 'lead' && member.duty === 'planning')?.agentId;
  return agents.find(agent => agent.id === lead)?.name ?? agents.find(agent => agent.id === 'pm' && agent.enabled)?.name ?? 'PM';
}
