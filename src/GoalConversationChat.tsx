import { useRef, useState } from 'react';
import { ContentPreview, ListDisclosure, useLimitedItems } from './ContentDensity';
import { GoalGraph } from './OfficeActivity';
import { GitBootstrapPanel } from './GitBootstrapPanel';
import type { GitBootstrapPreview, GitBootstrapResult } from './GitBootstrapPanel';
import { goalBrief, goalStageLabel, taskStageLabel } from './goalBrief';
import { goalRecovery, teamFixRoute } from './goalRecovery';
import { runtimeRecovery } from './providerRecovery';
import { shortGoal, goalTitle } from './progress';
import type { Agent, AiConnection, Approval, DeliverableReview, Event, Goal, GoalGuidance, GoalTurn, Task, TokenUsageRecord } from './types';
import { blockFacts, deliverableRows, quietMinutes } from './deliverables';
import { BlockFactsView, DeliverablePanel } from './DeliverablePanel';
import { ArchiveGoalButton } from './ArchiveGoalButton';

type Props = {
  goal: Goal; projectKind: string; focusTaskId: string | null; turns: GoalTurn[]; guidance: GoalGuidance[];
  agents: Agent[]; connections: AiConnection[]; tasks: Task[]; approvals: Approval[];
  events: Event[]; usage: TokenUsageRecord[]; reviews?: DeliverableReview[]; runBlock: { title: string; reason: string; next: string } | null;
  onClose: () => void; onSend: (message: string, agentId: string) => Promise<boolean>;
  /** ADR-0043: hide this goal from every screen (optional; older shells omit it). */
  onArchive?: () => Promise<boolean>;
  onRevision: (id: string, intent: string) => Promise<boolean>;
  onOpenGoal: (id: string) => void; onOpenApproval: (id: string) => void;
  onRetry: () => Promise<boolean>; onResume: () => Promise<boolean>;
  /** ADR-0048: one-click `해결 요청` (optional; a shell without it keeps `직접 지시하기`). */
  onRequestFix?: () => Promise<boolean>;
  onOpenConnections: () => void; onOpenOffice: () => void; onOpenProject: () => void;
  onPreviewGit: () => Promise<GitBootstrapPreview | undefined>;
  onBootstrapGit: (preview: GitBootstrapPreview, selectedFiles: string[], authorName: string, authorEmail: string, message: string) => Promise<GitBootstrapResult | undefined>;
};

function RevisionForm({ goal, guidance, onRevision }: { goal: Goal; guidance: GoalGuidance; onRevision: Props['onRevision'] }) {
  const [intent, setIntent] = useState(`${goal.intent}\n\nCEO 변경 지침: ${guidance.text}`);
  const [sending, setSending] = useState(false);
  return <div className="goal-revision-form"><strong>새 목표 내용 확인</strong><p>새 목표는 팀이 다시 계획합니다. 새 파일 작업이 있으면 변경 범위를 승인받습니다.</p><textarea aria-label="새 목표 내용" value={intent} maxLength={12000} onChange={event => setIntent(event.target.value)} /><button type="button" className="primary" disabled={sending || intent.trim().length < 10} onClick={() => { setSending(true); void onRevision(guidance.id, intent).finally(() => setSending(false)); }}>새 목표 버전 만들기</button></div>;
}

