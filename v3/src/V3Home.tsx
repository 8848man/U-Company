import { useState } from 'react';
import { GoalGraph } from '../../src/OfficeActivity';
import { ListDisclosure, useLimitedItems } from '../../src/ContentDensity';
import { goalTitle } from '../../src/progress';
import type { Agent, AiConnection, Approval, DeliverableReview, Event, Goal, GoalGuidance, MeetingReadiness, Project, Task } from '../../src/types';
import { blockFacts, deliverableRows } from '../../src/deliverables';
import { BlockFactsView } from '../../src/DeliverablePanel';
import { profileLabel } from '../../src/MeetingPanel';
import type { MeetingAttention } from '../../src/meetingStatus';

type BlockReason = { title: string; reason: string; next: string };
export type HomeMode = 'goal' | 'meeting';

/** First-pilot example (v3 plan): local material only, three named deliverables, no web/outreach. */
export const LAUNCH_PILOT_EXAMPLE = 'notes/ 폴더의 자료만 사용해서 출시 준비 문서 세 가지를 launch/ 폴더에 만들어 줘: (1) launch/customer-hypothesis.md 고객·문제 가설 (2) launch/offer-message.md 제안·메시지 초안 (3) launch/prelaunch-checklist.md 출시 전 확인 목록. 자료로 확인되지 않은 시장·고객 주장은 \'미확인\'으로 표시하고, 웹 조사·외부 게시·고객 연락은 하지 마.';

type Props = {
  project: Project | null; goals: Goal[]; tasks: Task[]; approvals: Approval[]; agents: Agent[]; connections: AiConnection[];
  guidance: GoalGuidance[]; events: Event[]; reviews: DeliverableReview[];
  mode: HomeMode; onMode: (mode: HomeMode) => void; readiness: MeetingReadiness | null; readinessError: string; onStartMeeting: () => void;
  draftSource: string | null; onClearDraftSource: () => void; meetingStatus: (goal: Goal) => MeetingAttention;
  intent: string; busy: boolean; teamBlock: BlockReason | null; runBlock: (goal: Goal) => BlockReason | null;
  onIntent: (value: string) => void; onSubmit: () => void; onRun: (id: string) => void; onOpen: (id: string) => void;
  onSettings: () => void; onConnections: () => void; onOffice: () => void; onExplain: (title: string, reason: string, next: string) => void;
  /** ADR-0046: the chat home lists goals on the left, so this pane shows only the new-goal box and decisions. */
  hideGoalList?: boolean;
};

function stageLabel(goal: Goal): string {
  return ({ new: '시작 전', analyzing: '목표 분석', planning: '산출물 계획', awaiting_approval: 'CEO 승인 대기', executing: '팀 작업 중', integrating: '결과 합치는 중', validating: 'QA 검증', completed: '인수 확인', blocked: '확인 필요' } as Record<string, string>)[goal.status] ?? goal.status;
}

