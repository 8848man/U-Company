import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { withSummaries } from './plainText';
import { listen } from '@tauri-apps/api/event';
import OfficeCanvas from './office/OfficeCanvas';
import ProjectPicker from './ProjectPicker';
import SpecImportPanel from './SpecImportPanel';
import ActionToast from './ActionToast';
import ConnectionDialog from './ConnectionDialog';
import OfficeTeamSettings from './OfficeTeamSettings';
import OfficeMemberPanel from './OfficeMemberPanel';
import { BoardProgress } from './OfficeActivity';
import { GoalConversation } from './GoalConversationChat';
import type { GitBootstrapPreview, GitBootstrapResult } from './GitBootstrapPanel';
import { OfficeGoalRail } from './OfficeGoalRail';
import { memberActivity, goalTitle } from './progress';
import { displayPath } from './paths';
import { canOpenProviderLogin, connectionRecovery } from './providerRecovery';
import { goalRecovery } from './goalRecovery';
import { openWork } from './openWork';
import { OpenWorkLauncher } from './OpenWorkLauncher';
import TokenUsageView from './TokenUsageView';
import { ContentPreview, ListDisclosure, useLimitedItems } from './ContentDensity';
import './ux.css';
import { PlanningOriginPanel } from './PlanningOriginPanel';
import type { Agent, AiConnection, DiscoveredAi, Goal, GoalTurn, Project, RolePreset, Snapshot } from './types';

type Tab = 'dashboard' | 'projects' | 'board' | 'usage' | 'connections' | 'office';
type Explanation = { title: string; reason: string; next: string };
const empty: Snapshot = { project: null, projects: [], agents: [], connections: [], goals: [], goalTurns: [], goalGuidance: [], tasks: [], events: [], approvals: [], usage: [], specProfiles: [], specDrafts: [] };
const labels: Record<Tab, string> = { dashboard: 'CEO 데스크', projects: '프로젝트', board: '업무 현황', usage: '토큰 사용량', connections: 'AI 연결', office: 'Office' };
const navigation: Tab[] = ['dashboard', 'projects', 'board', 'usage', 'connections', 'office'];
const icons: Record<Tab, string> = { dashboard: '◫', projects: '▣', board: '▤', usage: '◉', connections: '⌘', office: '▦' };

function statusLabel(status: string) {
  const map: Record<string, string> = {
    new: '시작 전', analyzing: '목표 분석 중', planning: '업무 나누는 중', awaiting_approval: '승인 대기',
    executing: '팀 작업 중', integrating: '결과 합치는 중', validating: '품질 확인 중', completed: '완료',
    blocked: '확인 필요', failed: '실패', pending: '시작 전', running: '작업 중', awaiting_integration: '결과 합치기 전',
    ready: '사용 가능', needs_auth_check: '로그인 확인 필요', needs_setup: '로그인 필요', missing: '미설치',
  };
  return map[status] ?? status;
}

function reason(title: string, why: string, next: string): Explanation { return { title, reason: why, next }; }

function teamRunBlock(snapshot: Snapshot): Explanation | null {
  const roles = [
    { label: 'CTO', find: (agent: Agent) => agent.id === 'cto' || agent.role.toLowerCase().includes('architect') },
    { label: 'PM', find: (agent: Agent) => agent.id === 'pm' || agent.role.toLowerCase().includes('plan') },
    { label: 'QA', find: (agent: Agent) => agent.id === 'qa' || agent.role.toLowerCase().includes('review') },
  ];
  for (const role of roles) {
    const agent = snapshot.agents.find(item => item.enabled && role.find(item));
    if (!agent) return reason(`${role.label} 구성원이 필요합니다`, '목표를 분석하고 검증할 필수 역할이 활성화되어 있지 않습니다.', 'Office에서 구성원을 활성화하고 역할을 확인하세요.');
    const connection = snapshot.connections.find(item => item.id === agent.connectionId);
    if (!connection) return reason(`${agent.name}에게 AI가 필요합니다`, '담당 구성원에게 등록된 AI가 배정되지 않았습니다.', 'AI 연결에서 도구를 등록한 뒤 Office에서 배정하세요.');
    if (['missing', 'failed', 'needs_setup'].includes(connection.status)) return reason(`${agent.name}의 AI를 사용할 수 없습니다`, `${connection.name} 상태: ${statusLabel(connection.status)}`, connectionRecovery(connection));
  }
  const developers = snapshot.agents.filter(agent => agent.enabled && agent.role.toLowerCase().includes('implement'));
  if (developers.length < 1) return reason('개발 담당이 필요합니다', '파일 작업을 맡길 활성 구성원이 없습니다.', 'Office에서 구현 역할의 구성원을 준비하세요.');
  for (const agent of developers) {
    const connection = snapshot.connections.find(item => item.id === agent.connectionId);
    if (!connection) return reason(`${agent.name}에게 AI가 필요합니다`, '개발 담당에게 등록된 AI가 배정되지 않았습니다.', 'AI 연결에서 CLI를 등록한 뒤 Office에서 배정하세요.');
    if (connection.kind === 'http') return reason(`${agent.name}에게 파일 작업 AI가 필요합니다`, '일반 HTTP 모델 연결은 프로젝트 파일을 편집할 수 없습니다.', 'Office에서 Agent CLI 또는 파일 작업 어댑터를 배정하세요.');
    if (['missing', 'failed', 'needs_setup'].includes(connection.status)) return reason(`${agent.name}의 AI를 사용할 수 없습니다`, `${connection.name} 상태: ${statusLabel(connection.status)}`, connectionRecovery(connection));
  }
  return null;
}

function GuardedButton({ children, className, blocked, onBlocked, onClick }: {
  children: React.ReactNode; className: string; blocked?: Explanation | null;
  onBlocked: (explanation: Explanation) => void; onClick: () => void;
}) {
  return <button type="button" className={`${className}${blocked ? ' is-blocked' : ''}`} title={blocked ? '눌러서 이유 보기' : undefined}
    onClick={() => blocked ? onBlocked(blocked) : onClick()}>{children}</button>;
}

