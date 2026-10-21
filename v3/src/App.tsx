import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { withSummaries } from '../../src/plainText';
import { BackupSettings } from '../../src/BackupSettings';
import { listen } from '@tauri-apps/api/event';
import OfficeCanvas from '../../src/office/OfficeCanvas';
import SpecImportPanel from '../../src/SpecImportPanel';
import ActionToast from '../../src/ActionToast';
import ConnectionDialog from '../../src/ConnectionDialog';
import OfficeTeamSettings from '../../src/OfficeTeamSettings';
import OfficeMemberPanel from '../../src/OfficeMemberPanel';
import { GoalChat } from '../../src/GoalChat';
import { V3ChatHome } from './V3ChatHome';
import { OfficeWindow } from '../../src/OfficeWindow';
import { SafeBoundary } from '../../src/SafeBoundary';
import { progressLine } from '../../src/goalStream';
import { blockFacts } from '../../src/deliverables';
import { goalStageLabel } from '../../src/goalBrief';
import { ExternalStaffPanel } from '../../src/ExternalStaffPanel';
import { officeBadges, squadFor } from '../../src/squad';
import { MeetingPanel } from '../../src/MeetingPanel';
import { meetingAttentionFor } from '../../src/meetingStatus';
import { meetingBackendOutdated } from '../../src/backendProtocol';
import type { GitBootstrapPreview, GitBootstrapResult } from '../../src/GitBootstrapPanel';
import { OfficeGoalRail } from '../../src/OfficeGoalRail';
import { memberActivity } from '../../src/progress';
import { displayPath } from '../../src/paths';
import { connectionRecovery } from '../../src/providerRecovery';
import { openWork } from '../../src/openWork';
import { OpenWorkLauncher } from '../../src/OpenWorkLauncher';
import TokenUsageView from '../../src/TokenUsageView';
import '../../src/ux.css';
import type { Agent, AiConnection, DiscoveredAi, Goal, GoalTurn, Meeting, MeetingPlan, MeetingReadiness, PlanInput, Project, Reference, RolePreset, Snapshot } from '../../src/types';
// v3 shell (ADR-0039): same five menus, controller copied from v2 (migration debt recorded in v3/README.md).
import { V2Sidebar, V2SettingsTabs, labels, type V2Tab, type SettingsSection } from '../../v2/src/V2Navigation';
import { V3Home, type HomeMode } from './V3Home';
import { V2Work } from '../../v2/src/V2Work';
import { V2Connections } from '../../v2/src/V2Connections';
import { V2Projects } from '../../v2/src/V2Projects';
import { V2NotificationSettings } from '../../v2/src/V2NotificationSettings';
import { useAttentionNotifications } from '../../src/useAttentionNotifications';
import { useReferenceTray } from '../../src/ReferenceTray';
import { goalReferences, stagedReferences } from '../../src/referenceAttach';
import { conversationGoals, goalChain } from '../../src/goalChain';

type Tab = V2Tab;
type Explanation = { title: string; reason: string; next: string };
const empty: Snapshot = { project: null, projects: [], agents: [], connections: [], goals: [], goalTurns: [], goalGuidance: [], tasks: [], events: [], approvals: [], usage: [], specProfiles: [], specDrafts: [] };

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