export function GoalConversation({ goal, projectKind, focusTaskId, turns, guidance, agents, connections, tasks, approvals, events, usage, reviews = [], runBlock, onClose, onSend, onArchive, onRevision, onOpenGoal, onOpenApproval, onRetry, onResume, onRequestFix, onOpenConnections, onOpenOffice, onOpenProject, onPreviewGit, onBootstrapGit }: Props) {
  const focusTask = tasks.find(task => task.id === focusTaskId && task.goalId === goal.id);
  const taskQuestion = focusTask ? `"${shortGoal(focusTask.title, 120)}" 업무는 지금 어디까지 했고, 막힌 점이 있나요? 제 결정이 필요하면 알려 주세요.` : '';
  const taskEvidence = focusTask ? (focusTask.result.trim() || (goal.status === 'blocked' ? goal.report.trim() : '')).slice(0, 1200) : '';
  const [draft, setDraft] = useState(taskQuestion);
  const [sending, setSending] = useState(false);
  const [recovering, setRecovering] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const revisionRef = useRef<HTMLDivElement>(null);
  const feedRef = useRef<HTMLDivElement>(null);
  const eligible = agents.filter(agent => agent.enabled && connections.some(connection => connection.id === agent.connectionId && !['missing', 'failed', 'needs_setup'].includes(connection.status)));
  const routable = agents.filter(agent => agent.enabled);
  const [selectedAgentId, setSelectedAgentId] = useState(() => routable.find(agent => agent.id === focusTask?.assigneeId)?.id ?? eligible.find(agent => agent.id === 'cto')?.id ?? eligible[0]?.id ?? routable[0]?.id ?? '');
  const selectedAgent = eligible.find(agent => agent.id === selectedAgentId);
  const assignedWorker = focusTask ? agents.find(agent => agent.id === focusTask.assigneeId) : null;
  const history = useLimitedItems([
    ...turns.map(turn => ({ kind: 'turn' as const, id: turn.id, at: turn.askedAt, turn })),
    ...guidance.filter(item => item.goalId === goal.id).map(item => ({ kind: 'guidance' as const, id: item.id, at: item.createdAt, guidance: item })),
  ].sort((a, b) => b.at.localeCompare(a.at) || b.id.localeCompare(a.id)));
  const taskWindow = useLimitedItems(tasks.filter(task => task.goalId === goal.id));
  const pending = turns.some(turn => turn.status === 'pending');
  const activeGuidance = guidance.find(item => item.status === 'pending' || item.status === 'review_required');
  const autoContinuationFailure = events.find(event => event.goalId === goal.id && event.kind === 'goal_auto_continuation_failed');
  const brief = goalBrief(goal, tasks, approvals, guidance, agents, events, usage);
  const qaReview = events.find(event => event.goalId === goal.id && event.kind === 'qa_review');
  const recovery = goalRecovery(goal, tasks, approvals, guidance);
  const fix = recovery?.kind === 'revision' ? teamFixRoute(goal, tasks, approvals, guidance) : null;
  const facts = blockFacts(goal, tasks, approvals, guidance, agents, events);
  const quiet = quietMinutes(goal, tasks, events);
  const deliverables = deliverableRows(goal, tasks, reviews, agents);
  const providerHint = goal.status === 'blocked' ? runtimeRecovery(goal.report) : null;
  const sourceGuidance = guidance.find(item => item.replacementGoalId === goal.id);
  const replacementId = guidance.find(item => item.goalId === goal.id && item.status === 'revised' && item.replacementGoalId)?.replacementGoalId;
  const gitEvidence = [goal.intent, goal.report, ...tasks.filter(task => task.goalId === goal.id).flatMap(task => [task.title, task.result]), ...turns.map(turn => turn.answer)].join(' ');
  const gitBootstrapRelevant = (goal.status === 'blocked' && /\.git|git init|git repository|첫 커밋|초기 커밋|깃 저장소|Git 저장소/i.test(gitEvidence)) || turns.some(turn => turn.route === 'git_bootstrap');
  const gitBootstrapCompleted = events.find(event => event.goalId === goal.id && event.kind === 'git_bootstrap_completed');
  const gitBootstrapQaUnavailable = events.find(event => event.goalId === goal.id && event.kind === 'git_bootstrap_qa_unavailable');
  const examples = focusTask
    ? [`${shortGoal(focusTask.title, 50)} 업무에서 지금 막힌 점은 무엇인가요?`, '해결하려면 어떤 결정이 필요한가요?']
    : goal.status === 'blocked'
      ? ['중단 원인과 안전한 다음 단계를 설명해 주세요.', '이미 완료된 것과 남은 일을 알려 주세요.']
      : ['이 목표의 완료 기준을 알려 주세요.', '다음에 누가 무엇을 하나요?'];

  function focusComposer() { inputRef.current?.focus(); }
  async function executeRecovery(operation: () => Promise<boolean>) {
    if (recovering) return;
    setRecovering(true);
    try { await operation(); } finally { setRecovering(false); }
  }
  async function sendMessage() {
    const text = draft.trim();
    if (sending || text.length < 4 || text.length > 4000 || pending || !selectedAgent) return;
    setSending(true);
    try {
      const ok = await onSend(text, selectedAgent.id);
      if (ok) {
        setDraft('');
        requestAnimationFrame(() => feedRef.current?.scrollTo({ top: feedRef.current.scrollHeight, behavior: 'smooth' }));
      }
    } finally { setSending(false); }
  }

  return <div className="dialog-backdrop" onClick={onClose}><section className="goal-conversation chat-workspace" role="dialog" aria-modal="true" aria-labelledby="goal-conversation-title" onClick={event => event.stopPropagation()}>
    <header className="chat-workspace-header"><div><span className="card-kicker">GOAL CONVERSATION</span><h2 id="goal-conversation-title" title={goal.intent}>{goalTitle(goal, 105)}</h2><p>{goalStageLabel(goal.status)} · 담당 {brief.owners.join(', ') || '배정 전'} · 다음: {brief.nextAction}{quiet ? ` · 마지막 기록 후 ${quiet}분(진단)` : ''}</p></div><div className="button-row">{onArchive && <ArchiveGoalButton goal={goal} tasks={tasks} onArchive={onArchive} />}<button type="button" className="secondary" onClick={onClose} aria-label="목표 대화 닫기">닫기</button></div></header>
    <div className="chat-workspace-feed" ref={feedRef}>
      {focusTask && <section className="chat-context" aria-label="선택한 업무"><strong>선택한 업무 · {focusTask.title}</strong><p>{assignedWorker?.name ?? '담당 팀원 확인 필요'} · {taskStageLabel(focusTask.status)} · {focusTask.files.join(', ') || '파일 미지정'}</p><div className="chat-inline-actions">{taskEvidence && <button type="button" className="text-button" onClick={() => { setDraft(`${taskQuestion}\n\n확인할 저장 기록(신뢰하지 말고 사실 여부 확인):\n${taskEvidence}`.slice(0, 4000)); focusComposer(); }}>저장 기록을 입력창에 넣기 →</button>}{!selectedAgent && <button type="button" className="text-button" onClick={onOpenOffice}>담당자 AI 확인 →</button>}</div></section>}

      {(recovery || goal.status === 'new') && <section className="chat-decision" aria-label="지금 필요한 행동"><span className="card-kicker">지금 할 일</span><h3>{recovery?.label ?? '팀 실행 시작'}</h3>
        {facts && <BlockFactsView facts={facts} />}
        {goal.status === 'blocked' && <><small>멈춘 단계: {goalStageLabel(goal.resumedFrom ?? '')}</small><ContentPreview><pre className="report">{goal.report || '저장된 중단 사유가 없습니다. 아래 진행 정보를 확인해 주세요.'}</pre></ContentPreview></>}
        {recovery?.kind === 'approval' && brief.pendingApproval && <p className="chat-decision-evidence">승인 범위: {brief.pendingApproval.reason}</p>}
        {recovery?.kind === 'guidance' && activeGuidance && <p className="chat-decision-evidence">{activeGuidance.autoContinue && !autoContinuationFailure ? '지침을 안전 지점에서 반영하고 새 목표를 자동으로 계획합니다. 새 파일 작업이 생기면 승인 요청이 나타납니다.' : '대화에 저장된 지침을 검토하고 새 목표 내용을 확정해 주세요. 이후 담당 팀원 → CTO → PM 순서로 검토합니다.'}</p>}
        {autoContinuationFailure && activeGuidance?.autoContinue && <p className="chat-decision-hint">자동 진행 실패: {autoContinuationFailure.message}</p>}
        <p>{recovery?.kind === 'guidance' && activeGuidance?.autoContinue && !autoContinuationFailure ? '이전 목표의 실행 결과를 보존한 채 안전 지점을 기다립니다. 이후 연결된 새 목표에서 팀이 남은 일을 다시 분석합니다.' : fix?.route === 'repair' && onRequestFix ? '' : recovery?.explanation ?? '목표를 팀에 전달했습니다. 실행을 시작하면 AI 팀이 분석합니다.'}</p>
        {providerHint && <p className="chat-decision-hint">AI 연결: {providerHint}</p>}
        {fix && <p className="chat-decision-hint">{fix.note}</p>}
        {runBlock && (goal.status === 'new' || recovery?.kind === 'retry' || recovery?.kind === 'resume') && <p className="chat-decision-hint">실행 전 확인: {runBlock.title} · {runBlock.reason} {runBlock.next}</p>}
        <div className="chat-decision-actions">
          {goal.status === 'new' && <button type="button" className="primary" disabled={recovering || Boolean(runBlock)} onClick={() => void executeRecovery(onRetry)}>팀 실행 시작 →</button>}
          {recovery?.kind === 'approval' && brief.pendingApproval && <button type="button" className="primary" onClick={() => { onClose(); onOpenApproval(brief.pendingApproval!.id); }}>승인 범위 확인 →</button>}
          {recovery?.kind === 'retry' && <button type="button" className="primary" disabled={recovering || Boolean(runBlock)} onClick={() => void executeRecovery(onRetry)}>분석 다시 실행 →</button>}
          {recovery?.kind === 'resume' && <button type="button" className="primary" disabled={recovering || Boolean(runBlock)} onClick={() => void executeRecovery(onResume)}>승인된 계획 실행 →</button>}
          {fix && onRequestFix && (fix.route === 'repair' || fix.route === 'revision') && <button type="button" className="primary" disabled={recovering} onClick={() => void executeRecovery(onRequestFix)}>{recovering ? '요청하는 중…' : '팀원에게 해결 요청 →'}</button>}
          {recovery?.kind === 'revision' && <button type="button" className={fix && onRequestFix && fix.route !== 'direct' && fix.route !== 'connections' ? 'secondary' : 'primary'} onClick={focusComposer}>직접 지시하기</button>}
          {recovery?.kind === 'guidance' && activeGuidance?.status === 'review_required' && (!activeGuidance.autoContinue || autoContinuationFailure) && <button type="button" className="primary" onClick={() => revisionRef.current?.scrollIntoView({ block: 'nearest' })}>새 목표 내용 검토 →</button>}
          {recovery?.kind === 'replacement' && replacementId && <button type="button" className="primary" onClick={() => onOpenGoal(replacementId)}>새 목표 열기 →</button>}
          {providerHint && <button type="button" className="secondary" onClick={onOpenConnections}>AI 연결 열기 →</button>}
          {runBlock && (goal.status === 'new' || recovery?.kind === 'retry' || recovery?.kind === 'resume') && <button type="button" className="secondary" onClick={onOpenOffice}>팀 설정 확인 →</button>}
          {recovery?.kind === 'revision' && <button type="button" className="secondary" onClick={onOpenProject}>프로젝트 확인 →</button>}
        </div>
        {recovery?.kind === 'guidance' && activeGuidance?.status === 'review_required' && (!activeGuidance.autoContinue || autoContinuationFailure) && <div ref={revisionRef}><RevisionForm key={activeGuidance.id} goal={goal} guidance={activeGuidance} onRevision={onRevision} /></div>}
      </section>}

      {deliverables.length > 0 && <section className="chat-context"><DeliverablePanel rows={deliverables} mode={deliverables.some(row => row.review) ? 'result' : 'plan'} />{goal.status === 'completed' && <p>산출물별 판정과 근거, 미확인 항목을 확인한 뒤 출시 준비의 다음 결정을 내려 주세요. 후속 작업은 새 목표로 전달합니다.</p>}</section>}
      {gitBootstrapRelevant && projectKind === 'folder' && <GitBootstrapPanel onPreview={onPreviewGit} onApply={onBootstrapGit} />}
      {gitBootstrapCompleted && projectKind === 'git' && <section className="chat-context" role="status"><strong>Git 저장소 초기화 완료</strong><p>{gitBootstrapCompleted.message}</p><p>{goal.status === 'validating' ? '커밋 파일이 원래 업무 범위와 일치해 독립 QA를 진행 중입니다.' : goal.status === 'completed' ? '원래 업무와 독립 QA까지 완료했습니다.' : '원래 업무와 파일 범위를 비교해 자동 QA 가능 여부를 확인했습니다. 목표 상태를 계속 확인해 주세요.'}</p>{gitBootstrapQaUnavailable && <p>QA 자동 진행 사유: {gitBootstrapQaUnavailable.message}</p>}</section>}

      {activeGuidance?.status === 'review_required' && !recovery && (!activeGuidance.autoContinue || autoContinuationFailure) && <section className="chat-context" ref={revisionRef}><RevisionForm key={activeGuidance.id} goal={goal} guidance={activeGuidance} onRevision={onRevision} /></section>}
      {sourceGuidance && <section className="chat-context" aria-label="이전 목표에서 이어받은 작업"><strong>이전 목표에서 이어받기</strong><p>CEO 지침을 바탕으로 {agents.find(agent => agent.id === sourceGuidance.addressedAgentId)?.name ?? '팀'} → CTO 영향 분석 → PM 재계획 순서로 검토합니다.</p>{sourceGuidance.workerHandoff && <ContentPreview><p>담당 팀원 인계 의견: {sourceGuidance.workerHandoff}</p></ContentPreview>}{sourceGuidance.continuation ? <p>{goal.status === 'completed' ? '재사용·QA 완료' : '파일 증거 확인 · QA 별도 검증'}: {sourceGuidance.continuation.reusedTaskIds.length}개 · 새 작업: {sourceGuidance.continuation.newTaskCount}개{sourceGuidance.continuation.reusedTaskIds.length > 0 ? ` · ${sourceGuidance.continuation.reusedTaskIds.map(id => tasks.find(task => task.id === id)?.title ?? id).join(', ')}` : ''}</p> : <small>분석 전입니다. 기존 결과를 아직 재사용 확정하지 않았습니다.</small>}<button type="button" className="text-button" onClick={() => onOpenGoal(sourceGuidance.goalId)}>이전 목표 보기 →</button></section>}
      {!recovery && replacementId && <section className="chat-context"><strong>새 목표 버전이 있습니다.</strong><button type="button" className="text-button" onClick={() => onOpenGoal(replacementId)}>새 목표 열기 →</button></section>}
      {autoContinuationFailure && !activeGuidance && <section className="chat-context" role="status"><strong>자동 진행을 끝내지 못했습니다.</strong><p>{autoContinuationFailure.message}</p>{replacementId && <button type="button" className="text-button" onClick={() => onOpenGoal(replacementId)}>새 목표에서 이어가기 →</button>}</section>}

      <details className="chat-progress"><summary>진행 정보 · 업무 {brief.owned.length}개 · QA {goal.reviewLoopsUsed}/{goal.reviewLoopLimit ?? 2}회 · 토큰 {brief.tokenTotal.toLocaleString()}개</summary><div className="chat-progress-body"><GoalGraph goal={goal} tasks={tasks} /><p>토큰: {brief.tokenTotal.toLocaleString()}개 {brief.estimated ? '· 추정 포함' : '· 제공자 보고 기반'}</p>{brief.latestEvent && <p>마지막 확인 기록 · {new Date(brief.latestEvent.at).toLocaleString()} · {brief.latestEvent.message}</p>}{brief.owned.length > 0 && <div className="chat-progress-tasks"><strong>담당 업무</strong>{taskWindow.visible.map(task => <p key={task.id}>{task.title} · {agents.find(agent => agent.id === task.assigneeId)?.name ?? task.assigneeId} · {taskStageLabel(task.status)} · {task.files.join(', ')}</p>)}<ListDisclosure label="업무" total={taskWindow.total} hidden={taskWindow.hidden} expanded={taskWindow.expanded} onToggle={taskWindow.toggle} /></div>}{qaReview && <ContentPreview><p>최근 QA 기록: {qaReview.message}</p></ContentPreview>}{goal.status === 'completed' && goal.report && <ContentPreview><pre className="report">{goal.report}</pre></ContentPreview>}</div></details>

      {history.total === 0 && <div className="chat-welcome"><strong>팀원에게 무엇을 말할까요?</strong><p>질문하거나 할 일을 자연스럽게 말해 주세요. 팀원이 의도를 파악해 답하거나 안전한 작업 흐름으로 연결합니다.</p><div className="chat-suggestions">{examples.map(example => <button type="button" className="secondary" key={example} onClick={() => { setDraft(example); focusComposer(); }}>{example}</button>)}</div></div>}
      <ListDisclosure label="이전 메시지" total={history.total} hidden={history.hidden} expanded={history.expanded} onToggle={history.toggle} />
      <div className="chat-timeline" aria-label="목표 대화와 지시 기록">{[...history.visible].reverse().map(item => item.kind === 'turn' ? <div className="chat-exchange" key={`turn:${item.id}`}>
        <div className="chat-bubble ceo"><div className="chat-bubble-meta"><strong>CEO → {item.turn.agentName || agents.find(agent => agent.id === item.turn.agentId)?.name || '팀원'} · 메시지</strong><time>{new Date(item.turn.askedAt).toLocaleString()}</time></div><ContentPreview><p>{item.turn.question}</p></ContentPreview></div>
        <div className="chat-bubble worker"><div className="chat-bubble-meta"><strong>{item.turn.route === 'direction' || item.turn.route === 'continue' ? `시스템 · ${item.turn.route === 'direction' ? '작업 지시 접수' : '실행 시작'}` : `${item.turn.agentName || '팀원'} · ${item.turn.status === 'pending' ? '의도 확인 중…' : item.turn.status === 'answered' ? '답변' : item.turn.status === 'interrupted' ? '중단됨' : '답변 실패'}`}</strong><small>{item.turn.route ? 'AI 해석' : 'AI 연결'}: {item.turn.connectionName}{item.turn.modelId ? ` · ${item.turn.modelId}` : ' · 기본 모델'}</small></div>{item.turn.answer ? <ContentPreview><p>{item.turn.answer}</p></ContentPreview> : <p>{item.turn.status === 'pending' ? '메시지를 확인하고 있습니다.' : '저장된 답변이 없습니다.'}</p>}</div>
      </div> : <div className="chat-exchange" key={`guidance:${item.id}`}>
        {!turns.some(turn => turn.guidanceId === item.id) && <div className="chat-bubble ceo"><div className="chat-bubble-meta"><strong>CEO → {agents.find(agent => agent.id === item.guidance.addressedAgentId)?.name ?? '팀'} · 목표 변경 지시</strong><time>{new Date(item.guidance.createdAt).toLocaleString()}</time></div><ContentPreview><p>{item.guidance.text}</p></ContentPreview></div>}
        <div className="chat-bubble worker chat-guidance-status" role="status"><strong>{item.guidance.status === 'pending' ? '작업 지시 접수 · 안전 지점 대기' : item.guidance.status === 'review_required' ? item.guidance.autoContinue && !autoContinuationFailure ? '안전 지점에서 멈춤 · 새 목표 자동 준비 중' : '안전 지점에서 멈춤 · 새 목표 검토 필요' : '새 목표 버전으로 연결됨'}</strong><p>{item.guidance.status === 'pending' ? '현재 실행 중인 호출이 끝나면 기존 목표를 멈춥니다. 담당 팀원의 인계는 새 목표 실행 때 시작합니다.' : item.guidance.status === 'review_required' ? item.guidance.autoContinue && !autoContinuationFailure ? `멈춘 지점: ${item.guidance.checkpoint ?? '확인 필요'}. CTO·PM 재계획을 자동으로 이어갑니다.` : `멈춘 지점: ${item.guidance.checkpoint ?? '확인 필요'}. 위의 새 목표 내용을 검토해 주세요.` : '기존 작업과 승인은 기록으로 남습니다. 새 목표에서 CTO·PM이 영향 범위를 확인합니다.'}</p>{item.guidance.replacementGoalId && <button type="button" className="text-button" onClick={() => onOpenGoal(item.guidance.replacementGoalId!)}>새 목표 열기 →</button>}</div>
      </div>)}</div>
    </div>

    <div className="chat-composer"><div className="chat-composer-top"><strong>팀원에게 메시지</strong><label>받는 팀원 <select value={selectedAgentId} onChange={event => setSelectedAgentId(event.target.value)}>{routable.length ? routable.map(agent => <option value={agent.id} key={agent.id}>{agent.name} · {agent.role}{eligible.some(ready => ready.id === agent.id) ? '' : ' · 연결 확인 필요'}</option>) : <option value="">활성 팀원 없음</option>}</select></label></div>
      <label className="chat-input-label" htmlFor="goal-chat-input">메시지</label><textarea id="goal-chat-input" ref={inputRef} value={draft} maxLength={4000} placeholder={focusTask ? '이 업무에 관해 묻거나, 다음에 할 일을 말해 주세요.' : '팀원에게 질문하거나 원하는 일을 말해 주세요.'} onChange={event => setDraft(event.target.value)} />
      <div className="chat-composer-bottom"><small>{!selectedAgent ? '선택한 팀원의 AI 연결을 확인해 주세요.' : pending ? '이전 메시지의 AI 해석이 끝나면 다음 메시지를 보낼 수 있습니다.' : activeGuidance ? '이전 지침이 처리 중입니다. 질문은 계속할 수 있습니다.' : replacementId ? '후속 작업 지시는 이어진 새 목표에서 보내 주세요.' : goal.status === 'completed' ? '완료된 목표의 후속 작업은 새 목표가 필요합니다.' : '명확한 작업 지시는 안전 지점에서 재계획합니다. 새 파일 작업은 별도 승인을 요청합니다.'}</small><div className="chat-send-actions">{!selectedAgent && <button type="button" className="secondary" onClick={onOpenConnections}>AI 연결 확인 →</button>}<button type="button" className="primary" disabled={sending || pending || !selectedAgent || draft.trim().length < 4} onClick={() => void sendMessage()}>전송 →</button></div></div>
    </div>
  </section></div>;
}