export default function App() {
  const [tab, setTab] = useState<Tab>('dashboard');
  const [snapshot, setSnapshot] = useState<Snapshot>(empty);
  const [discovered, setDiscovered] = useState<DiscoveredAi[]>([]);
  const [presets, setPresets] = useState<RolePreset[]>([]);
  const [busy, setBusy] = useState(false);
  const [backendOutdated, setBackendOutdated] = useState(false);
  const [dialog, setDialog] = useState<Explanation | null>(null);
  const [reviewLimitChange, setReviewLimitChange] = useState<{ kind: 'project' | 'goal'; id: string; name: string; before: number; after: number } | null>(null);
  const [approvalDialogId, setApprovalDialogId] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  const [noticeGoalId, setNoticeGoalId] = useState<string | null>(null);
  const [connectionDialogOpen, setConnectionDialogOpen] = useState(false);
  const [connectionPathChange, setConnectionPathChange] = useState<DiscoveredAi | null>(null);
  const [intent, setIntent] = useState('');
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const [detailMode, setDetailMode] = useState<'activity' | 'settings'>('activity');
  const [officeMode, setOfficeMode] = useState<'floor' | 'team'>('floor');
  const [agentDraft, setAgentDraft] = useState<Agent | null>(null);
  const [presetId, setPresetId] = useState('');
  const [expandedGoalIds, setExpandedGoalIds] = useState<Set<string>>(() => new Set());
  const [conversationGoalId, setConversationGoalId] = useState<string | null>(null);
  const [conversationTaskId, setConversationTaskId] = useState<string | null>(null);

  const refreshSequence = useRef(0);

  const refresh = useCallback(async () => {
    const sequence = ++refreshSequence.current;
    const next = await invoke<Snapshot>('get_snapshot');
    if (sequence === refreshSequence.current) setSnapshot(withSummaries(next));
  }, []);
  const refreshDiscovery = useCallback(async () => { setDiscovered(await invoke<DiscoveredAi[]>('discover_ai')); }, []);
  useEffect(() => {
    void invoke<{ protocolVersion: number }>('get_backend_info').then(info => setBackendOutdated(info.protocolVersion < 19)).catch(() => setBackendOutdated(true));
    void refresh().catch(error => setDialog(reason('앱 상태를 읽지 못했습니다', String(error), '앱을 다시 시작하고 데이터 경로를 확인하세요.')));
    void refreshDiscovery().catch(error => setDialog(reason('AI 검색에 실패했습니다', String(error), '연결을 다시 검사하세요.')));
    void invoke<RolePreset[]>('get_role_presets').then(setPresets).catch(() => {});
    let pendingRefresh: number | undefined;
    const listening = listen('state-changed', () => {
      if (pendingRefresh !== undefined) return;
      pendingRefresh = window.setTimeout(() => { pendingRefresh = undefined; void refresh(); }, 100);
    });
    return () => { if (pendingRefresh !== undefined) window.clearTimeout(pendingRefresh); void listening.then(unlisten => unlisten()); };
  }, [refresh, refreshDiscovery]);

  const visibleGoals = useMemo(() => snapshot.goals.filter(goal => goal.kind !== 'deliberation' && (goal.projectId === snapshot.project?.id || goal.projectId === null)), [snapshot.goals, snapshot.project]);
  const goalIds = useMemo(() => new Set(visibleGoals.map(goal => goal.id)), [visibleGoals]);
  const visibleTasks = snapshot.tasks.filter(task => goalIds.has(task.goalId));
  const visibleApprovals = snapshot.approvals.filter(approval => approval.goalId && goalIds.has(approval.goalId));
  const visibleEvents = snapshot.events.filter(event => event.goalId === null || goalIds.has(event.goalId));
  const projectUsage = (snapshot.usage ?? []).filter(item => item.projectId === snapshot.project?.id);
  const projectTokenTotal = projectUsage.reduce((total, item) => total + item.inputTokens + item.outputTokens, 0);
  const openWorkItems = useMemo(() => openWork(snapshot).filter(item => item.goal.kind !== 'deliberation'), [snapshot]);
  const urgentGoalIds = new Set(visibleApprovals.filter(approval => approval.status === 'pending').map(approval => approval.goalId));
  const dashboardGoals = useLimitedItems([...visibleGoals].sort((a, b) =>
    Number(b.id === noticeGoalId && Boolean(notice)) - Number(a.id === noticeGoalId && Boolean(notice)) ||
    Number(urgentGoalIds.has(b.id)) - Number(urgentGoalIds.has(a.id)) ||
    b.createdAt.localeCompare(a.createdAt)
  ), Math.max(5, urgentGoalIds.size + Number(Boolean(notice && noticeGoalId))));
  const boardTasks = useLimitedItems([...visibleTasks].sort((a, b) => {
    const priority = (status: string) => ({ running: 0, blocked: 1, failed: 1, pending: 2, awaiting_integration: 3, completed: 4 }[status] ?? 5);
    return priority(a.status) - priority(b.status) || (snapshot.goals.find(goal => goal.id === b.goalId)?.createdAt ?? '').localeCompare(snapshot.goals.find(goal => goal.id === a.goalId)?.createdAt ?? '');
  }));
  const boardEvents = useLimitedItems(visibleEvents);
  const registeredProjects = useLimitedItems([...snapshot.projects].sort((a, b) => Number(b.id === snapshot.project?.id) - Number(a.id === snapshot.project?.id) || a.name.localeCompare(b.name, 'ko')));
  const localConnections = useLimitedItems(discovered.filter(item => item.source === 'local'));
  const presetConnections = useLimitedItems(discovered.filter(item => item.source === 'preset'));
  const savedConnections = useLimitedItems(snapshot.connections);
  const activities = useMemo(() => Object.fromEntries(snapshot.agents.map(agent => [agent.id, memberActivity(agent, snapshot.agents, visibleGoals, visibleTasks, visibleEvents)])), [snapshot.agents, visibleGoals, visibleTasks, visibleEvents]);

  function selectAgent(id: string) {
    const agent = snapshot.agents.find(item => item.id === id);
    setOfficeMode('floor');
    setSelectedAgentId(id);
    setDetailMode('activity');
    setAgentDraft(agent ? { ...agent } : null);
    setPresetId(agent?.id.startsWith('developer') ? 'developer' : agent?.id ?? 'specialist');
  }

  async function hideAgent(agent: Agent) {
    const saved = await action<Agent>('save_agent', { agent: { ...agent, enabled: false } }, `${agent.name}을(를) 팀에서 숨겼습니다. 숨긴 팀원 목록에서 다시 배치할 수 있습니다.`);
    if (saved) { setSelectedAgentId(null); setAgentDraft(null); }
  }

  async function addAgent() {
    const preset = presets.find(item => item.id === 'specialist');
    const agent: Agent = { id: crypto.randomUUID(), name: '새 구성원', role: preset?.role ?? 'specialist', instructions: preset?.instructions ?? '', reportsTo: null, connectionId: '', modelId: '', x: 50, y: 50, enabled: true };
    const saved = await action<Agent>('save_agent', { agent }, '구성원을 추가했습니다.');
    if (saved) { setSelectedAgentId(saved.id); setAgentDraft(saved); setPresetId('specialist'); setDetailMode('settings'); setOfficeMode('floor'); }
  }

  async function checkSavedConnection(id: string) {
    const checked = await action<DiscoveredAi>('check_connection', { id });
    if (checked) setDialog(reason(`${checked.connection.name} 상태: ${statusLabel(checked.connection.status)}`, checked.detail, connectionRecovery(checked.connection)));
  }

  async function openConnectionLogin(id: string) {
    const message = await action<string>('start_connection_login', { id });
    if (message) setDialog(reason('로그인 창을 열었습니다', message, '로그인을 마친 뒤 AI 연결에서 “이 경로로 다시 검사”를 눌러 상태를 확인하세요.'));
  }

  async function action<T>(command: string, args?: Record<string, unknown>, success?: string): Promise<T | undefined> {
    if (busy) { setDialog(reason('잠시 기다려 주세요', '다른 작업을 처리하고 있습니다.', '현재 작업이 끝나면 다시 누르세요.')); return undefined; }
    setBusy(true); setNotice(''); setNoticeGoalId(null);
    try {
      const result = await invoke<T>(command, args);
      await refresh();
      if (success) setNotice(success);
      return result;
    } catch (error) {
      setDialog(reason('작업을 완료하지 못했습니다', String(error), '표시된 조건을 확인한 뒤 다시 시도하세요.'));
      await refresh().catch(() => {});
      return undefined;
    } finally { setBusy(false); }
  }

  async function moveAgent(id: string, x: number, y: number) {
    const agent = snapshot.agents.find(item => item.id === id);
    if (!agent) return;
    if (busy) { setDialog(reason('위치를 옮길 수 없습니다', '다른 저장 작업이 진행 중입니다.', '잠시 후 다시 드래그하세요.')); return; }
    setSnapshot(current => ({ ...current, agents: current.agents.map(item => item.id === id ? { ...item, x, y } : item) }));
    await action<void>('save_agent', { agent: { ...agent, x, y } }, `${agent.name}의 자리를 저장했습니다.`);
    if (selectedAgentId === id) setAgentDraft(current => current ? { ...current, x, y } : current);
  }

  const busyReason = busy ? reason('잠시 기다려 주세요', '다른 작업을 처리하고 있습니다.', '작업이 끝나면 다시 누르세요.') : null;
  function openGoalWorkspace(goalId: string, taskId: string | null = null) {
    setConversationTaskId(taskId);
    setConversationGoalId(goalId);
  }
  function closeGoalWorkspace() {
    setConversationGoalId(null);
    setConversationTaskId(null);
  }
  async function openWorkItem(goalId: string, taskId?: string) {
    const goal = snapshot.goals.find(item => item.id === goalId);
    if (!goal) { setDialog(reason('목표를 찾을 수 없습니다', '목록을 여는 동안 목표 상태가 바뀌었습니다.', '열린 작업 목록을 다시 확인해 주세요.')); return; }
    if (taskId && !snapshot.tasks.some(task => task.id === taskId && task.goalId === goalId)) { setDialog(reason('업무를 찾을 수 없습니다', '목록을 여는 동안 업무 상태가 바뀌었습니다.', '열린 작업 목록을 다시 확인해 주세요.')); return; }
    if (goal.projectId && snapshot.project?.id !== goal.projectId) {
      if (!snapshot.projects.some(project => project.id === goal.projectId)) { setDialog(reason('프로젝트를 찾을 수 없습니다', '이 목표의 등록된 프로젝트가 없습니다.', '프로젝트 목록에서 경로를 확인해 주세요.')); return; }
      const switched = await action<Project>('switch_project', { id: goal.projectId });
      if (!switched) return;
    }
    openGoalWorkspace(goalId, taskId ?? null);
  }
  function recoveryRunBlock(goal: Goal): Explanation | null {
    return busyReason ?? (backendOutdated ? reason('앱 재시작이 필요합니다', '실행 중인 백엔드에는 최신 목표 실행 코드가 없습니다.', '앱을 닫고 시작 스크립트로 다시 실행하세요.') : !goal.projectId ? reason('프로젝트 연결이 없습니다', '이 목표의 작업 폴더를 확인할 수 없습니다.', '프로젝트를 다시 선택해 주세요.') : teamRunBlock(snapshot));
  }

  function reviewLimitDropdown(project: Project) {
    return <label className="review-limit">새 목표의 QA 기본값<select aria-label={`${project.name} 새 목표의 QA 재작업 기본값`} value={project.reviewLoopLimit ?? 2} onChange={event => {
      if (backendOutdated) { setDialog(reason('앱 재시작이 필요합니다', '실행 중인 백엔드가 이 설정을 지원하지 않습니다.', '앱을 다시 시작한 뒤 저장하세요.')); return; }
      if (busy) { setDialog(busyReason!); return; }
      const after = Number(event.target.value);
      if (after !== (project.reviewLoopLimit ?? 2)) setReviewLimitChange({ kind: 'project', id: project.id, name: project.name, before: project.reviewLoopLimit ?? 2, after });
    }}>{[0, 1, 2, 3, 4, 5].map(count => <option key={count} value={count}>{count}회</option>)}</select></label>;
  }

  function goalReviewLimitDropdown(goal: Goal) {
    return <label className="review-limit">이 목표의 QA 재작업 최대 횟수<select aria-label={`${goal.intent} QA 재작업 최대 횟수`} value={goal.reviewLoopLimit ?? 2} onChange={event => {
      if (backendOutdated) { setDialog(reason('앱 재시작이 필요합니다', '실행 중인 백엔드가 목표별 QA 설정을 지원하지 않습니다.', '앱을 다시 시작한 뒤 저장하세요.')); return; }
      if (busy) { setDialog(busyReason!); return; }
      const after = Number(event.target.value);
      if (after !== (goal.reviewLoopLimit ?? 2)) setReviewLimitChange({ kind: 'goal', id: goal.id, name: goal.intent, before: goal.reviewLoopLimit ?? 2, after });
    }}>{[0, 1, 2, 3, 4, 5].map(count => <option key={count} value={count}>{count}회</option>)}</select></label>;
  }

  function openGoalReview(goal: Goal) {
    openGoalWorkspace(goal.id);
  }
  const approvalToReview = snapshot.approvals.find(approval => approval.id === approvalDialogId && approval.status === 'pending');

  return <div className="shell">
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark">✦</span><div><strong>UTOPIA</strong><small>AI ORGANIZATION OS</small></div></div>
      <div className="sidebar-label">WORKSPACE</div>
      <button className="workspace-switch" onClick={() => setTab('projects')}><small>현재 프로젝트</small><strong>{snapshot.project?.name ?? '프로젝트 선택'}</strong><span>변경 →</span></button>
      <nav>{navigation.map(key => <button key={key} className={tab === key ? 'nav active' : 'nav'} onClick={() => setTab(key)}><span className="nav-icon">{icons[key]}</span>{labels[key]}</button>)}</nav>
      <div className="sidebar-spacer" />
      <div className="sidebar-foot"><span className="online-dot" /> LOCAL FIRST <span className="version">v0.1</span></div>
    </aside>

    <main className="main">
      <header className="topbar"><div className="breadcrumb">WORKSPACE <span>/</span> {labels[tab]}</div><div className="topbar-right"><span className="platform">{snapshot.project?.name ?? '설정 시작'}</span><span className="avatar">CEO</span></div></header>
      <div className="main-scroll">
      {backendOutdated && <div className="restart-banner" role="alert"><strong>앱을 다시 시작해야 최신 업무 진행 표시가 적용됩니다.</strong><span>현재 화면은 새 코드지만 실행 중인 백엔드는 이전 버전입니다.</span><button type="button" className="secondary" onClick={() => setDialog(reason('앱 재시작 방법', '실행 중인 창은 이전 Rust 백엔드를 사용하고 있습니다.', '앱 창을 닫고 PowerShell에서 .\\scripts\\start-windows.ps1을 다시 실행하세요.'))}>재시작 방법</button></div>}
      {notice && <ActionToast message={notice} actionLabel={noticeGoalId ? '목표 보기' : undefined} onAction={noticeGoalId ? () => document.getElementById(`goal-${noticeGoalId}`)?.scrollIntoView({ behavior: 'smooth', block: 'center' }) : undefined} onDismiss={() => setNotice('')} />}

      {tab === 'dashboard' && <div className="page dashboard">
        <section className="hero"><div><div className="eyebrow">YOUR ORGANIZATION, AT WORK</div><h1>목표를 말하면,<br /><em>팀이 움직입니다.</em></h1><p>프로젝트를 고르고, AI 팀을 준비한 뒤 원하는 결과를 말해 주세요.</p></div><div className="hero-symbol">✳</div></section>
        <div className="journey"><span className={snapshot.project ? 'done' : ''}>1. 프로젝트 선택</span><span className={snapshot.agents.some(agent => agent.connectionId) ? 'done' : ''}>2. AI 팀 연결</span><span>3. 목표 전달</span></div>
        <div className="dashboard-grid">
          <section className="card command-card"><div className="card-kicker">CEO GOAL</div><h2>팀에 무엇을 맡길까요?</h2><p>만들 결과와 꼭 지켜야 할 조건을 편하게 적어주세요.</p><textarea value={intent} onChange={event => setIntent(event.target.value)} placeholder="예: 로그인 화면을 개선하고 테스트까지 마친 뒤 결과를 알려줘." /><div className="card-actions"><span>{snapshot.project ? `작업할 곳: ${snapshot.project.name}` : '먼저 프로젝트를 선택하세요'}</span><GuardedButton className="primary" onBlocked={setDialog} blocked={busyReason ?? (!snapshot.project ? reason('프로젝트가 필요합니다', 'AI 팀이 작업할 폴더가 정해지지 않았습니다.', '프로젝트 탭에서 작업할 폴더를 선택해 주세요.') : intent.trim().length < 10 ? reason('목표를 조금 더 적어 주세요', '무엇을 만들지 판단할 정보가 부족합니다.', '원하는 결과와 조건을 한 문장 이상 적어 주세요.') : null)} onClick={() => void action<Goal>('submit_goal', { intent }, '목표를 팀에 전달했습니다. 아래에서 팀 실행을 시작하세요.').then(created => { if (created) { setIntent(''); setNoticeGoalId(created.id); } })}>목표 전달 ↗</GuardedButton></div></section>
          <section className="card project-card"><div className="card-kicker">CURRENT PROJECT</div><h2>{snapshot.project?.name ?? '프로젝트를 골라 주세요'}</h2><p>{snapshot.project ? '이 프로젝트에 목표가 저장되고 AI 팀이 작업합니다.' : '작업할 폴더 하나를 선택하면 시작할 수 있습니다.'}</p>{snapshot.project && <code className="path-line">{displayPath(snapshot.project.path)}</code>}<div className="token-summary" aria-live="polite">이 프로젝트의 누적 토큰 추정치 <strong>{projectTokenTotal.toLocaleString()}</strong><small>{projectUsage.some(item => item.source === 'estimated') ? '일부 추정 포함' : '제공자 보고 기반'}</small></div>{snapshot.project && reviewLimitDropdown(snapshot.project)}<button className="secondary" onClick={() => setTab('projects')}>{snapshot.project ? '다른 프로젝트 보기 →' : '프로젝트 선택하기 →'}</button></section>
        </div>
        <div className="section-head"><div><div className="eyebrow">TEAM WORK</div><h2>현재 프로젝트의 목표</h2></div><span className="count">{visibleGoals.length} GOALS</span></div>
        <div className="goal-list">{visibleGoals.length ? dashboardGoals.visible.map(goal => {
          const goalProject = snapshot.projects.find(project => project.id === goal.projectId);
          const retryable = goal.status === 'blocked' && ['analyzing', 'planning'].includes(goal.resumedFrom ?? '')
            && !visibleTasks.some(task => task.goalId === goal.id) && !visibleApprovals.some(approval => approval.goalId === goal.id);
          const analysisAgent = snapshot.agents.find(agent => agent.id === 'cto' && agent.enabled)
            ?? snapshot.agents.find(agent => agent.enabled && agent.role.toLowerCase().includes('architect'));
          const analysisConnection = snapshot.connections.find(connection => connection.id === analysisAgent?.connectionId);
          const startBlock = busyReason ?? (backendOutdated ? reason('앱 재시작이 필요합니다', '실행 중인 백엔드에는 최신 AI 실행 수정이 없습니다.', '앱 창을 닫고 시작 스크립트로 다시 실행하세요.') : !goal.projectId ? reason('프로젝트 연결이 없습니다', '이 목표의 작업 폴더를 확인할 수 없습니다.', '현재 프로젝트에서 목표를 다시 전달해 주세요.') : teamRunBlock(snapshot));
          return <article className="goal-card" id={`goal-${goal.id}`} key={goal.id}>
            <div className="goal-top"><span className={`status ${goal.status}`}>{statusLabel(goal.status)}</span><time>{new Date(goal.createdAt).toLocaleString()}</time></div>
            <h3>{expandedGoalIds.has(goal.id) ? goal.intent : goalTitle(goal)}</h3>
            {(goalTitle(goal) !== goal.intent || goal.report) && <button type="button" className="text-button goal-expand" aria-expanded={expandedGoalIds.has(goal.id)} onClick={() => setExpandedGoalIds(current => { const next = new Set(current); if (next.has(goal.id)) next.delete(goal.id); else next.add(goal.id); return next; })}>{expandedGoalIds.has(goal.id) ? '간단히 보기 ↑' : '전체 목표와 보고 보기 ↓'}</button>}
            <p className="muted">{visibleTasks.filter(task => task.goalId === goal.id).length}개 업무 · QA 재작업 {goal.reviewLoopsUsed ?? 0}/{goal.reviewLoopLimit ?? 2}회 · {goal.projectId ? goalProject?.name ?? '프로젝트 없음' : '이전 버전 목표'}</p>
            {(goal.status === 'new' || retryable) && <p className="muted">다음 분석 담당: {analysisAgent?.name ?? '미배정'} · {analysisConnection?.name ?? 'AI 미연결'} <button type="button" className="text-button" onClick={() => { setTab('office'); if (analysisAgent) selectAgent(analysisAgent.id); }}>담당 AI 확인 →</button></p>}
            {(snapshot.goalGuidance ?? []).some(item => item.goalId === goal.id && item.status !== 'revised') && <p className="goal-guidance-badge">CEO 방향 변경 {snapshot.goalGuidance.some(item => item.goalId === goal.id && item.status === 'review_required') ? '검토 필요' : '다음 안전 지점 대기 중'}</p>}
            <button type="button" className="secondary goal-conversation-open" onClick={() => openGoalWorkspace(goal.id)}>팀과 대화 · 이해 확인 ({(snapshot.goalTurns ?? []).filter(turn => turn.goalId === goal.id).length})</button>
            {goalProject && <div className="goal-review-limit">{goalReviewLimitDropdown(goal)}<small>이 목표에만 적용됩니다.</small></div>}
            {goal.status === 'new' && <GuardedButton className="primary action" onBlocked={setDialog} blocked={startBlock} onClick={() => void action('run_goal', { goalId: goal.id }, 'AI 팀이 분석을 시작했습니다.')}>팀 실행 시작</GuardedButton>}
            {['blocked', 'awaiting_approval'].includes(goal.status) && <button type="button" className="secondary action" onClick={() => openGoalReview(goal)}>{goalRecovery(goal, visibleTasks, visibleApprovals, snapshot.goalGuidance ?? [])?.label ?? '다음 단계 확인'} →</button>}
            {goal.report && expandedGoalIds.has(goal.id) && <ContentPreview><pre className="report">{goal.report}</pre></ContentPreview>}
          </article>;
        }) : <div className="empty-state">아직 이 프로젝트의 목표가 없습니다. 위에서 첫 목표를 전달해 주세요.</div>}<ListDisclosure label="목표" total={dashboardGoals.total} hidden={dashboardGoals.hidden} expanded={dashboardGoals.expanded} onToggle={dashboardGoals.toggle} /></div>
      </div>}

      {tab === 'projects' && <div className="page">
        <div className="page-title"><div className="eyebrow">WORKSPACE PROJECTS</div><h1>프로젝트</h1><p>작업할 폴더를 선택하세요. Git 없이도 빈 폴더나 기존 파일이 있는 폴더를 그대로 등록할 수 있습니다.</p></div>
        <div className="section-head"><h2>등록된 프로젝트</h2><span className="count">{snapshot.projects.length} PROJECTS</span></div>
        <div className="project-grid">
          {registeredProjects.visible.map(project => <article className={`card project-tile ${snapshot.project?.id === project.id ? 'current' : ''}`} key={project.id}>
            <span className="project-icon">▣</span><div><ContentPreview><h2>{project.name}</h2></ContentPreview><code className="path-line" title={displayPath(project.path)}>{displayPath(project.path)}</code><small>{project.kind === 'folder' ? '일반 폴더' : 'Git 프로젝트'} · {snapshot.goals.filter(goal => goal.projectId === project.id).length}개 목표</small>{reviewLimitDropdown(project)}{snapshot.goals.some(goal => goal.projectId === project.id && ['blocked', 'awaiting_approval'].includes(goal.status)) && <button type="button" className="text-button" onClick={() => { const goal = snapshot.goals.filter(item => item.projectId === project.id && ['blocked', 'awaiting_approval'].includes(item.status)).sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0]; if (goal) { if (snapshot.project?.id !== project.id) void action<Project>('switch_project', { id: project.id }).then(saved => { if (saved) { setTab('dashboard'); openGoalReview(goal); } }); else { setTab('dashboard'); openGoalReview(goal); } } }}>확인 필요한 목표 보기 →</button>}</div>
            <GuardedButton className="secondary" onBlocked={setDialog} blocked={busyReason ?? (snapshot.project?.id === project.id ? reason('이미 선택된 프로젝트입니다', '현재 이 프로젝트에서 작업하고 있습니다.', '다른 프로젝트를 고르거나 CEO 데스크로 돌아가세요.') : null)} onClick={() => void action<Project>('switch_project', { id: project.id }, `${project.name} 프로젝트로 전환했습니다.`)}>선택</GuardedButton>
          </article>)}
          {snapshot.projects.length === 0 && <div className="empty-state">등록된 프로젝트가 없습니다. 아래에서 작업할 폴더를 선택해 주세요.</div>}
          <ListDisclosure label="프로젝트" total={registeredProjects.total} hidden={registeredProjects.hidden} expanded={registeredProjects.expanded} onToggle={registeredProjects.toggle} />
        </div>
        <div className="project-picker-row">
          <ProjectPicker busy={busy} onError={message => setDialog(reason('폴더를 선택할 수 없습니다', message, '다른 폴더를 고르거나 직접 경로를 입력해 주세요.'))} onRegister={path => void action<Project>('set_project', { path }, '프로젝트를 준비하고 선택했습니다.')} />
        </div>
        <SpecImportPanel key={snapshot.project?.id ?? 'no-project'} project={snapshot.project} agents={snapshot.agents} profile={snapshot.specProfiles?.find(item => item.projectId === snapshot.project?.id)} savedDraft={[...(snapshot.specDrafts ?? [])].filter(item => item.projectId === snapshot.project?.id).sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0]} busy={busy} backendOutdated={backendOutdated} onApplied={refresh} onError={message => setDialog(reason('명세를 가져오지 못했습니다', message, '파일 형식과 대상 프로젝트를 확인한 뒤 다시 미리보기를 만드세요.'))} />
      </div>}
      {tab === 'board' && <div className="page"><div className="page-title"><div className="eyebrow">TEAM PROGRESS</div><h1>업무 현황</h1><p>누가 무엇을 하고 있는지, 승인이 필요한지 확인하세요.</p></div><BoardProgress goals={visibleGoals} tasks={visibleTasks} agents={snapshot.agents} approvals={visibleApprovals} onOpenGoal={openGoalWorkspace} onSaveGroup={(id, group) => void action<Goal>('set_goal_work_group', { id, group }, '업무 묶음을 저장했습니다.')} /><div className="section-head"><h2>결정이 필요한 일</h2></div>{visibleApprovals.filter(approval => approval.status === 'pending').map(approval => <article className="card board-card" key={approval.id}><strong>프로젝트 변경 승인</strong><p>{approval.reason}</p><div className="button-row"><button type="button" className="primary" onClick={() => setApprovalDialogId(approval.id)}>확인하기 ↗</button></div></article>)}{!visibleApprovals.some(approval => approval.status === 'pending') && <div className="empty-state compact">지금 결정할 일은 없습니다.</div>}<div className="section-head"><h2>팀이 맡은 일</h2><span className="count">{visibleTasks.length} TASKS</span></div><div className="board-grid">{boardTasks.visible.map(task => <article className="card board-card" key={task.id}><span className={`status ${task.status}`}>{statusLabel(task.status)}</span><ContentPreview><h2>{task.title}</h2></ContentPreview><ContentPreview><p>{task.instructions}</p></ContentPreview><button type="button" className="assignee-link" onClick={() => { selectAgent(task.assigneeId); setTab('office'); }}>담당: {snapshot.agents.find(agent => agent.id === task.assigneeId)?.name ?? task.assigneeId} → Office에서 보기</button><small>대상 파일: {task.files.join(', ') || '미지정'}</small>{task.worktree && <small className="path-line">작업 폴더: {task.worktree}</small>}{task.result && <ContentPreview><pre className="report">{task.result}</pre></ContentPreview>}</article>)}</div><ListDisclosure label="업무" total={boardTasks.total} hidden={boardTasks.hidden} expanded={boardTasks.expanded} onToggle={boardTasks.toggle} />{visibleTasks.length === 0 && <div className="empty-state compact">목표를 실행하면 담당 업무가 여기에 나타납니다.</div>}<div className="section-head"><h2>활동 기록</h2></div><div className="event-list">{boardEvents.visible.map(event => <div key={event.id}><time>{new Date(event.at).toLocaleString()}</time><strong>{event.kind}</strong><ContentPreview><span>{event.kind === 'agent_handoff' ? `${snapshot.agents.find(agent => agent.id === event.agentId)?.name ?? '팀원'} → ${snapshot.agents.find(agent => agent.id === event.message)?.name ?? '팀원'}에게 인계` : event.message}</span></ContentPreview></div>)}</div><ListDisclosure label="활동 기록" total={boardEvents.total} hidden={boardEvents.hidden} expanded={boardEvents.expanded} onToggle={boardEvents.toggle} /></div>}

      {tab === 'usage' && <TokenUsageView snapshot={snapshot} />}

      {tab === 'connections' && <div className="page">
        <div className="page-title"><div className="eyebrow">AI CONNECTIONS</div><h1>AI 연결</h1><p>이 PC에서 실행 파일이나 모델 서버가 확인된 AI만 바로 등록할 수 있습니다.</p></div>
        <div className="section-head"><h2>이 PC에서 확인됨</h2><GuardedButton className="secondary" onBlocked={setDialog} blocked={backendOutdated ? reason('앱 재시작이 필요합니다', 'AI 검색을 수행하는 백엔드가 이전 버전입니다.', '앱 창을 닫고 시작 스크립트로 다시 실행하세요.') : null} onClick={() => void refreshDiscovery().then(() => setNotice('설치된 AI를 다시 검사했습니다.')).catch(error => setDialog(reason('AI 검색 실패', String(error), 'CLI가 실행되는지 확인하세요.')))}>다시 검사</GuardedButton></div>
        <div className="connection-list">
          {localConnections.visible.map(item => {
            const saved = snapshot.connections.find(connection => connection.id === item.connection.id);
            const registered = Boolean(saved && saved.executable === item.connection.executable);
            const differentPath = Boolean(saved && saved.executable !== item.connection.executable);
            const blocked = backendOutdated ? reason('앱 재시작이 필요합니다', '새 AI 연결 수정이 실행 중인 백엔드에는 아직 적용되지 않았습니다.', '앱 창을 닫고 시작 스크립트로 다시 실행하세요.') : registered ? reason('이미 등록되어 있습니다', '이 AI 연결은 이 워크스페이스에 저장되어 있습니다.', '아래 등록된 AI에서 저장된 경로를 다시 검사하세요.') : item.connection.status === 'failed' ? reason('이 AI를 실행하지 못했습니다', item.detail, item.nextAction) : null;
            return <article className="card connection-card" key={item.connection.id}>
              <div className="connection-row"><span className="connection-icon">◎</span><span><strong>{item.connection.name}</strong><ContentPreview><small>{statusLabel(item.connection.status)} · {item.detail}</small></ContentPreview><code title={displayPath(item.connection.executable || item.connection.endpoint)}>{displayPath(item.connection.executable || item.connection.endpoint)}</code></span>
                <GuardedButton className="secondary" onBlocked={setDialog} blocked={busyReason ?? blocked} onClick={() => differentPath ? setConnectionPathChange(item) : void action<AiConnection>('save_connection', { connection: item.connection }, `${item.connection.name} 연결을 저장하고 AI가 없는 팀원에게 배정했습니다.`)}>{registered ? '등록됨' : differentPath ? '발견 경로로 변경' : '등록하기'}</GuardedButton>
              </div><ContentPreview><p className="connection-next">다음 단계: {item.nextAction}</p></ContentPreview>
            </article>;
          })}
          {!discovered.some(item => item.source === 'local') && <div className="empty-state compact">실행 가능한 로컬 AI를 찾지 못했습니다. 아래에서 다른 AI를 연결할 수 있습니다.</div>}
          <ListDisclosure label="확인된 AI" total={localConnections.total} hidden={localConnections.hidden} expanded={localConnections.expanded} onToggle={localConnections.toggle} />
        </div>
        <div className="section-head"><h2>지원하지만 확인되지 않은 도구</h2></div>
        <div className="preset-tools">{presetConnections.visible.map(item => <div className="preset-tool" key={item.connection.id}><strong>{item.connection.name}</strong><ContentPreview><span>{item.detail}</span></ContentPreview><GuardedButton className="text-button" onBlocked={setDialog} blocked={reason(`${item.connection.name}을 추가할 수 없습니다`, item.detail, item.nextAction)} onClick={() => {}}>왜 안 되나요?</GuardedButton></div>)}<ListDisclosure label="지원 도구" total={presetConnections.total} hidden={presetConnections.hidden} expanded={presetConnections.expanded} onToggle={presetConnections.toggle} /></div>
        <div className="connection-bottom">
          <section className="card connection-add-card"><div className="card-kicker">CUSTOM AI</div><h2>다른 AI 연결</h2><p>CLI, 로컬 모델 서버, 외부 어댑터를 추가할 수 있습니다.</p><GuardedButton className="secondary" onBlocked={setDialog} blocked={backendOutdated ? reason('앱 재시작이 필요합니다', '연결을 저장하는 백엔드가 이전 버전입니다.', '앱 창을 닫고 시작 스크립트로 다시 실행하세요.') : null} onClick={() => setConnectionDialogOpen(true)}>연결 정보 입력 ↗</GuardedButton></section>
          <section className="card"><div className="card-kicker">YOUR CONNECTIONS</div><h2>등록된 AI</h2>
            {snapshot.connections.length ? savedConnections.visible.map(connection => <div className="registered-row" key={connection.id}>
              <strong>{connection.name}</strong><span>{statusLabel(connection.status)} · {snapshot.agents.filter(agent => agent.connectionId === connection.id).length}명 사용</span>
              <code>{displayPath(connection.executable || connection.endpoint)}</code>
              {connection.kind === 'cli' && <GuardedButton className="text-button" onBlocked={setDialog} blocked={busyReason ?? (backendOutdated ? reason('앱 재시작이 필요합니다', '실행 중인 백엔드는 저장된 AI 경로 재검사를 지원하지 않습니다.', '앱을 닫고 다시 실행하세요.') : null)} onClick={() => void checkSavedConnection(connection.id)}>이 경로로 다시 검사 · 결과 보기</GuardedButton>}
              {canOpenProviderLogin(connection) && <GuardedButton className="text-button" onBlocked={setDialog} blocked={busyReason ?? (backendOutdated ? reason('앱 재시작이 필요합니다', '로그인 창을 여는 새 백엔드를 실행해야 합니다.', '앱을 닫고 다시 실행하세요.') : null)} onClick={() => void openConnectionLogin(connection.id)}>{connection.status === 'ready' ? '다시 로그인 ↗' : '로그인 창 열기 ↗'}</GuardedButton>}
              {connection.status !== 'ready' && <small className="connection-next">{connectionRecovery(connection)}</small>}
            </div>) : <div className="empty-state compact">아직 등록된 AI가 없습니다. 위에서 이 PC의 AI를 등록하세요.</div>}
            <ListDisclosure label="등록된 AI" total={savedConnections.total} hidden={savedConnections.hidden} expanded={savedConnections.expanded} onToggle={savedConnections.toggle} />
          </section>
        </div>
      </div>}
      {tab === 'office' && <div className="page">
        <div className="page-title"><div className="eyebrow">LIVE OFFICE</div><h1>팀 Office</h1><p>팀원의 활동과 개인 설정을 확인하고, 팀 전체 변경은 팀 설정에서 관리하세요.</p></div>
        <div className="office-section-tabs" role="tablist" aria-label="Office 보기">
          <button type="button" role="tab" aria-selected={officeMode === 'floor'} className={officeMode === 'floor' ? 'selected' : ''} onClick={() => selectedAgentId ? selectAgent(selectedAgentId) : setOfficeMode('floor')}>Office 현황</button>
          <button type="button" role="tab" aria-selected={officeMode === 'team'} className={officeMode === 'team' ? 'selected' : ''} onClick={() => setOfficeMode('team')}>팀 설정</button>
        </div>
        {officeMode === 'floor' ? <>
          <OfficeGoalRail goals={visibleGoals} tasks={visibleTasks} agents={snapshot.agents} approvals={visibleApprovals} onOpen={openGoalWorkspace} onReview={openGoalReview} />
          <div className="office-layout">
            <section className="office"><div className="office-header"><strong>UTOPIA COMPANY</strong><span><span className="online-dot" /> TEAM STATE</span></div><div className="office-room"><OfficeCanvas agents={snapshot.agents} connections={snapshot.connections} activities={activities} events={visibleEvents} selectedId={selectedAgentId} onSelect={selectAgent} onMove={(id, x, y) => void moveAgent(id, x, y)} /></div><div className="office-hint">캐릭터 클릭: 활동·개인 설정 · 드래그: 자리 이동 · 방향키: 조금 이동</div></section>
            <OfficeMemberPanel agents={snapshot.agents} connections={snapshot.connections} presets={presets} selectedId={selectedAgentId} draft={agentDraft} activity={selectedAgentId ? activities[selectedAgentId] : undefined} tasks={visibleTasks} usage={projectUsage} mode={detailMode} presetId={presetId} busy={busy} onSelect={selectAgent} onMode={setDetailMode} onDraft={setAgentDraft} onPreset={setPresetId} onSave={() => { if (agentDraft) void action('save_agent', { agent: agentDraft }, `${agentDraft.name} 설정을 저장했습니다.`); }} onExplain={(title, detail, next) => setDialog(reason(title, detail, next))} onNotice={setNotice} onOpenGoal={openGoalWorkspace} />
          </div>
          <div className="office-note">Office는 저장된 팀과 업무 상태를 표시합니다. 배경 좌표 투영 코드는 Pixel Agents의 MIT 코드에서 가져왔고, 캐릭터는 자체 픽셀 도형입니다.</div>
        </> : <OfficeTeamSettings agents={snapshot.agents} connections={snapshot.connections} tasks={snapshot.tasks} importedSpec={snapshot.specProfiles?.find(item => item.projectId === snapshot.project?.id)} busy={busy} onAdd={() => void addAgent()} onAssign={async (connectionId, agentIds) => Boolean(await action<number>('assign_team_connection', { connectionId, agentIds }, `${agentIds.length}명의 AI 연결을 변경했습니다.`))} onHide={agent => void hideAgent(agent)} onRestore={agent => { void action<Agent>('save_agent', { agent: { ...agent, enabled: true } }, `${agent.name}을(를) 다시 배치했습니다.`); }} onExplain={(title, detail, next) => setDialog(reason(title, detail, next))} />}
      </div>}
      </div>
    </main>
    <OpenWorkLauncher items={openWorkItems} agents={snapshot.agents} onSelect={(goalId, taskId) => void openWorkItem(goalId, taskId)} />
    {conversationGoalId && snapshot.goals.find(goal => goal.id === conversationGoalId) && <GoalConversation key={`${conversationGoalId}:${conversationTaskId ?? ''}`} goal={snapshot.goals.find(goal => goal.id === conversationGoalId)!} projectKind={snapshot.projects.find(project => project.id === snapshot.goals.find(goal => goal.id === conversationGoalId)?.projectId)?.kind ?? 'folder'} focusTaskId={conversationTaskId} turns={(snapshot.goalTurns ?? []).filter(turn => turn.goalId === conversationGoalId)} guidance={(snapshot.goalGuidance ?? []).filter(item => item.goalId === conversationGoalId || item.replacementGoalId === conversationGoalId)} agents={snapshot.agents} connections={snapshot.connections} tasks={visibleTasks} approvals={visibleApprovals} events={visibleEvents} usage={projectUsage} reviews={snapshot.deliverableReviews ?? []} runBlock={recoveryRunBlock(snapshot.goals.find(goal => goal.id === conversationGoalId)!)} onClose={closeGoalWorkspace} onSend={async (message, agentId) => { if (backendOutdated) { setDialog(reason('앱 재시작이 필요합니다', '실행 중인 백엔드는 자연어 메시지 라우팅을 지원하지 않습니다.', '앱 창을 닫고 시작 스크립트로 다시 실행하세요.')); return false; } return Boolean(await action<GoalTurn>('send_goal_message', { goalId: conversationGoalId, agentId, message }, '메시지를 전달했습니다. 팀원이 의도를 확인해 이 대화에서 답하거나 작업을 이어갑니다.')); }} onOpenGoal={openGoalWorkspace} onOpenApproval={setApprovalDialogId} onRetry={async () => Boolean(await action('run_goal', { goalId: conversationGoalId }, '목표 분석을 다시 시작했습니다.'))} onResume={async () => Boolean(await action('resume_goal', { goalId: conversationGoalId }, '승인된 계획을 다시 시작했습니다.'))} onOpenConnections={() => { closeGoalWorkspace(); setTab('connections'); }} onOpenOffice={() => { closeGoalWorkspace(); setTab('office'); setOfficeMode('team'); }} onOpenProject={() => { closeGoalWorkspace(); setTab('projects'); }} onPreviewGit={async () => { if (backendOutdated) { setDialog(reason('앱 재시작이 필요합니다', 'Git 초기화 승인 기능을 쓰려면 새 백엔드가 필요합니다.', '앱을 닫고 다시 실행하세요.')); return undefined; } return action<GitBootstrapPreview>('preview_git_bootstrap', { goalId: conversationGoalId }); }} onBootstrapGit={async (preview, selectedFiles, authorName, authorEmail, message) => action<GitBootstrapResult>('bootstrap_project_git', { goalId: conversationGoalId, fingerprint: preview.fingerprint, selectedFiles, authorName, authorEmail, message }, 'Git 첫 커밋을 만들었습니다. 목표 대화에서 검증 결과를 확인하세요.')} onRevision={async (guidanceId, intent) => { const created = await action<Goal>('create_goal_revision', { guidanceId, intent }, '새 목표 버전을 만들었습니다. 팀 실행 후 새 계획을 확인하고, 새 파일 작업이 있으면 승인해 주세요.'); if (created) { closeGoalWorkspace(); setTab('dashboard'); setNoticeGoalId(created.id); } return Boolean(created); }} />}
    {connectionDialogOpen && <ConnectionDialog onClose={() => setConnectionDialogOpen(false)} onError={message => setDialog(reason('연결을 확인해 주세요', message, '입력값과 모델 서버 상태를 확인한 뒤 다시 시도하세요.'))} onSave={async connection => Boolean(await action<AiConnection>('save_connection', { connection }, `${connection.name} 연결을 저장하고 AI가 없는 팀원에게 배정했습니다.`))} />}
    {connectionPathChange && <div className="dialog-backdrop" onClick={() => setConnectionPathChange(null)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="connection-path-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">?</div><h2 id="connection-path-title">AI 실행 경로를 변경할까요?</h2><p>등록된 {connectionPathChange.connection.name}의 실행 경로를 새로 발견한 경로로 바꿉니다. 기존 팀원 배정은 유지됩니다.</p><div className="dialog-next"><strong>현재 경로</strong><code className="path-line">{displayPath(snapshot.connections.find(item => item.id === connectionPathChange.connection.id)?.executable ?? '')}</code><strong>새 경로</strong><code className="path-line">{displayPath(connectionPathChange.connection.executable)}</code><span>새 경로의 상태: {statusLabel(connectionPathChange.connection.status)}</span></div><div className="dialog-actions"><button type="button" className="secondary" onClick={() => setConnectionPathChange(null)}>취소</button><button type="button" className="primary" autoFocus onClick={() => { const selected = connectionPathChange; setConnectionPathChange(null); void action<AiConnection>('save_connection', { connection: selected.connection }, 'AI 실행 경로를 변경했습니다. 아래에서 저장된 경로를 다시 검사하세요.'); }}>경로 변경</button></div></section></div>}
    {reviewLimitChange && <div className="dialog-backdrop" onClick={() => setReviewLimitChange(null)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="review-limit-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">?</div><h2 id="review-limit-title">QA 재작업 횟수를 변경할까요?</h2><p><strong>{reviewLimitChange.name}</strong>의 {reviewLimitChange.kind === 'project' ? '새 목표 기본값' : '이 목표의 최대 횟수'}를 {reviewLimitChange.before}회에서 {reviewLimitChange.after}회로 바꿉니다.</p><p>{reviewLimitChange.kind === 'project' ? '앞으로 만드는 목표에만 적용됩니다. 기존 목표의 한도는 유지됩니다.' : '진행 중인 목표라면 다음 QA 판단부터 적용됩니다. 이미 사용한 횟수는 유지됩니다.'}</p><div className="dialog-actions"><button type="button" className="secondary" onClick={() => setReviewLimitChange(null)}>취소</button><button type="button" className="primary" autoFocus onClick={() => { const change = reviewLimitChange; setReviewLimitChange(null); void action<Project | Goal>(change.kind === 'project' ? 'set_project_review_limit' : 'set_goal_review_limit', { id: change.id, limit: change.after }, 'QA 재작업 횟수를 저장했습니다.'); }}>변경 확인</button></div></section></div>}
    {approvalToReview && <div className="dialog-backdrop" onClick={() => setApprovalDialogId(null)}><section className="reason-dialog approval-dialog" role="dialog" aria-modal="true" aria-labelledby="approval-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">!</div><h2 id="approval-title">프로젝트 변경을 확인해 주세요</h2><p>{approvalToReview.reason}</p><PlanningOriginPanel goalId={approvalToReview.goalId} goalOrigins={snapshot.goalOrigins} projects={snapshot.projects} squads={snapshot.squads} onOpenConnections={() => { setApprovalDialogId(null); setTab('connections'); }} onError={message => setDialog(reason('작업 구조를 처리하지 못했습니다', message, '상태를 새로고침한 뒤 다시 시도하세요.'))} /><div className="dialog-actions"><button type="button" className="secondary" onClick={() => setApprovalDialogId(null)}>나중에</button><button type="button" className="secondary" onClick={() => { const id = approvalToReview.id; setApprovalDialogId(null); void action('decide_approval', { id, approved: false }, '변경을 거부했습니다.'); }}>거부</button><button type="button" className="primary" autoFocus onClick={() => { const id = approvalToReview.id; setApprovalDialogId(null); void action('decide_approval', { id, approved: true, squadDigest: (snapshot.squads ?? []).find(squad => squad.goalId === approvalToReview.goalId)?.digest ?? null }, '승인했습니다. 팀이 작업을 시작합니다.'); }}>범위 확인 후 승인</button></div></section></div>}
    {dialog && <div className="dialog-backdrop" onClick={() => setDialog(null)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">i</div><h2 id="dialog-title">{dialog.title}</h2><p>{dialog.reason}</p><div className="dialog-next"><strong>다음에 할 일</strong><span>{dialog.next}</span></div><button className="primary" autoFocus onClick={() => setDialog(null)}>알겠어요</button></section></div>}
  </div>;
}