export default function App() {
  const [tab, setTab] = useState<Tab>('dashboard');
  const [settingsSection, setSettingsSection] = useState<SettingsSection>('projects');
  const [snapshot, setSnapshot] = useState<Snapshot>(empty);
  const [snapshotReady, setSnapshotReady] = useState(false);
  const [discovered, setDiscovered] = useState<DiscoveredAi[]>([]);
  const [presets, setPresets] = useState<RolePreset[]>([]);
  const [busy, setBusy] = useState(false);
  const [backendOutdated, setBackendOutdated] = useState(false);
  const [backendBuild, setBackendBuild] = useState('');
  const [dialog, setDialog] = useState<Explanation | null>(null);
  const [reviewLimitChange, setReviewLimitChange] = useState<{ kind: 'project' | 'goal'; id: string; name: string; before: number; after: number } | null>(null);
  const [notice, setNotice] = useState('');
  const [noticeGoalId, setNoticeGoalId] = useState<string | null>(null);
  const [connectionDialogOpen, setConnectionDialogOpen] = useState(false);
  const [connectionPathChange, setConnectionPathChange] = useState<DiscoveredAi | null>(null);
  const [intent, setIntent] = useState('');
  // ADR-0049: material attached in the composer, held by the backend until the goal is sent.
  const [stagedIds, setStagedIds] = useState<string[]>([]);
  const attach = useReferenceTray({
    references: stagedReferences(snapshot.references, stagedIds),
    onStageText: async (name, text, source) => { const staged = await action<Reference>('stage_text_reference', { name, text, source }); if (staged) setStagedIds(ids => [...ids, staged.id]); return staged; },
    onStageImage: async image => { const staged = await action<Reference>('stage_image_reference', image); if (staged) setStagedIds(ids => [...ids, staged.id]); return staged; },
    onDiscard: id => { setStagedIds(ids => ids.filter(item => item !== id)); void action('discard_reference', { id }); },
    onCaption: (id, caption) => void action<Reference>('set_reference_caption', { id, caption }),
    onRole: (id, role) => void action<Reference>('set_reference_role', { id, role }),
    curatorEnabled: Boolean(snapshot.curatorEnabled),
    onCurator: enabled => void action('set_curator_enabled', { enabled }, enabled ? '자료 정리 담당을 켰어요. 새로 첨부하는 긴 글부터 정리해요.' : '자료 정리 담당을 껐어요. 규칙으로만 정리해요.'),
    onInsert: text => setIntent(current => current ? `${current}\n\n${text}` : text),
    onError: message => setDialog(reason('자료를 첨부하지 못했습니다', message, '파일 형식과 크기를 확인해 주세요.')),
  });
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const [detailMode, setDetailMode] = useState<'activity' | 'settings'>('activity');
  const [agentDraft, setAgentDraft] = useState<Agent | null>(null);
  const [presetId, setPresetId] = useState('');
  const [conversationGoalId, setConversationGoalId] = useState<string | null>(null);
  const [officeOpen, setOfficeOpen] = useState(false);
  const [homeMode, setHomeMode] = useState<HomeMode>('goal');
  const [meetingReadiness, setMeetingReadiness] = useState<MeetingReadiness | null>(null);
  const [meetingReadinessError, setMeetingReadinessError] = useState('');
  const [draftSource, setDraftSource] = useState<{ meetingId: string; packageSha256: string } | null>(null);
  const [conversationTaskId, setConversationTaskId] = useState<string | null>(null);

  function openSettings(section: SettingsSection) { setSettingsSection(section); setTab('settings'); }

  const refreshSequence = useRef(0);
  const attention = useAttentionNotifications(snapshot, snapshotReady);

  const refresh = useCallback(async () => {
    const sequence = ++refreshSequence.current;
    const next = await invoke<Snapshot>('get_snapshot');
    if (sequence === refreshSequence.current) { setSnapshot(withSummaries(next)); setSnapshotReady(true); }
  }, []);
  const refreshDiscovery = useCallback(async () => { setDiscovered(await invoke<DiscoveredAi[]>('discover_ai')); }, []);
  useEffect(() => {
    void invoke<{ protocolVersion: number; buildId?: string }>('get_backend_info').then(info => { setBackendOutdated(meetingBackendOutdated(info.protocolVersion)); setBackendBuild(info.buildId ?? ''); }).catch(() => setBackendOutdated(true));
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

  const visibleGoals = useMemo(() => snapshot.goals.filter(goal => goal.projectId === snapshot.project?.id || goal.projectId === null), [snapshot.goals, snapshot.project]);
  const goalIds = useMemo(() => new Set(visibleGoals.map(goal => goal.id)), [visibleGoals]);
  const visibleTasks = snapshot.tasks.filter(task => goalIds.has(task.goalId));
  const visibleApprovals = snapshot.approvals.filter(approval => approval.goalId && goalIds.has(approval.goalId));
  const visibleEvents = snapshot.events.filter(event => event.goalId === null || goalIds.has(event.goalId));
  const projectUsage = (snapshot.usage ?? []).filter(item => item.projectId === snapshot.project?.id);
  const openWorkItems = useMemo(() => openWork(snapshot), [snapshot]);
  const activities = useMemo(() => Object.fromEntries(snapshot.agents.map(agent => [agent.id, memberActivity(agent, snapshot.agents, visibleGoals, visibleTasks, visibleEvents)])), [snapshot.agents, visibleGoals, visibleTasks, visibleEvents]);

  function selectAgent(id: string) {
    const agent = snapshot.agents.find(item => item.id === id);
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
    if (saved) { setSelectedAgentId(saved.id); setAgentDraft(saved); setPresetId('specialist'); setDetailMode('settings'); }
  }

  async function checkSavedConnection(id: string) {
    const checked = await action<DiscoveredAi>('check_connection', { id });
    if (checked) setDialog(reason(`${checked.connection.name} 상태: ${statusLabel(checked.connection.status)}`, checked.detail, connectionRecovery(checked.connection)));
  }

  async function openConnectionLogin(id: string) {
    const message = await action<string>('start_connection_login', { id });
    if (message) setDialog(reason('로그인 창을 열었습니다', message, '로그인을 마친 뒤 AI 연결에서 “이 경로로 다시 검사”를 눌러 상태를 확인하세요.'));
  }

  async function archiveGoal(goalId: string): Promise<boolean> {
    const done = (await action('archive_goal', { goalId }, '보관했어요. 설정 › 백업 삭제에서 되돌리거나 영구 삭제할 수 있어요.')) !== undefined;
    if (done) closeGoalWorkspace();
    return done;
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
    // ADR-0046: an execution goal opens as the conversation on Home; a meeting keeps its own panel.
    if (snapshot.goals.find(goal => goal.id === goalId)?.kind !== 'deliberation') setTab('dashboard');
  }
  function closeGoalWorkspace() {
    setConversationGoalId(null);
    setConversationTaskId(null);
  }
  async function meetingCommand<T>(command: string, args: Record<string, unknown>): Promise<T> {
    if (backendOutdated) throw new Error('새 백엔드를 실행한 뒤 회의를 진행해 주세요');
    try { return await invoke<T>(command, args); } finally { await refresh().catch(() => {}); }
  }
  const meetingHandlers = {
    onReadiness: () => invoke<MeetingReadiness>('meeting_readiness'),
    onUpdatePlan: (meetingId: string, baseRevision: number, plan: PlanInput) => meetingCommand<MeetingPlan>('update_meeting_plan', { meetingId, baseRevision, plan }),
    onApprovePlan: (meetingId: string, revision: number, digest: string, mode: 'plan' | 'per_stage') => meetingCommand<Meeting>('approve_meeting_plan', { meetingId, revision, digest, mode }).then(() => undefined),
    onRejectPlan: (meetingId: string) => meetingCommand<void>('reject_meeting_plan', { meetingId }),
    onRevokePlan: (meetingId: string) => meetingCommand<void>('revoke_meeting_plan', { meetingId }),
    onDecide: (approvalId: string, previewRevision: number, payloadSha256: string, approved: boolean, confirmDuplicate: boolean) => meetingCommand<void>('decide_transmission', { approvalId, previewRevision, payloadSha256, approved, confirmDuplicate }),
    onDispatch: (meetingId: string) => meetingCommand<void>('dispatch_meeting', { meetingId }),
    onStop: (meetingId: string) => meetingCommand<void>('stop_meeting', { meetingId }),
    onResume: (meetingId: string) => meetingCommand<void>('resume_meeting', { meetingId }),
    onRetryPacket: (meetingId: string, packetId: string, confirmDuplicate: boolean) => meetingCommand<void>('retry_meeting_packet', { meetingId, packetId, confirmDuplicate }),
    onContinueSourceSnapshot: (meetingId: string) => meetingCommand<void>('continue_meeting_source_snapshot', { meetingId }),
    onClosePartial: (meetingId: string) => meetingCommand<void>('close_meeting_partial', { meetingId }),
    onDecision: (meetingId: string, kind: 'adopt' | 'revise' | 'defer', recommendationIds: string[], note: string) => meetingCommand<string | null>('record_meeting_decision', { meetingId, kind, recommendationIds, note }),
    onDraft: (draft: string, meetingId: string, packageSha256: string) => { setIntent(draft); setHomeMode('goal'); setDraftSource({ meetingId, packageSha256 }); closeGoalWorkspace(); setTab('dashboard'); setNotice('채택한 권고를 홈의 목표 수행 초안에 넣었습니다. 고친 뒤 직접 전달해야 목표가 만들어집니다.'); },
  };
  useEffect(() => {
    if (homeMode !== 'meeting' || backendOutdated) { if (backendOutdated && homeMode === 'meeting') setMeetingReadinessError('실행 중인 백엔드가 이전 버전입니다. 앱을 다시 시작하세요.'); return; }
    let live = true;
    invoke<MeetingReadiness>('meeting_readiness').then(ready => { if (live) { setMeetingReadiness(ready); setMeetingReadinessError(''); } }).catch(error => { if (live) { setMeetingReadiness(null); setMeetingReadinessError(String(error)); } });
    return () => { live = false; };
  }, [homeMode, backendOutdated, snapshot]);
  async function startMeetingFromHome() {
    const created = await action<Meeting>('start_meeting_from_home', { topic: intent }, '의장에게 회의 계획을 요청했습니다. 계획이 오면 검토해 주세요.');
    if (created) { setIntent(''); setNoticeGoalId(created.goalId); openGoalWorkspace(created.goalId); }
  }
  const meetingStatus = (goal: Goal) => meetingAttentionFor(goal, snapshot);
  // ADR-0051: a goal and the versions that replaced it are one conversation; it always shows the newest version.
  const homeChain = conversationGoalId ? goalChain(conversationGoalId, snapshot.goalGuidance ?? [], snapshot.goals) : [];
  const homeGoal = snapshot.goals.find(goal => goal.id === homeChain[homeChain.length - 1] && goal.kind !== 'deliberation') ?? null;
  const homeEarlier = homeChain.slice(0, -1).map(id => snapshot.goals.find(goal => goal.id === id)).filter((goal): goal is Goal => Boolean(goal));
  const chatGoals = conversationGoals(visibleGoals, snapshot.goalGuidance ?? []);
  const needsDecision = (goal: Goal) => goal.kind === 'deliberation' ? meetingStatus(goal).needsDecision : Boolean(blockFacts(goal, snapshot.tasks, snapshot.approvals, snapshot.goalGuidance ?? [], snapshot.agents, visibleEvents));
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

  return <div className="shell v2 v3">
    <V2Sidebar version="v3" compact tab={tab} project={snapshot.project} projects={snapshot.projects} onTab={setTab} onProject={project => { if (snapshot.project?.id !== project.id) void action<Project>('switch_project', { id: project.id }, `${project.name} 프로젝트로 전환했습니다.`); }} onAddProject={() => openSettings('projects')} />
    <main className="main"><header className="topbar"><div className="breadcrumb">WORKSPACE <span>/</span> {labels[tab]}{tab === 'settings' ? ` / ${settingsSection}` : ''}</div><div className="topbar-right"><button type="button" className="secondary" aria-pressed={officeOpen} onClick={() => setOfficeOpen(value => !value)}>Office 창</button><span className="platform">{snapshot.project?.name ?? '설정 시작'}</span><span className="avatar">CEO</span></div></header><div className="main-scroll">
      {backendOutdated && <div className="restart-banner" role="alert"><strong>앱을 다시 시작해야 최신 업무 진행 표시가 적용됩니다.</strong><span>현재 화면은 새 코드지만 실행 중인 백엔드는 이전 버전입니다.</span><button type="button" className="secondary" onClick={() => setDialog(reason('앱 재시작 방법', '실행 중인 창은 이전 Rust 백엔드를 사용하고 있습니다.', '앱 창을 닫고 PowerShell에서 .\\v3\\start-windows.ps1을 다시 실행하세요.'))}>재시작 방법</button></div>}
      {notice && <ActionToast message={notice} actionLabel={noticeGoalId ? '목표 보기' : undefined} onAction={noticeGoalId ? () => openGoalWorkspace(noticeGoalId) : undefined} onDismiss={() => setNotice('')} />}

      {tab === 'dashboard' && <V3ChatHome goals={chatGoals} selectedId={homeGoal?.id ?? null} needsDecision={needsDecision} stageLabel={goal => goal.kind === 'deliberation' ? `회의 · ${meetingStatus(goal).label}` : goalStageLabel(goal.status)} onSelect={openGoalWorkspace} onNew={closeGoalWorkspace}>{homeGoal ? <GoalChat layout="panel" key={`${homeGoal.id}:${conversationTaskId ?? ''}`} focusTaskId={conversationTaskId} goal={homeGoal} projectKind={snapshot.projects.find(project => project.id === homeGoal.projectId)?.kind ?? 'folder'} earlier={homeEarlier} turns={(snapshot.goalTurns ?? []).filter(turn => homeChain.includes(turn.goalId))} guidance={(snapshot.goalGuidance ?? []).filter(item => homeChain.includes(item.goalId) || (item.replacementGoalId !== null && homeChain.includes(item.replacementGoalId)))} agents={snapshot.agents} connections={snapshot.connections} tasks={visibleTasks} approvals={visibleApprovals} events={visibleEvents} usage={projectUsage} reviews={snapshot.deliverableReviews ?? []} runBlock={recoveryRunBlock(homeGoal)} squad={squadFor(homeGoal.id, snapshot.squads)} squads={snapshot.squads} summary={snapshot.goalSummaries?.find(item => item.goalId === homeGoal.id)?.text ?? null} goalOrigins={snapshot.goalOrigins} projects={snapshot.projects} references={goalReferences(homeGoal.id, snapshot)} onClose={closeGoalWorkspace} onArchive={() => archiveGoal(homeGoal.id)} onSend={async (message, agentId) => { if (backendOutdated) { setDialog(reason('앱 재시작이 필요합니다', '실행 중인 백엔드는 자연어 메시지 라우팅을 지원하지 않습니다.', '앱 창을 닫고 시작 스크립트로 다시 실행하세요.')); return false; } return Boolean(await action<GoalTurn>('send_goal_message', { goalId: homeGoal.id, agentId, message }, '메시지를 전달했습니다.')); }} onOpenGoal={openGoalWorkspace} onRetry={async () => Boolean(await action('run_goal', { goalId: homeGoal.id }, 'AI 팀이 분석을 시작했습니다.'))} onRequestFix={async () => Boolean(await action<GoalTurn>('request_team_fix', { goalId: homeGoal.id }, '해결을 요청했습니다. 팀이 이어서 진행합니다.'))} onResume={async () => Boolean(await action('resume_goal', { goalId: homeGoal.id }, '승인된 계획을 다시 시작했습니다.'))} onDecide={async (id, approved, squadDigest) => Boolean(await action('decide_approval', approved ? { id, approved, squadDigest } : { id, approved }, approved ? '승인했습니다. 팀이 작업을 시작합니다.' : '계획을 거부했습니다.'))} onRevise={async (approvalId, note) => Boolean(await action<GoalTurn>('revise_plan', { goalId: homeGoal.id, approvalId, note }, '개선 방침을 보냈습니다. 팀이 같은 목표에서 계획을 다시 세웁니다.'))} onCancel={async approvalId => Boolean(await action('cancel_goal', { goalId: homeGoal.id, approvalId }, '계획을 거부하고 목표를 취소했습니다. 파일은 바뀌지 않았습니다.'))} onRestart={async intent => { const created = await action<Goal>('submit_goal', { intent, lineageMeetingId: null, lineagePackageSha256: null }, '같은 내용으로 새 목표를 만들었습니다.'); if (!created) return false; openGoalWorkspace(created.id); if (!recoveryRunBlock(created)) await action('run_goal', { goalId: created.id }, 'AI 팀이 분석을 시작했습니다.'); return true; }} onSwapSquad={(from, to) => void action('swap_squad_member', { goalId: homeGoal.id, fromAgentId: from, toAgentId: to }, '스쿼드 팀원을 바꿨습니다. 바뀐 구성을 확인한 뒤 승인하세요.')} onAddSquad={(slot, agentId, taskId) => void action('add_squad_member', { goalId: homeGoal.id, slot, agentId, taskId }, '빈 자리를 채웠습니다. 바뀐 구성을 확인한 뒤 승인하세요.')} onOpenConnections={() => openSettings('connections')} onOpenOffice={() => setOfficeOpen(true)} onOpenTeam={() => openSettings('team')} onOpenProject={() => openSettings('projects')} onError={message => setDialog(reason('처리하지 못했습니다', message, '상태를 새로고침한 뒤 다시 시도하세요.'))} onPreviewGit={async () => { if (backendOutdated) { setDialog(reason('앱 재시작이 필요합니다', 'Git 초기화 승인 기능을 쓰려면 새 백엔드가 필요합니다.', '앱을 닫고 다시 실행하세요.')); return undefined; } return action<GitBootstrapPreview>('preview_git_bootstrap', { goalId: homeGoal.id }); }} onBootstrapGit={async (preview, selectedFiles, authorName, authorEmail, message) => action<GitBootstrapResult>('bootstrap_project_git', { goalId: homeGoal.id, fingerprint: preview.fingerprint, selectedFiles, authorName, authorEmail, message }, 'Git 첫 커밋을 만들었습니다. 목표 대화에서 검증 결과를 확인하세요.')} onRevision={async (guidanceId, intent) => { const created = await action<Goal>('create_goal_revision', { guidanceId, intent }, '새 목표 버전을 만들었습니다.'); if (created) { setNoticeGoalId(created.id); openGoalWorkspace(created.id); } return Boolean(created); }} /> : <V3Home hideGoalList attachments={attach.tray} onPasteLong={text => void attach.paste(text)} onDropFiles={files => void attach.addFiles(files)} guidance={snapshot.goalGuidance ?? []} events={visibleEvents} reviews={snapshot.deliverableReviews ?? []} connections={snapshot.connections} teamBlock={teamRunBlock(snapshot)} onConnections={() => openSettings('connections')} mode={homeMode} onMode={setHomeMode} readiness={meetingReadiness} readinessError={meetingReadinessError} onStartMeeting={() => void startMeetingFromHome()} draftSource={draftSource?.packageSha256 ?? null} onClearDraftSource={() => setDraftSource(null)} meetingStatus={meetingStatus} project={snapshot.project} goals={visibleGoals} tasks={visibleTasks} approvals={visibleApprovals} agents={snapshot.agents} intent={intent} busy={busy} onIntent={setIntent} onSubmit={() => void action<Goal>('submit_goal', { intent, lineageMeetingId: draftSource?.meetingId ?? null, lineagePackageSha256: draftSource?.packageSha256 ?? null, referenceIds: stagedReferences(snapshot.references, stagedIds).map(item => item.id) }, '목표를 팀에 전달했습니다.').then(created => { if (created) { setIntent(''); setDraftSource(null); setStagedIds([]); setNoticeGoalId(created.id); openGoalWorkspace(created.id); if (!recoveryRunBlock(created)) void action('run_goal', { goalId: created.id }, 'AI 팀이 분석을 시작했습니다.'); } })} onRun={id => void action('run_goal', { goalId: id }, 'AI 팀이 분석을 시작했습니다.')} onOpen={openGoalWorkspace} onSettings={() => openSettings('projects')} onOffice={() => openSettings('team')} onExplain={(title, why, next) => setDialog(reason(title, why, next))} runBlock={goal => { const blocked = busyReason ?? (backendOutdated ? reason('앱 재시작이 필요합니다', '실행 중인 백엔드가 이전 버전입니다.', 'v3 시작 스크립트(v3\\start-windows.ps1)로 다시 실행하세요.') : !goal.projectId ? reason('프로젝트 연결이 없습니다', '목표의 작업 폴더가 없습니다.', '프로젝트를 확인하세요.') : teamRunBlock(snapshot)); return blocked; }} />}</V3ChatHome>}
      {tab === 'board' && <V2Work meetingStatus={meetingStatus} goals={visibleGoals} tasks={visibleTasks} agents={snapshot.agents} approvals={visibleApprovals} events={visibleEvents} onOpen={openGoalWorkspace} onSaveGroup={(id, group) => void action<Goal>('set_goal_work_group', { id, group }, '업무 묶음을 저장했습니다.')} onApprove={id => { const goalId = snapshot.approvals.find(item => item.id === id)?.goalId; if (goalId) openGoalWorkspace(goalId); }} reviewControl={goalReviewLimitDropdown} />}

      {tab === 'usage' && <TokenUsageView snapshot={snapshot} />}

      {tab === 'settings' && <div className="page"><div className="v2-heading"><div><div className="eyebrow">WORKSPACE SETTINGS</div><h1>설정</h1><p>프로젝트, AI 연결, 팀 구성을 한곳에서 관리합니다.</p></div></div><V2SettingsTabs section={settingsSection} onSection={setSettingsSection}/>
      {settingsSection === 'projects' && <V2Projects projects={snapshot.projects} current={snapshot.project} goals={snapshot.goals} busy={busy} reviewControl={reviewLimitDropdown} onSelect={project => void action<Project>('switch_project', { id: project.id }, `${project.name} 프로젝트로 전환했습니다.`)} onOpenGoal={(goal, project) => { if (snapshot.project?.id !== project.id) void action<Project>('switch_project', { id: project.id }).then(switched => { if (switched) openGoalReview(goal); }); else openGoalReview(goal); }} onRegister={path => void action<Project>('set_project', { path }, '프로젝트를 준비하고 선택했습니다.')} onExplain={(title, why, next) => setDialog(reason(title, why, next))} />}
      {settingsSection === 'connections' && <V2Connections saved={snapshot.connections} discovered={discovered} agentCounts={Object.fromEntries(snapshot.connections.map(connection => [connection.id, snapshot.agents.filter(agent => agent.connectionId === connection.id).length]))} busy={busy} outdated={backendOutdated} onDiscover={() => void refreshDiscovery().then(() => setNotice('설치된 AI를 다시 검사했습니다.')).catch(error => setDialog(reason('AI 검색 실패', String(error), 'CLI가 실행되는지 확인하세요.')))} onCustom={() => setConnectionDialogOpen(true)} onSave={item => void action<AiConnection>('save_connection', { connection: item.connection }, `${item.connection.name} 연결을 저장하고 AI가 없는 팀원에게 배정했습니다.`)} onChangePath={setConnectionPathChange} onCheck={id => void checkSavedConnection(id)} onLogin={id => void openConnectionLogin(id)} onExplain={(title, detail, next) => setDialog(reason(title, detail, next))} onAddMock={modes => void action<AiConnection>('add_mock_connection', { modes }, '모의 AI를 등록하고 AI가 없는 팀원에게 배정했습니다. 다른 팀원은 Office 팀 설정에서 바꿀 수 있어요.')} />}
      {settingsSection === 'spec' && <div className="v2-settings-narrow"><SpecImportPanel key={snapshot.project?.id ?? 'no-project'} project={snapshot.project} agents={snapshot.agents} profile={snapshot.specProfiles?.find(item => item.projectId === snapshot.project?.id)} savedDraft={[...(snapshot.specDrafts ?? [])].filter(item => item.projectId === snapshot.project?.id).sort((a, b) => b.createdAt.localeCompare(a.createdAt))[0]} busy={busy} backendOutdated={backendOutdated} onApplied={refresh} onError={message => setDialog(reason('명세를 가져오지 못했습니다', message, '파일 형식과 대상 프로젝트를 확인하세요.'))} /></div>}
      {settingsSection === 'team' && <OfficeTeamSettings agents={snapshot.agents} connections={snapshot.connections} tasks={snapshot.tasks} importedSpec={snapshot.specProfiles?.find(item => item.projectId === snapshot.project?.id)} busy={busy} onAdd={() => void addAgent()} onAssign={async (connectionId, agentIds) => Boolean(await action<number>('assign_team_connection', { connectionId, agentIds }, `${agentIds.length}명의 AI 연결을 변경했습니다.`))} onHide={agent => void hideAgent(agent)} onRestore={agent => { void action<Agent>('save_agent', { agent: { ...agent, enabled: true } }, `${agent.name}을(를) 다시 배치했습니다.`); }} onExplain={(title, detail, next) => setDialog(reason(title, detail, next))} />}
      {settingsSection === 'backup' && <BackupSettings archived={snapshot.archivedGoals ?? []} busy={busy} onRestore={async goalId => (await action('restore_goal', { goalId }, '되돌렸어요. 다시 화면에 보여요.')) !== undefined} onPurge={async goalId => (await action<number>('purge_goal', { goalId, confirmed: true }, '영구 삭제했어요.')) !== undefined} />}
      {settingsSection === 'notifications' && <V2NotificationSettings preferences={attention.preferences} error={attention.error} pending={attention.pending} onSystem={enabled => void attention.setSystem(enabled)} onSound={attention.setSound} onTestSystem={attention.testSystem} onTestSound={attention.testSound} />}
      </div>}
      {tab === 'office' && <div className="page"><div className="v2-heading"><div><div className="eyebrow">LIVE OFFICE</div><h1>Office</h1><p>팀원이 지금 무엇을 하는지 확인하고, 캐릭터를 눌러 개인 설정을 열어 보세요.</p></div><button className="secondary" onClick={() => openSettings('team')}>팀 전체 설정</button></div><div className="v2-office-rail">
          <OfficeGoalRail goals={visibleGoals} tasks={visibleTasks} agents={snapshot.agents} approvals={visibleApprovals} onOpen={openGoalWorkspace} onReview={openGoalReview} meetingStatus={meetingStatus} meetings={snapshot.meetings ?? []} meetingPackets={snapshot.meetingPackets ?? []} guidance={snapshot.goalGuidance ?? []} events={visibleEvents} />
          </div>
          <div className="office-layout">
            <section className="office"><div className="office-header"><strong>UTOPIA COMPANY</strong><span><span className="online-dot" /> TEAM STATE</span></div><div className="office-room"><OfficeCanvas agents={snapshot.agents} connections={snapshot.connections} activities={activities} events={visibleEvents} badges={officeBadges(snapshot.squads, snapshot.goals, snapshot.agents)} selectedId={selectedAgentId} onSelect={selectAgent} onMove={(id, x, y) => void moveAgent(id, x, y)} /></div><div className="office-hint">캐릭터 클릭: 활동·개인 설정 · 드래그: 자리 이동 · 방향키: 조금 이동</div></section>
            <OfficeMemberPanel agents={snapshot.agents} connections={snapshot.connections} presets={presets} selectedId={selectedAgentId} draft={agentDraft} activity={selectedAgentId ? activities[selectedAgentId] : undefined} tasks={visibleTasks} usage={projectUsage} mode={detailMode} presetId={presetId} busy={busy} onSelect={selectAgent} onMode={setDetailMode} onDraft={setAgentDraft} onPreset={setPresetId} onSave={() => { if (agentDraft) void action('save_agent', { agent: agentDraft }, `${agentDraft.name} 설정을 저장했습니다.`); }} onExplain={(title, detail, next) => setDialog(reason(title, detail, next))} onNotice={setNotice} onOpenGoal={openGoalWorkspace} />
          </div>
          <ExternalStaffPanel agents={snapshot.agents} connections={snapshot.connections} busy={busy} onAction={action} onError={message => setDialog(reason('캐릭터 파일을 확인해 주세요', message, 'U Company 캐릭터 파일(format: ucompany.character, version 1)인지 확인하세요.'))} />
          <div className="office-note">Office는 저장된 팀과 업무 상태를 표시합니다. 배경 좌표 투영 코드는 Pixel Agents의 MIT 코드에서 가져왔고, 캐릭터는 JIK-A-4 Metro City(CC0) 그림을 넣으면 그 그림을, 없으면 자체 픽셀 도형을 씁니다(ADR-0040).</div>
      </div>}
      </div>
    </main>
    {officeOpen && <SafeBoundary label="Office 창" onClose={() => setOfficeOpen(false)}><OfficeWindow goal={homeGoal} agents={snapshot.agents} connections={snapshot.connections} tasks={visibleTasks} events={visibleEvents} squad={homeGoal ? squadFor(homeGoal.id, snapshot.squads) : null} activities={activities} badges={officeBadges(snapshot.squads, snapshot.goals, snapshot.agents)} progress={homeGoal ? progressLine(homeGoal, visibleTasks, snapshot.agents, visibleEvents, squadFor(homeGoal.id, snapshot.squads)) : null} selectedId={selectedAgentId} onSelect={selectAgent} onMove={(id, x, y) => void moveAgent(id, x, y)} turns={homeGoal ? (snapshot.goalTurns ?? []).filter(turn => turn.goalId === homeGoal.id) : []} onAsk={async (agentId, question) => { if (!homeGoal) return false; return Boolean(await action<GoalTurn>('ask_goal', { goalId: homeGoal.id, agentId, question })); }} onClose={() => setOfficeOpen(false)} /></SafeBoundary>}
    <OpenWorkLauncher items={openWorkItems} agents={snapshot.agents} onSelect={(goalId, taskId) => void openWorkItem(goalId, taskId)} />
    {conversationGoalId && snapshot.goals.find(goal => goal.id === conversationGoalId)?.kind === 'deliberation' && <MeetingPanel snapshot={snapshot} goal={snapshot.goals.find(goal => goal.id === conversationGoalId)!} onArchive={() => archiveGoal(conversationGoalId)} backendBuild={backendBuild} onClose={closeGoalWorkspace} {...meetingHandlers} />}
    {connectionDialogOpen && <ConnectionDialog onClose={() => setConnectionDialogOpen(false)} onError={message => setDialog(reason('연결을 확인해 주세요', message, '입력값과 모델 서버 상태를 확인한 뒤 다시 시도하세요.'))} onSave={async connection => Boolean(await action<AiConnection>('save_connection', { connection }, `${connection.name} 연결을 저장하고 AI가 없는 팀원에게 배정했습니다.`))} />}
    {connectionPathChange && <div className="dialog-backdrop" onClick={() => setConnectionPathChange(null)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="connection-path-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">?</div><h2 id="connection-path-title">AI 실행 경로를 변경할까요?</h2><p>등록된 {connectionPathChange.connection.name}의 실행 경로를 새로 발견한 경로로 바꿉니다. 기존 팀원 배정은 유지됩니다.</p><div className="dialog-next"><strong>현재 경로</strong><code className="path-line">{displayPath(snapshot.connections.find(item => item.id === connectionPathChange.connection.id)?.executable ?? '')}</code><strong>새 경로</strong><code className="path-line">{displayPath(connectionPathChange.connection.executable)}</code><span>새 경로의 상태: {statusLabel(connectionPathChange.connection.status)}</span></div><div className="dialog-actions"><button type="button" className="secondary" onClick={() => setConnectionPathChange(null)}>취소</button><button type="button" className="primary" autoFocus onClick={() => { const selected = connectionPathChange; setConnectionPathChange(null); void action<AiConnection>('save_connection', { connection: selected.connection }, 'AI 실행 경로를 변경했습니다. 아래에서 저장된 경로를 다시 검사하세요.'); }}>경로 변경</button></div></section></div>}
    {reviewLimitChange && <div className="dialog-backdrop" onClick={() => setReviewLimitChange(null)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="review-limit-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">?</div><h2 id="review-limit-title">QA 재작업 횟수를 변경할까요?</h2><p><strong>{reviewLimitChange.name}</strong>의 {reviewLimitChange.kind === 'project' ? '새 목표 기본값' : '이 목표의 최대 횟수'}를 {reviewLimitChange.before}회에서 {reviewLimitChange.after}회로 바꿉니다.</p><p>{reviewLimitChange.kind === 'project' ? '앞으로 만드는 목표에만 적용됩니다. 기존 목표의 한도는 유지됩니다.' : '진행 중인 목표라면 다음 QA 판단부터 적용됩니다. 이미 사용한 횟수는 유지됩니다.'}</p><div className="dialog-actions"><button type="button" className="secondary" onClick={() => setReviewLimitChange(null)}>취소</button><button type="button" className="primary" autoFocus onClick={() => { const change = reviewLimitChange; setReviewLimitChange(null); void action<Project | Goal>(change.kind === 'project' ? 'set_project_review_limit' : 'set_goal_review_limit', { id: change.id, limit: change.after }, 'QA 재작업 횟수를 저장했습니다.'); }}>변경 확인</button></div></section></div>}
    {dialog && <div className="dialog-backdrop" onClick={() => setDialog(null)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">i</div><h2 id="dialog-title">{dialog.title}</h2><p>{dialog.reason}</p><div className="dialog-next"><strong>다음에 할 일</strong><span>{dialog.next}</span></div><button className="primary" autoFocus onClick={() => setDialog(null)}>알겠어요</button></section></div>}
  </div>;
}





