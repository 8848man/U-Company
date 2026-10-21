import { useEffect, useRef, useState } from 'react';
import { ContentPreview, ListDisclosure, useLimitedItems } from './ContentDensity';
import { GoalGraph } from './OfficeActivity';
import { GitBootstrapPanel } from './GitBootstrapPanel';
import type { GitBootstrapPreview, GitBootstrapResult } from './GitBootstrapPanel';
import { goalBrief, goalStageLabel, taskStageLabel } from './goalBrief';
import { goalRecovery } from './goalRecovery';
import { runtimeRecovery } from './providerRecovery';
import { goalTitle } from './progress';
import { blockFacts, deliverableRows } from './deliverables';
import { BlockFactsView, DeliverablePanel } from './DeliverablePanel';
import { ArchiveGoalButton } from './ArchiveGoalButton';
import { SquadPanel } from './SquadPanel';
import { PlanningOriginPanel } from './PlanningOriginPanel';
import { planCardLabel, planCards, progressLine, stageLines, type PlanCard } from './goalStream';
import type { GoalOrigin } from './sddOrigin';
import type { Agent, AiConnection, Approval, DeliverableReview, Event, Goal, GoalGuidance, GoalTurn, Project, Squad, Task, TokenUsageRecord } from './types';

type RunBlock = { title: string; reason: string; next: string } | null;

type Props = {
  goal: Goal; projectKind: string; focusTaskId?: string | null; turns: GoalTurn[]; guidance: GoalGuidance[];
  agents: Agent[]; connections: AiConnection[]; tasks: Task[]; approvals: Approval[];
  events: Event[]; usage: TokenUsageRecord[]; reviews: DeliverableReview[]; runBlock: RunBlock;
  squad: Squad | null; squads: Squad[] | undefined; summary: string | null;
  goalOrigins: GoalOrigin[] | undefined; projects: Project[];
  /** `dialog` floats over the page (M1); `panel` fills its container (home, M2). */
  layout?: 'dialog' | 'panel';
  onClose: () => void; onSend: (message: string, agentId: string) => Promise<boolean>;
  onArchive?: () => Promise<boolean>;
  onRevision: (id: string, intent: string) => Promise<boolean>;
  onOpenGoal: (id: string) => void;
  onRetry: () => Promise<boolean>; onResume: () => Promise<boolean>;
  onDecide: (approvalId: string, approved: boolean, squadDigest: string | null) => Promise<boolean>;
  /** ADR-0046 M3: `개선` (same goal, new plan) and `거부` (cancel the goal); `다시 시작` sends the same goal again. */
  onRevise: (approvalId: string, note: string) => Promise<boolean>; onCancel: (approvalId: string) => Promise<boolean>; onRestart: (intent: string) => Promise<boolean>;
  onSwapSquad: (from: string, to: string) => void; onAddSquad: (slot: 'writer' | 'reviewer', agentId: string, taskId: string | null) => void;
  onOpenConnections: () => void; onOpenOffice: () => void; onOpenTeam: () => void; onOpenProject: () => void;
  onError: (message: string) => void;
  onPreviewGit: () => Promise<GitBootstrapPreview | undefined>;
  onBootstrapGit: (preview: GitBootstrapPreview, selectedFiles: string[], authorName: string, authorEmail: string, message: string) => Promise<GitBootstrapResult | undefined>;
};

function RevisionForm({ goal, guidance, onRevision }: { goal: Goal; guidance: GoalGuidance; onRevision: Props['onRevision'] }) {
  const [intent, setIntent] = useState(`${goal.intent}\n\nCEO 변경 지침: ${guidance.text}`);
  const [sending, setSending] = useState(false);
  return <div className="goal-revision-form"><strong>새 목표 내용 확인</strong><p>새 목표는 팀이 다시 계획합니다. 새 파일 작업이 있으면 변경 범위를 승인받습니다.</p><textarea aria-label="새 목표 내용" value={intent} maxLength={12000} onChange={event => setIntent(event.target.value)} /><button type="button" className="primary" disabled={sending || intent.trim().length < 10} onClick={() => { setSending(true); void onRevision(guidance.id, intent).finally(() => setSending(false)); }}>새 목표 버전 만들기</button></div>;
}