/** Home of the v3 shell: readiness, one command box, a decision inbox and per-deliverable goal cards. */
export function V3Home(props: Props) {
  const { project, goals, tasks, approvals, agents, connections, guidance, events, reviews, mode, onMode, readiness, readinessError, onStartMeeting, draftSource, onClearDraftSource, meetingStatus, intent, busy, teamBlock, runBlock, onIntent, onSubmit, onRun, onOpen, onSettings, onConnections, onOffice, onExplain, hideGoalList = false } = props;
  const [showGraph, setShowGraph] = useState<string | null>(null);
  const decisions = goals.filter(goal => goal.kind === 'deliberation' ? meetingStatus(goal).needsDecision : Boolean(blockFacts(goal, tasks, approvals, guidance, agents, events)));
  const ordered = [...goals].sort((a, b) => Number(decisions.includes(b)) - Number(decisions.includes(a)) || b.createdAt.localeCompare(a.createdAt));
  const window = useLimitedItems(ordered);
  const inbox = useLimitedItems(decisions);
  const readyConnections = connections.filter(item => !['missing', 'failed', 'needs_setup'].includes(item.status)).length;

  if (!project) return <div className="page dashboard"><div className="v2-heading"><div><div className="eyebrow">CEO DESK · v3</div><h1>작업할 폴더를 골라 주세요</h1><p>출시 준비 자료가 있는 로컬 폴더를 고르면 팀에 목표를 전할 수 있습니다. 폴더를 등록해도 파일은 바뀌지 않습니다.</p></div><button className="primary" onClick={onSettings}>폴더 선택</button></div></div>;

  return <div className="page dashboard"><div className="v2-heading"><div><div className="eyebrow">CEO DESK · v3</div><h1>{project.name}의 오늘 할 일</h1><p>목표를 한 문장으로 전하면 팀이 산출물·담당·변경 파일을 한 계획 카드로 보여 줍니다. 승인한 범위만 실행합니다.</p></div></div>
    <div className="v2-stack">
      {teamBlock && <section className="card v3-setup" aria-label="시작 전 준비"><strong>시작 전 준비 · {teamBlock.title}</strong><small>{teamBlock.reason}</small><small>AI 연결 {readyConnections}개 사용 가능 · {teamBlock.next}</small><div className="button-row"><button className="primary" onClick={onConnections}>AI 연결 확인</button><button className="secondary" onClick={onOffice}>팀 확인</button></div></section>}

      <section className="card command-card"><div className="v2-row"><div className="card-kicker">{mode === 'goal' ? '새 목표' : '새 회의'}</div><label className="home-mode">명령 방식<select aria-label="명령 방식" value={mode} onChange={event => onMode(event.target.value as HomeMode)}><option value="goal">목표 수행</option><option value="meeting">회의</option></select></label></div>
        <h2>{mode === 'goal' ? '팀에 무엇을 맡길까요?' : '팀이 무엇을 먼저 토론할까요?'}</h2>
        <textarea aria-label={mode === 'goal' ? '새 목표' : '회의 안건'} value={intent} onChange={event => onIntent(event.target.value)} placeholder={mode === 'goal' ? '만들 결과, 사용할 자료, 지켜야 할 조건을 한두 문장으로 적어 주세요.' : '결정할 질문과 기대하는 결과를 적어 주세요. 의장이 목표·제한·참여자를 제안합니다.'}/>
        {mode === 'goal' && !intent.trim() && <div className="v3-example"><small>예시 · 첫 파일럿: 로컬 자료로 출시 준비 문서 세 가지</small><button type="button" className="text-button" onClick={() => onIntent(LAUNCH_PILOT_EXAMPLE)}>출시 준비 예시 넣기 →</button></div>}
        {draftSource && (mode === 'goal' ? <p className="meeting-draft-note" role="note">회의 결정 자료 {draftSource.slice(0, 12)}…에서 가져온 <strong>초안</strong>입니다. 고친 뒤 직접 전달해야 목표가 만들어지며, 파일 변경은 이후 별도 승인을 받습니다. <button className="text-button" onClick={onClearDraftSource}>출처 연결 해제</button></p> : <p className="meeting-draft-note" role="note">회의 결정 초안의 출처 연결은 <strong>목표 수행</strong>으로 전달할 때만 기록됩니다.</p>)}
        {mode === 'meeting' && <div className="meeting-disclosure" aria-live="polite">{readiness?.chair ? <><strong>의장 {readiness.chair.name} ({readiness.chair.role})</strong><small>{readiness.chair.connectionName} · {readiness.chair.modelId || '기본 모델'} · {profileLabel[readiness.chair.profile] ?? readiness.chair.profile}</small><small>안건이 이 AI에 전송되고, AI는 프로젝트 폴더를 읽을 수 있으며 파일은 바꾸지 않습니다.</small></> : <><strong>회의를 시작할 수 없습니다</strong><small>{readinessError || readiness?.chairReason || '의장 준비 상태를 확인하는 중입니다.'}</small></>}</div>}
        <div className="card-actions"><span>작업할 곳: {project.name}{mode === 'goal' ? ' · 다음: 팀이 산출물 계획 → CEO 승인' : ''}</span>{mode === 'goal'
          ? <button className={busy || intent.trim().length < 10 ? 'primary is-blocked' : 'primary'} onClick={() => busy ? onExplain('잠시 기다려 주세요', '다른 작업을 처리하고 있습니다.', '끝난 뒤 다시 눌러 주세요.') : intent.trim().length < 10 ? onExplain('목표를 조금 더 적어 주세요', '원하는 결과를 판단할 정보가 부족합니다.', '만들 결과와 사용할 자료를 한 문장 이상 적어 주세요.') : onSubmit()}>목표 전달 ↗</button>
          : <button className={busy || intent.trim().length < 10 || !readiness?.chair ? 'primary is-blocked' : 'primary'} onClick={() => busy ? onExplain('잠시 기다려 주세요', '다른 작업을 처리하고 있습니다.', '끝난 뒤 다시 눌러 주세요.') : !readiness?.chair ? onExplain('의장을 준비해 주세요', readinessError || readiness?.chairReason || '의장 상태를 확인하지 못했습니다.', 'PM 또는 CTO에게 Claude/Codex CLI 연결을 배정하세요.') : intent.trim().length < 10 ? onExplain('안건을 조금 더 적어 주세요', '의장이 계획을 세울 정보가 부족합니다.', '결정할 질문과 조건을 한 문장 이상 적어 주세요.') : onStartMeeting()}>회의 시작 ↗</button>}</div>
      </section>

      {decisions.length > 0 && <section className="card v3-inbox" aria-label="결정할 일"><div className="v2-row"><strong>결정할 일 {decisions.length}건</strong><small>이유 → 마지막 확인 → 담당 → 다음 행동</small></div>
        {inbox.visible.map(goal => { const facts = goal.kind === 'deliberation' ? null : blockFacts(goal, tasks, approvals, guidance, agents, events); const meeting = goal.kind === 'deliberation' ? meetingStatus(goal) : null; return <article key={goal.id} className="v3-inbox-item"><div className="v2-row"><strong title={goal.intent}>{goalTitle(goal, 90)}</strong><button className="primary" onClick={() => onOpen(goal.id)}>결정 열기 →</button></div>{facts ? <BlockFactsView facts={facts} /> : <small>{meeting?.label} · {meeting?.detail}</small>}</article>; })}
        <ListDisclosure label="결정할 일" total={inbox.total} hidden={inbox.hidden} expanded={inbox.expanded} onToggle={inbox.toggle}/>
      </section>}

      {!hideGoalList && <><div className="section-head"><h2>목표</h2><span className="count">{goals.length} GOALS</span></div>
      <div className="v2-stack">{window.visible.map(goal => {
        const isMeeting = goal.kind === 'deliberation';
        const rows = deliverableRows(goal, tasks, reviews, agents);
        const passed = rows.filter(row => row.review?.verdict === 'pass').length;
        const failed = rows.filter(row => row.review?.verdict === 'fail').length;
        const owners = [...new Set(tasks.filter(task => task.goalId === goal.id).map(task => agents.find(agent => agent.id === task.assigneeId)?.name ?? task.assigneeId))];
        const block = runBlock(goal);
        return <article className="card v2-goal" key={goal.id}>
          <div className="v2-row"><span className={`status ${goal.status}`}>{isMeeting ? `다관점 회의 · ${meetingStatus(goal).label}` : stageLabel(goal)}</span><small>{new Date(goal.createdAt).toLocaleDateString()}</small></div>
          <h3>{goalTitle(goal, 130)}</h3>
          <p>{isMeeting ? `${meetingStatus(goal).detail} · 다음: ${meetingStatus(goal).next}` : rows.length ? `산출물 ${rows.length}개 · QA 통과 ${passed}${failed ? ` · 실패 ${failed}` : ''} · 담당 ${owners.join(', ') || '배정 전'}` : `담당 ${owners.length ? owners.join(', ') : '배정 전'} · QA 재작업 ${goal.reviewLoopsUsed}/${goal.reviewLoopLimit ?? 2}`}</p>
          {rows.length > 0 && <ul className="v3-deliverables">{rows.map(row => <li key={row.task.id} className={row.review ? row.review.verdict : row.task.status}>{row.title}<small> · {row.owner}{row.review ? ` · QA ${row.review.verdict === 'pass' ? '통과' : '실패'}` : ''}</small></li>)}</ul>}
          <div className="button-row">
            <button className={goal.status === 'completed' && rows.length ? 'primary' : 'secondary'} onClick={() => onOpen(goal.id)}>{isMeeting ? (meetingStatus(goal).needsDecision ? '결정 열기' : '회의 열기') : decisions.includes(goal) ? '결정 열기' : goal.status === 'completed' && rows.length ? '산출물 인수 확인' : '목표 열기'}</button>
            {!isMeeting && goal.status === 'new' && <button className={block ? 'primary is-blocked' : 'primary'} onClick={() => block ? onExplain(block.title, block.reason, block.next) : onRun(goal.id)}>팀 실행 시작</button>}
            {!isMeeting && <button className="text-button" aria-expanded={showGraph === goal.id} onClick={() => setShowGraph(showGraph === goal.id ? null : goal.id)}>진행 보기</button>}
          </div>
          {!isMeeting && showGraph === goal.id && <GoalGraph goal={goal} tasks={tasks}/>}
        </article>;
      })}{!goals.length && <div className="empty-state compact">아직 목표가 없습니다. 위에서 첫 목표를 전달해 주세요. 출시 준비 예시로 시작할 수 있습니다.</div>}<ListDisclosure label="목표" total={window.total} hidden={window.hidden} expanded={window.expanded} onToggle={window.toggle}/></div></>}
    </div></div>;
}