type StreamItem =
  | { kind: 'turn'; id: string; at: string; turn: GoalTurn }
  | { kind: 'guidance'; id: string; at: string; guidance: GoalGuidance }
  | { kind: 'line'; id: string; at: string; text: string }
  | { kind: 'plan'; id: string; at: string; card: PlanCard };

/** v3 goal conversation (ADR-0046): the goal, stage lines, plan cards and recovery in one stream, with a progress line. */
export function GoalChat(props: Props) {
  const { goal, projectKind, focusTaskId = null, turns, guidance, agents, connections, tasks, approvals, events, usage, reviews, runBlock, squad, squads, summary, goalOrigins, projects, layout = 'dialog', onClose, onSend, onArchive, onRevision, onOpenGoal, onRetry, onResume, onDecide, onRevise, onCancel, onRestart, onSwapSquad, onAddSquad, onOpenConnections, onOpenOffice, onOpenTeam, onOpenProject, onError, onPreviewGit, onBootstrapGit } = props;
  const focusTask = tasks.find(task => task.id === focusTaskId && task.goalId === goal.id);
  const [draft, setDraft] = useState(focusTask ? `"${focusTask.title.slice(0, 120)}" 업무는 지금 어디까지 했고, 막힌 점이 있나요? 제 결정이 필요하면 알려 주세요.` : '');
  const [improving, setImproving] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [working, setWorking] = useState(false);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const revisionRef = useRef<HTMLDivElement>(null);
  const feedRef = useRef<HTMLDivElement>(null);
  const eligible = agents.filter(agent => agent.enabled && connections.some(connection => connection.id === agent.connectionId && !['missing', 'failed', 'needs_setup'].includes(connection.status)));
  const routable = agents.filter(agent => agent.enabled);
  const [selectedAgentId, setSelectedAgentId] = useState(() => routable.find(agent => agent.id === focusTask?.assigneeId)?.id ?? eligible.find(agent => agent.id === 'cto')?.id ?? eligible[0]?.id ?? routable[0]?.id ?? '');
  const selectedAgent = eligible.find(agent => agent.id === selectedAgentId);
  const pending = turns.some(turn => turn.status === 'pending');
  const activeGuidance = guidance.find(item => item.goalId === goal.id && (item.status === 'pending' || item.status === 'review_required'));
  const autoContinuationFailure = events.find(event => event.goalId === goal.id && event.kind === 'goal_auto_continuation_failed');
  const brief = goalBrief(goal, tasks, approvals, guidance, agents, events, usage);
  const recovery = goalRecovery(goal, tasks, approvals, guidance);
  const facts = blockFacts(goal, tasks, approvals, guidance, agents, events);
  const deliverables = deliverableRows(goal, tasks, reviews, agents);
  const providerHint = goal.status === 'blocked' ? runtimeRecovery(goal.report) : null;
  const sourceGuidance = guidance.find(item => item.replacementGoalId === goal.id);
  const replacementId = guidance.find(item => item.goalId === goal.id && item.status === 'revised' && item.replacementGoalId)?.replacementGoalId;
  const gitEvidence = [goal.intent, goal.report, ...tasks.filter(task => task.goalId === goal.id).flatMap(task => [task.title, task.result]), ...turns.map(turn => turn.answer)].join(' ');
  const gitBootstrapRelevant = (goal.status === 'blocked' && /\.git|git init|git repository|첫 커밋|초기 커밋|깃 저장소|Git 저장소/i.test(gitEvidence)) || turns.some(turn => turn.route === 'git_bootstrap');
  const gitBootstrapCompleted = events.find(event => event.goalId === goal.id && event.kind === 'git_bootstrap_completed');
  const qaReview = events.find(event => event.goalId === goal.id && event.kind === 'qa_review');
  const progress = progressLine(goal, tasks, agents, events, squad);
  const cards = planCards(goal, approvals, tasks, summary);
  const ownedTasks = tasks.filter(task => task.goalId === goal.id && task.status !== 'superseded');
  const taskWindow = useLimitedItems(ownedTasks);
  // The decision that needs the CEO now, outside the plan card: start, recovery or a guidance review.
  const decision = recovery && recovery.kind !== 'approval' ? recovery : null;

  const stream: StreamItem[] = [
    ...turns.map(turn => ({ kind: 'turn' as const, id: `turn:${turn.id}`, at: turn.askedAt, turn })),
    ...guidance.filter(item => item.goalId === goal.id).map(item => ({ kind: 'guidance' as const, id: `guidance:${item.id}`, at: item.createdAt, guidance: item })),
    ...stageLines(goal, events).map(line => ({ kind: 'line' as const, ...line })),
    ...cards.map(card => ({ kind: 'plan' as const, id: `plan:${card.approval.id}`, at: card.approval.createdAt, card })),
  ].sort((a, b) => a.at.localeCompare(b.at) || a.id.localeCompare(b.id));
  const history = useLimitedItems([...stream].reverse());
  // Open at the latest entry, and follow new entries, like a chat.
  useEffect(() => { feedRef.current?.scrollTo({ top: feedRef.current.scrollHeight }); }, [stream.length, goal.status]);

  function focusComposer() { inputRef.current?.focus(); }
  function scrollDown() { requestAnimationFrame(() => feedRef.current?.scrollTo({ top: feedRef.current.scrollHeight, behavior: 'smooth' })); }
  async function guarded(operation: () => Promise<boolean>) {
    if (working) return;
    setWorking(true);
    try { await operation(); } finally { setWorking(false); }
  }
  async function decide(card: PlanCard, approved: boolean) {
    await guarded(() => approved ? onDecide(card.approval.id, true, squad?.digest ?? null) : onCancel(card.approval.id));
    scrollDown();
  }
  function startImproving(card: PlanCard) { setImproving(card.approval.id); setDraft(''); requestAnimationFrame(focusComposer); }
  async function sendMessage() {
    const text = draft.trim();
    if (sending || text.length < 4 || text.length > 4000 || pending || (!improving && !selectedAgent)) return;
    setSending(true);
    try {
      // `개선` is its own command: the note goes to the PM of this goal, never through AI interpretation.
      const ok = improving ? await onRevise(improving, text) : await onSend(text, selectedAgent!.id);
      if (ok) { setDraft(''); setImproving(null); scrollDown(); }
    } finally { setSending(false); }
  }

  function planCardView(card: PlanCard) {
    if (!card.current) return <div className="gc-plan is-closed" key={card.approval.id}><strong>{planCardLabel(card)}</strong>{card.counts && <span> · {card.counts}</span>}<details><summary>승인 범위 보기</summary><ContentPreview><pre className="report">{card.approval.reason}</pre></ContentPreview></details></div>;
    return <section className="gc-plan" key={card.approval.id} aria-label="계획 확인">
      <h3>{planCardLabel(card)}</h3>
      {card.summary && <p>{card.summary}</p>}
      {card.counts && <p className="gc-plan-counts">{card.counts}</p>}
      <details className="gc-plan-detail"><summary>작업과 팀 자세히 보기</summary>
        {deliverables.length > 0 && <DeliverablePanel rows={deliverables} mode="plan" />}
        {squad && <SquadPanel squad={squad} agents={agents} connections={connections} tasks={tasks} editable onSwap={onSwapSquad} onAdd={onAddSquad} />}
        <p className="muted">승인 범위 전문</p><ContentPreview><pre className="report">{card.approval.reason}</pre></ContentPreview>
      </details>
      <small className="muted">승인 전에는 프로젝트 파일을 바꾸지 않아요 · 승인은 이 버튼으로만 해요</small>
      <div className="gc-plan-actions">
        <button type="button" className={improving ? 'secondary' : 'primary'} disabled={working} onClick={() => void decide(card, true)}>승인</button>
        <button type="button" className="secondary" disabled={working} onClick={() => startImproving(card)}>개선</button>
        <button type="button" className="secondary" disabled={working} onClick={() => void decide(card, false)}>거부</button>
      </div>
      <div className="gc-plan-extra"><PlanningOriginPanel goalId={goal.id} goalOrigins={goalOrigins} projects={projects} squads={squads} onOpenConnections={onOpenConnections} onError={onError} /></div>
    </section>;
  }

  function turnView(turn: GoalTurn) {
    return <div className="chat-exchange" key={`turn:${turn.id}`}>
      <div className="chat-bubble ceo"><div className="chat-bubble-meta"><strong>{turn.route === 'plan_revision' ? 'CEO · 계획 개선 방침' : `CEO → ${turn.agentName || agents.find(agent => agent.id === turn.agentId)?.name || '팀원'}`}</strong><time>{new Date(turn.askedAt).toLocaleString()}</time></div><ContentPreview><p>{turn.question}</p></ContentPreview></div>
      <div className="chat-bubble worker"><div className="chat-bubble-meta"><strong>{turn.route === 'plan_revision' ? '시스템 · 계획 다시 세우기' : turn.route === 'direction' || turn.route === 'continue' ? `시스템 · ${turn.route === 'direction' ? '작업 지시 접수' : '실행 시작'}` : `${turn.agentName || '팀원'} · ${turn.status === 'pending' ? '의도 확인 중…' : turn.status === 'answered' ? '답변' : turn.status === 'interrupted' ? '중단됨' : '답변 실패'}`}</strong></div>{turn.answer ? <ContentPreview><p>{turn.answer}</p></ContentPreview> : <p>{turn.status === 'pending' ? '메시지를 확인하고 있습니다.' : '저장된 답변이 없습니다.'}</p>}</div>
    </div>;
  }

  function guidanceView(item: GoalGuidance) {
    return <div className="chat-exchange" key={`guidance:${item.id}`}>
      {!turns.some(turn => turn.guidanceId === item.id) && <div className="chat-bubble ceo"><div className="chat-bubble-meta"><strong>CEO · 목표 변경 지시</strong><time>{new Date(item.createdAt).toLocaleString()}</time></div><ContentPreview><p>{item.text}</p></ContentPreview></div>}
      <div className="chat-bubble worker chat-guidance-status" role="status"><strong>{item.status === 'pending' ? '작업 지시 접수 · 안전 지점 대기' : item.status === 'review_required' ? item.autoContinue && !autoContinuationFailure ? '안전 지점에서 멈춤 · 새 목표 자동 준비 중' : '안전 지점에서 멈춤 · 새 목표 검토 필요' : '새 목표 버전으로 연결됨'}</strong>{item.replacementGoalId && <button type="button" className="text-button" onClick={() => onOpenGoal(item.replacementGoalId!)}>새 목표 열기 →</button>}</div>
    </div>;
  }

  const body = <section className={layout === 'dialog' ? 'goal-conversation chat-workspace gc' : 'chat-workspace gc gc-panel'} role={layout === 'dialog' ? 'dialog' : 'region'} aria-modal={layout === 'dialog' ? true : undefined} aria-labelledby="goal-chat-title" onClick={event => event.stopPropagation()}>
    <header className="chat-workspace-header"><div><span className="card-kicker">{goalStageLabel(goal.status)}</span><h2 id="goal-chat-title" title={goal.intent}>{goalTitle(goal, 105)}</h2></div><div className="button-row"><button type="button" className="secondary" onClick={onOpenOffice}>Office ↗</button>{onArchive && <ArchiveGoalButton goal={goal} tasks={tasks} onArchive={onArchive} />}{layout === 'dialog' && <button type="button" className="secondary" onClick={onClose} aria-label="목표 대화 닫기">닫기</button>}</div></header>
    <div className="chat-workspace-feed" ref={feedRef}>
      <ListDisclosure label="이전 기록" total={history.total} hidden={history.hidden} expanded={history.expanded} onToggle={history.toggle} />
      {history.hidden === 0 && <div className="chat-bubble ceo gc-goal"><div className="chat-bubble-meta"><strong>CEO · 목표</strong><time>{new Date(goal.createdAt).toLocaleString()}</time></div><ContentPreview><p>{goal.intent}</p></ContentPreview></div>}
      {sourceGuidance && <section className="chat-context" aria-label="이전 목표에서 이어받은 작업"><strong>이전 목표에서 이어받기</strong>{sourceGuidance.continuation ? <p>재사용 {sourceGuidance.continuation.reusedTaskIds.length}개 · 새 작업 {sourceGuidance.continuation.newTaskCount}개</p> : <small>분석 전입니다. 기존 결과를 아직 재사용 확정하지 않았습니다.</small>}<button type="button" className="text-button" onClick={() => onOpenGoal(sourceGuidance.goalId)}>이전 목표 보기 →</button></section>}
      <div className="chat-timeline" aria-label="목표 대화와 진행 기록">{[...history.visible].reverse().map(item => item.kind === 'turn' ? turnView(item.turn) : item.kind === 'guidance' ? guidanceView(item.guidance) : item.kind === 'line' ? <p className="gc-line" key={item.id}>{item.text}</p> : planCardView(item.card))}</div>

      {goal.status === 'new' && <section className="chat-decision" aria-label="지금 필요한 행동"><span className="card-kicker">지금 할 일</span><h3>{runBlock ? runBlock.title : '팀 실행 시작'}</h3><p>{runBlock ? `${runBlock.reason} ${runBlock.next}` : '목표를 팀에 전달했습니다. 시작하면 AI 팀이 분석합니다.'}</p><div className="chat-decision-actions"><button type="button" className="primary" disabled={working || Boolean(runBlock)} onClick={() => void guarded(onRetry)}>팀 실행 시작 →</button>{runBlock && <button type="button" className="secondary" onClick={onOpenConnections}>AI 연결 열기 →</button>}{runBlock && <button type="button" className="secondary" onClick={onOpenTeam}>팀 설정 확인 →</button>}</div></section>}
      {decision && <section className="chat-decision" aria-label="지금 필요한 행동"><span className="card-kicker">지금 할 일</span><h3>{decision.label}</h3>
        {facts && <BlockFactsView facts={facts} />}
        {goal.status === 'blocked' && <ContentPreview><pre className="report">{goal.report || '저장된 중단 사유가 없습니다.'}</pre></ContentPreview>}
        <p>{decision.explanation}</p>
        {providerHint && <p className="chat-decision-hint">AI 연결: {providerHint}</p>}
        {runBlock && (decision.kind === 'retry' || decision.kind === 'resume') && <p className="chat-decision-hint">실행 전 확인: {runBlock.title} · {runBlock.reason} {runBlock.next}</p>}
        <div className="chat-decision-actions">
          {decision.kind === 'retry' && <button type="button" className="primary" disabled={working || Boolean(runBlock)} onClick={() => void guarded(onRetry)}>분석 다시 실행 →</button>}
          {decision.kind === 'resume' && <button type="button" className="primary" disabled={working || Boolean(runBlock)} onClick={() => void guarded(onResume)}>승인된 계획 실행 →</button>}
          {decision.kind === 'revision' && <button type="button" className="primary" onClick={focusComposer}>팀원에게 해결 요청 →</button>}
          {decision.kind === 'guidance' && activeGuidance?.status === 'review_required' && (!activeGuidance.autoContinue || autoContinuationFailure) && <button type="button" className="primary" onClick={() => revisionRef.current?.scrollIntoView({ block: 'nearest' })}>새 목표 내용 검토 →</button>}
          {decision.kind === 'replacement' && replacementId && <button type="button" className="primary" onClick={() => onOpenGoal(replacementId)}>새 목표 열기 →</button>}
          {providerHint && <button type="button" className="secondary" onClick={onOpenConnections}>AI 연결 열기 →</button>}
          {decision.kind === 'revision' && <button type="button" className="secondary" onClick={onOpenProject}>프로젝트 확인 →</button>}
        </div>
        {decision.kind === 'guidance' && activeGuidance?.status === 'review_required' && (!activeGuidance.autoContinue || autoContinuationFailure) && <div ref={revisionRef}><RevisionForm key={activeGuidance.id} goal={goal} guidance={activeGuidance} onRevision={onRevision} /></div>}
      </section>}
      {goal.status === 'cancelled' && <section className="chat-context gc-cancelled" aria-label="취소됨"><strong>이 목표를 취소했어요</strong><p>계획을 거부해 프로젝트 파일은 바뀌지 않았어요. 같은 내용으로 다시 맡기려면 다시 시작하세요.</p><div className="chat-decision-actions"><button type="button" className="secondary" disabled={working} onClick={() => void guarded(() => onRestart(goal.intent))}>같은 내용으로 다시 시작</button></div></section>}
      {goal.status === 'completed' && <section className="chat-context gc-result" aria-label="결과"><strong>결과</strong>{deliverables.length > 0 && <DeliverablePanel rows={deliverables} mode="result" />}{goal.report && <ContentPreview><pre className="report">{goal.report}</pre></ContentPreview>}<p>후속 작업은 새 목표로 보내 주세요.</p></section>}
      {gitBootstrapRelevant && projectKind === 'folder' && <GitBootstrapPanel onPreview={onPreviewGit} onApply={onBootstrapGit} />}
      {gitBootstrapCompleted && projectKind === 'git' && <section className="chat-context" role="status"><strong>Git 저장소 초기화 완료</strong><p>{gitBootstrapCompleted.message}</p></section>}
      {autoContinuationFailure && !activeGuidance && <section className="chat-context" role="status"><strong>자동 진행을 끝내지 못했습니다.</strong><p>{autoContinuationFailure.message}</p>{replacementId && <button type="button" className="text-button" onClick={() => onOpenGoal(replacementId)}>새 목표에서 이어가기 →</button>}</section>}

      <details className="chat-progress"><summary>진행 정보 · 업무 {ownedTasks.length}개 · QA {goal.reviewLoopsUsed}/{goal.reviewLoopLimit ?? 2}회 · 토큰 {brief.tokenTotal.toLocaleString()}개</summary><div className="chat-progress-body"><GoalGraph goal={goal} tasks={tasks} />{brief.latestEvent && <p>마지막 확인 기록 · {new Date(brief.latestEvent.at).toLocaleString()} · {brief.latestEvent.message}</p>}{ownedTasks.length > 0 && <div className="chat-progress-tasks"><strong>업무</strong>{taskWindow.visible.map(task => <p key={task.id}>{task.title} · {agents.find(agent => agent.id === task.assigneeId)?.name ?? '배정 전'} · {taskStageLabel(task.status)}</p>)}<ListDisclosure label="업무" total={taskWindow.total} hidden={taskWindow.hidden} expanded={taskWindow.expanded} onToggle={taskWindow.toggle} /></div>}{qaReview && <ContentPreview><p>최근 QA 기록: {qaReview.message}</p></ContentPreview>}</div></details>
    </div>

    {progress && <button type="button" className="gc-progress" onClick={onOpenOffice} aria-label={`진행 중: ${progress}. Office 열기`}><span className="gc-progress-dot" aria-hidden="true" />{progress}</button>}
    <div className="chat-composer"><div className="chat-composer-top"><strong>{improving ? '계획을 어떻게 고칠까요?' : '팀원에게 메시지'}</strong>{improving ? <button type="button" className="text-button" onClick={() => { setImproving(null); setDraft(''); }}>개선 취소</button> : <label>받는 팀원 <select value={selectedAgentId} onChange={event => setSelectedAgentId(event.target.value)}>{routable.length ? routable.map(agent => <option value={agent.id} key={agent.id}>{agent.name} · {agent.role}{eligible.some(ready => ready.id === agent.id) ? '' : ' · 연결 확인 필요'}</option>) : <option value="">활성 팀원 없음</option>}</select></label>}</div>
      <label className="chat-input-label" htmlFor="goal-chat-input">메시지</label><textarea id="goal-chat-input" ref={inputRef} value={draft} maxLength={4000} placeholder={improving ? '예: 파일을 하나로 합쳐 주세요. 시험도 같이 추가해 주세요.' : '팀원에게 질문하거나 원하는 일을 말해 주세요.'} onChange={event => setDraft(event.target.value)} onKeyDown={event => { if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) void sendMessage(); }} />
      <div className="chat-composer-bottom"><small>{improving ? '보낸 방침으로 팀이 같은 목표에서 계획을 다시 세웁니다. 새 계획은 다시 승인받습니다.' : goal.status === 'cancelled' ? '취소한 목표예요. 새 일은 새 목표로 보내 주세요.' : !selectedAgent ? '선택한 팀원의 AI 연결을 확인해 주세요.' : pending ? '이전 메시지의 AI 해석이 끝나면 다음 메시지를 보낼 수 있습니다.' : goal.status === 'completed' ? '완료된 목표의 후속 작업은 새 목표가 필요합니다.' : '대화는 승인이 아니에요. 새 파일 작업은 계획 카드에서 승인해요.'}</small><div className="chat-send-actions">{!selectedAgent && <button type="button" className="secondary" onClick={onOpenConnections}>AI 연결 확인 →</button>}<button type="button" className={cards.some(card => card.current) && !improving ? 'secondary' : 'primary'} disabled={sending || pending || (!improving && !selectedAgent) || draft.trim().length < 4} onClick={() => void sendMessage()}>{improving ? '개선 방침 보내기' : '전송'}</button></div></div>
    </div>
  </section>;
  return layout === 'dialog' ? <div className="dialog-backdrop" onClick={onClose}>{body}</div> : body;
}
