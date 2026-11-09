import { useEffect, useReducer, useRef, useState, type Dispatch, type KeyboardEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { tabKeyTarget } from '../../src/shell/V2Navigation';
import type { Agent, Approval, Goal, Project, TechAiDraftRequest, TechDraft, TechNumericTarget, TechPick, TechPlanSnapshot, TechPreset, TechPresetState, TechPreview, TechStage } from '../../src/types';

const stages: { id: TechStage; title: string; question: string }[] = [
  { id: 'beginner', title: '초급', question: '어떤 기술로 만들까요?' },
  { id: 'intermediate', title: '중급', question: '코드를 어떻게 나눌까요?' },
  { id: 'advanced', title: '고급', question: '어떤 운영 목표에 맞출까요?' },
];
/** Shown with no project: the engine's built-in names (technology_presets.rs builtins), never a selection. */
export const builtinOutline: Record<TechStage, string[]> = {
  beginner: ['웹 앱', 'API 서버', '크로스플랫폼 앱'],
  intermediate: ['단일 앱', '모듈형 단일 앱', '서비스 분리'],
  advanced: ['응답 지연 최소화', '대량 처리', '고가용성'],
};
const statusText: Record<string, string> = { pass: '통과', question: '질문 필요', conflict: '충돌', unmeasured: '미검증' };
const emptyPick = (stage: TechStage): TechPick => ({ stage, values: {} });
export function selectionChanged(state: TechPresetState, plan: TechPlanSnapshot): boolean {
  if (!plan.applicable) return false;
  return stages.some(({ id }) => {
    const selected = state.selections.find(item => item.stage === id);
    const fixed = plan.choices.find(item => item.stage === id);
    return (selected?.presetId ?? '') !== (fixed?.preset.id ?? '') || (selected?.version ?? '') !== (fixed?.preset.version ?? '') || (selected?.digest ?? '') !== (fixed?.digest ?? '') || JSON.stringify(selected?.values ?? {}) !== JSON.stringify(fixed?.values ?? {});
  });
}
export function canApplyPreview(preview: TechPreview | null, projectId: string): boolean {
  return !!preview && preview.projectId === projectId && !['question', 'conflict'].includes(preview.report.status);
}
export function canApproveAiRequest(request: TechAiDraftRequest | null, projectId: string): boolean {
  return !!request && request.projectId === projectId && request.status === 'previewed';
}
export function planFingerprintForApproval(plan: TechPlanSnapshot | null, approvalId: string, projectId: string, goalId: string): string | null {
  return plan?.approvalId === approvalId && plan.projectId === projectId && plan.goalId === goalId && plan.fingerprint ? plan.fingerprint : null;
}
/** A loaded state, preview, AI request or plan belongs to exactly one project; anything else is dropped. */
export function forProject<T extends { projectId: string }>(value: T | null, projectId: string): T | null {
  return value && value.projectId === projectId ? value : null;
}
/** Plain-language active choice per stage, from the saved selection and its catalog name only. */
export function selectionSummary(state: TechPresetState): { stage: TechStage; title: string; label: string; version: string | null }[] {
  return stages.map(({ id, title }) => {
    const selected = state.selections.find(item => item.stage === id);
    const name = selected && state.catalog.find(item => item.preset.id === selected.presetId && item.preset.version === selected.version)?.preset.name;
    return { stage: id, title, label: selected ? name ?? '저장된 선택(이름 확인 불가)' : '선택 안 함', version: selected ? `${selected.presetId}/${selected.version}` : null };
  });
}
/** The one next action shown first, derived from saved facts and the current preview. */
export function nextAction({ state, plan, preview, aiRequest, projectId }: { state: TechPresetState; plan: TechPlanSnapshot | null; preview: TechPreview | null; aiRequest: TechAiDraftRequest | null; projectId: string }): string {
  if (canApproveAiRequest(aiRequest, projectId)) return 'AI에게 보낼 내용과 목적지를 확인하고 전송을 승인하거나 거절하세요.';
  const shown = forProject(preview, projectId);
  if (shown && !canApplyPreview(shown, projectId)) return '후보에 충돌이나 질문이 있습니다. 해결하기 전에는 저장할 수 없습니다.';
  if (shown) return '차이와 영향을 확인한 뒤 직접 저장하세요. 저장 전에는 아무것도 바뀌지 않습니다.';
  const fixed = forProject(plan, projectId);
  if (fixed?.applicable && selectionChanged(state, fixed)) return '현재 설정이 진행 중인 계획과 다릅니다. 반영하려면 목표에서 계획을 다시 세워 승인하세요.';
  return '바꿀 것이 없으면 그대로 두세요. 바꾸려면 단계를 고르고 차이를 확인하세요.';
}

export function targetLabel(target: TechNumericTarget): string {
  const status = target.status === 'achieved' ? '달성' : target.status === 'failed' ? '실패' : '미검증';
  return `${target.key} 목표 ${target.target}${target.unit} · ${status} · ${target.reason}`;
}
export function manualDraftTemplate(preset: TechPreset, requirement: string, now: number): TechPreset {
  return { ...preset, id: `U-${preset.stage.toUpperCase()}-${now}`, name: '사용자 프리셋', purpose: requirement.trim() || preset.purpose };
}
/** Dependency and conflict reasons of one definition, the "제약" a user checks before previewing. */
export function presetConstraints(preset: Partial<TechPreset>): string[] {
  return (preset.dependencies ?? []).map(rule => `필요: ${rule.requires.join(', ')} · ${rule.reason}`).concat((preset.conflicts ?? []).map(rule => `충돌: ${rule.with} · ${rule.reason}`));
}
export function PlanPresetCard({ project, goal, approvals, onSnapshot, onOpenSettings }: { project: Project | null; goal: Goal; approvals: Approval[]; onSnapshot?: (approvalId: string, fingerprint: string | null) => void; onOpenSettings?: () => void }) {
  const [plan, setPlan] = useState<TechPlanSnapshot | null>(null);
  const [selection, setSelection] = useState<TechPresetState | null>(null);
  const [error, setError] = useState('');
  const approval = [...approvals].reverse().find(item => item.goalId === goal.id && item.operation === 'project_write');
  useEffect(() => {
    setPlan(null); setSelection(null); setError('');
    if (!approval || !project || goal.projectId !== project.id) return;
    let live = true;
    Promise.all([
      invoke<TechPlanSnapshot | null>('get_plan_preset_snapshot', { approvalId: approval.id }),
      invoke<TechPresetState>('get_tech_presets', { projectId: project.id }),
    ]).then(([nextPlan, nextSelection]) => { if (live) { setPlan(nextPlan); setSelection(nextSelection); } }).catch(reason => { if (live) setError(String(reason)); });
    return () => { live = false; };
  }, [approval?.id, project?.id, goal.id, goal.projectId]);
  useEffect(() => {
    if (!approval) return;
    const fingerprint = planFingerprintForApproval(plan, approval.id, project?.id ?? '', goal.id);
    onSnapshot?.(approval.id, fingerprint);
    return () => onSnapshot?.(approval.id, null);
  }, [approval?.id, plan, project?.id, goal.id, onSnapshot]);
  if (!approval || !project || goal.projectId !== project.id) return null;
  return <aside className="v4-plan-card" aria-label="계획의 기술·구조 결정">
    <strong>이 계획에 고정된 기술·구조 결정</strong>
    {error && <p role="alert">계획의 프리셋 기록을 읽지 못했습니다: {error}</p>}
    {!error && !plan && <p>계획 기록을 확인하는 중입니다.</p>}
    {plan && !plan.applicable && <p>앱 산출물이 아닌 목표입니다. 프리셋을 적용하지 않습니다.</p>}
    {plan?.applicable && <>
      <p>계획 {plan.planRevision}차 · {approval.status === 'approved' ? '승인됨' : approval.status === 'pending' ? '승인 대기' : approval.status}</p>
      <p>{plan.choices.length ? plan.choices.map(item => `${item.preset.name} ${item.preset.version}`).join(' + ') : '고정된 선택 없음'}</p>
      <p>성능 목표: {plan.targets?.length ? `${plan.targets.length}개` : plan.choices.some(item => item.stage === 'advanced') ? '미검증 · 저장된 측정 근거 없음' : '선택 없음'}</p>
      {plan.targets?.map(target => <p key={`${target.presetId}:${target.key}`}>{targetLabel(target)}</p>)}
      {selection && selectionChanged(selection, plan) && <p className="v4-warning">재계획 필요 · 현재 설정이 이 계획의 고정 버전과 다릅니다. 기존 승인 범위는 그대로입니다.</p>}
      <details><summary>고정 버전과 검증 기준 보기</summary><ul>{plan.choices.map(item => <li key={item.stage}>{item.preset.id}/{item.preset.version}</li>)}</ul><ul>{plan.qaChecklist.map((line, index) => <li key={index}>{line}</li>)}</ul></details>
    </>}
    {onOpenSettings && <button type="button" className="secondary" onClick={onOpenSettings}>설정 &gt; 앱 기술·구조 열기</button>}
    {onOpenSettings && <small>설정을 열어도 이 계획의 고정값과 승인 대기는 바뀌지 않습니다.</small>}
  </aside>;
}

type SettingsProps = { project: Project | null; goals: Goal[]; approvals: Approval[]; agents: Agent[]; busy: boolean; refresh: () => Promise<void>; onSelectProject?: () => void };

/** `설정 > 앱 기술·구조`. The project panel is keyed by project ID so drafts, previews and AI consent never carry over. */
export function PresetSettings({ project, onSelectProject, ...rest }: SettingsProps) {
  if (!project) return <NoProjectPresets onSelectProject={onSelectProject} />;
  return <ProjectPresets key={project.id} project={project} {...rest} />;
}

export function NoProjectPresets({ onSelectProject }: { onSelectProject?: () => void }) {
  return <section className="v4-presets" aria-label="애플리케이션 기술·구조 결정">
    <div className="v4-summary">
      <h2>앱 기술·구조</h2>
      <p className="v4-next" role="status"><strong>프로젝트를 먼저 선택하세요.</strong> 선택은 프로젝트마다 따로 저장됩니다. 지금은 저장·AI 전송·계획 반영을 할 수 없습니다.</p>
      {onSelectProject && <button type="button" className="primary" onClick={onSelectProject}>프로젝트 선택</button>}
    </div>
    <details className="v4-section"><summary>기본 프리셋 설명 보기</summary>
      <p>단계는 품질 등급이 아니라 직접 정할 깊이입니다. 모든 단계는 선택하지 않아도 됩니다.</p>
      <ul>{stages.map(item => <li key={item.id}><strong>{item.title}</strong> · {item.question} {builtinOutline[item.id].join(', ')}</li>)}</ul>
    </details>
  </section>;
}

/** Everything the screen holds locally for one project. Engine results for another project never enter it. */
export type PresetScreen = { projectId: string; saved: TechPresetState | null; plan: TechPlanSnapshot | null; stage: TechStage; pick: TechPick; preview: TechPreview | null; aiRequest: TechAiDraftRequest | null; aiMessage: string; requirement: string; draftText: string; draft: TechDraft | null; error: string; loading: boolean };
export type PresetEvent =
  | { type: 'loaded'; state: TechPresetState }
  | { type: 'plan'; plan: TechPlanSnapshot | null }
  | { type: 'stage'; stage: TechStage }
  | { type: 'pick'; pick: TechPick }
  | { type: 'loading' }
  | { type: 'failed'; error: string; dropPreview?: boolean; dropRequest?: boolean }
  | { type: 'previewed'; preview: TechPreview; pick: TechPick }
  | { type: 'saved'; state: TechPresetState }
  | { type: 'requirement'; text: string }
  | { type: 'draftText'; text: string }
  | { type: 'aiPreviewed'; request: TechAiDraftRequest }
  | { type: 'aiDecided'; request: TechAiDraftRequest | null; message: string }
  | { type: 'aiFinished'; message: string }
  | { type: 'draftSaved'; draft: TechDraft; state: TechPresetState; stage: TechStage };
export const initialScreen = (projectId: string): PresetScreen => ({ projectId, saved: null, plan: null, stage: 'beginner', pick: emptyPick('beginner'), preview: null, aiRequest: null, aiMessage: '', requirement: '', draftText: '', draft: null, error: '', loading: false });
const samePick = (a: TechPick, b: TechPick) => JSON.stringify(a) === JSON.stringify(b);

/** Only `saved` reflects the engine's active selection; it changes on load, poll, or an explicit save. */
export function presetScreen(screen: PresetScreen, event: PresetEvent): PresetScreen {
  const own = <T extends { projectId: string }>(value: T | null) => forProject(value, screen.projectId);
  switch (event.type) {
    case 'loaded': return own(event.state) ? { ...screen, saved: event.state } : screen;
    case 'plan': return { ...screen, plan: own(event.plan) };
    case 'stage': return { ...screen, stage: event.stage, pick: emptyPick(event.stage), preview: null, error: '' };
    case 'pick': return { ...screen, pick: event.pick, preview: null, error: '' };
    case 'loading': return { ...screen, loading: true, error: '' };
    case 'failed': return { ...screen, loading: false, error: event.error, preview: event.dropPreview ? null : screen.preview, aiRequest: event.dropRequest ? null : screen.aiRequest };
    // A preview answers one pick; a late answer for an earlier pick or project is not shown.
    case 'previewed': return { ...screen, loading: false, preview: own(event.preview) && samePick(event.pick, screen.pick) ? event.preview : null };
    case 'saved': return own(event.state) ? { ...screen, loading: false, saved: event.state, preview: null } : { ...screen, loading: false };
    case 'requirement': return { ...screen, requirement: event.text, aiRequest: screen.aiRequest?.status === 'sending' ? screen.aiRequest : null };
    case 'draftText': return { ...screen, draftText: event.text };
    case 'aiPreviewed': return { ...screen, loading: false, aiMessage: '', aiRequest: own(event.request) };
    case 'aiDecided': return { ...screen, loading: false, aiRequest: own(event.request), aiMessage: event.message };
    case 'aiFinished': return { ...screen, aiRequest: null, aiMessage: event.message };
    // A kept draft becomes the current candidate only; the active selection still needs preview and save.
    case 'draftSaved': return own(event.draft) && own(event.state) && event.stage === screen.stage ? { ...screen, loading: false, saved: event.state, draft: event.draft, pick: { stage: screen.stage, draftId: event.draft.id, values: {} }, preview: null } : { ...screen, loading: false };
  }
}

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
/** Engine commands of the screen. Only `save` calls `select_tech_preset`, and only with the shown preview fingerprint. */
export function presetActions({ projectId, call, dispatch, read, refresh, agents }: { projectId: string; call: Invoke; dispatch: Dispatch<PresetEvent>; read: () => PresetScreen; refresh: () => Promise<void>; agents: Agent[] }) {
  const reload = () => call<TechPresetState>('get_tech_presets', { projectId });
  return {
    async preview() {
      const { pick } = read();
      dispatch({ type: 'loading' });
      try { dispatch({ type: 'previewed', preview: await call<TechPreview>('preview_tech_preset', { projectId, pick }), pick }); }
      catch (reason) { dispatch({ type: 'failed', error: String(reason), dropPreview: true }); }
    },
    async save() {
      const { pick, preview } = read();
      if (!preview || !canApplyPreview(preview, projectId)) return;
      dispatch({ type: 'loading' });
      try {
        await call('select_tech_preset', { projectId, pick, previewFingerprint: preview.fingerprint });
        dispatch({ type: 'saved', state: await reload() });
        await refresh();
      } catch (reason) { dispatch({ type: 'failed', error: String(reason) }); }
    },
    async aiPreview() {
      const { stage, requirement } = read();
      const agent = agents.find(item => item.id === 'cto' && item.enabled) ?? agents.find(item => item.enabled);
      if (!agent || requirement.trim().length < 4) return;
      dispatch({ type: 'loading' });
      try { dispatch({ type: 'aiPreviewed', request: await call<TechAiDraftRequest>('preview_tech_preset_ai_draft', { projectId, agentId: agent.id, requirement: `${stage} 단계 프리셋. ${requirement.trim()}` }) }); }
      catch (reason) { dispatch({ type: 'failed', error: String(reason), dropRequest: true }); }
    },
    async aiDecide(approved: boolean) {
      const { aiRequest } = read();
      if (!aiRequest || !canApproveAiRequest(aiRequest, projectId)) return;
      dispatch({ type: 'loading' });
      try {
        const request = await call<TechAiDraftRequest>(approved ? 'approve_tech_preset_ai_draft' : 'refuse_tech_preset_ai_draft', approved ? { projectId, requestId: aiRequest.id, payloadSha256: aiRequest.payloadSha256, destinationSha256: aiRequest.destinationSha256 } : { projectId, requestId: aiRequest.id });
        dispatch({ type: 'aiDecided', request: approved ? request : null, message: approved ? '' : '전송하지 않았습니다.' });
      } catch (reason) { dispatch({ type: 'failed', error: String(reason), dropRequest: true }); }
    },
    async saveDraft() {
      const { draftText, stage } = read();
      dispatch({ type: 'loading' });
      try {
        const definition = JSON.parse(draftText) as Partial<TechPreset>;
        const draft = await call<TechDraft>('save_tech_preset_draft', { projectId, draftId: null, origin: 'user', definition });
        dispatch({ type: 'draftSaved', draft, state: await reload(), stage });
      } catch (reason) { dispatch({ type: 'failed', error: String(reason) }); }
    },
  };
}
export type PresetHandlers = { stage: (stage: TechStage) => void; pick: (pick: TechPick) => void; preview: () => void; save: () => void; requirement: (text: string) => void; draftText: (text: string) => void; template: () => void; aiPreview: () => void; aiDecide: (approved: boolean) => void; saveDraft: () => void };
export function presetHandlers(dispatch: Dispatch<PresetEvent>, actions: ReturnType<typeof presetActions>, read: () => PresetScreen): PresetHandlers {
  return {
    stage: stage => dispatch({ type: 'stage', stage }),
    pick: pick => dispatch({ type: 'pick', pick }),
    preview: () => void actions.preview(),
    save: () => void actions.save(),
    requirement: text => dispatch({ type: 'requirement', text }),
    draftText: text => dispatch({ type: 'draftText', text }),
    template: () => {
      const { saved, stage, requirement } = read();
      const catalog = saved?.catalog.filter(item => item.preset.stage === stage) ?? [];
      const suggested = catalog.find(item => item.origin === 'builtin') ?? catalog[0];
      if (suggested) dispatch({ type: 'draftText', text: JSON.stringify(manualDraftTemplate(suggested.preset, requirement, Date.now()), null, 2) });
    },
    aiPreview: () => void actions.aiPreview(),
    aiDecide: approved => void actions.aiDecide(approved),
    saveDraft: () => void actions.saveDraft(),
  };
}

function ProjectPresets({ project, goals, approvals, agents, busy, refresh }: Omit<SettingsProps, 'project' | 'onSelectProject'> & { project: Project }) {
  const [screen, dispatch] = useReducer(presetScreen, project.id, initialScreen);
  const latest = useRef(screen);
  latest.current = screen;
  const read = () => latest.current;
  const actions = presetActions({ projectId: project.id, call: invoke, dispatch, read, refresh, agents });
  const currentGoal = [...goals].reverse().find(item => item.projectId === project.id && item.kind !== 'deliberation');
  const approval = [...approvals].reverse().find(item => item.goalId === currentGoal?.id && item.operation === 'project_write') ?? null;
  const aiRequest = forProject(screen.aiRequest, project.id);
  useEffect(() => {
    let live = true;
    invoke<TechPresetState>('get_tech_presets', { projectId: project.id }).then(state => { if (live) dispatch({ type: 'loaded', state }); }).catch(reason => { if (live) dispatch({ type: 'failed', error: String(reason) }); });
    return () => { live = false; };
  }, [project.id]);
  useEffect(() => {
    if (!approval) { dispatch({ type: 'plan', plan: null }); return; }
    let live = true;
    invoke<TechPlanSnapshot | null>('get_plan_preset_snapshot', { approvalId: approval.id }).then(plan => { if (live) dispatch({ type: 'plan', plan }); }).catch(() => { if (live) dispatch({ type: 'plan', plan: null }); });
    return () => { live = false; };
  }, [approval?.id]);
  useEffect(() => {
    if (aiRequest?.status !== 'sending') return;
    const requestId = aiRequest.id;
    let live = true;
    const timer = window.setInterval(() => {
      void Promise.all([invoke<TechPresetState>('get_tech_presets', { projectId: project.id }), invoke<{ events: { message: string }[] }>('get_snapshot')]).then(([state, snapshot]) => {
        if (!live) return;
        dispatch({ type: 'loaded', state });
        const event = [...snapshot.events].reverse().find(item => item.message.includes(`request ${requestId}:`) && /completed|failed|interrupted/.test(item.message));
        if (!event) return;
        window.clearInterval(timer);
        dispatch({ type: 'aiFinished', message: event.message });
      }).catch(reason => { if (live) { window.clearInterval(timer); dispatch({ type: 'failed', error: String(reason), dropRequest: true }); } });
    }, 1000);
    return () => { live = false; window.clearInterval(timer); };
  }, [aiRequest?.id, aiRequest?.status, project.id]);
  return <PresetPanel project={project} screen={screen} approval={approval} busy={busy} agentsReady={agents.some(item => item.enabled)} on={presetHandlers(dispatch, actions, read)} />;
}

const approvalLabel = (approval: Approval | null) => approval?.status === 'approved' ? '승인됨' : approval?.status === 'pending' ? '승인 대기' : approval?.status ?? '승인 기록 없음';
const issueLines = (issues: TechPreview['report']['issues']) => issues.map((issue, index) => <li key={index}><strong>{issue.field}</strong> · {issue.reason} · 어떤 값으로 정할까요?</li>);

/** Hook-free view: current project and fixed values, then stage choice and checks, then full definitions. */
export function PresetPanel({ project, screen, approval, busy, agentsReady, on }: { project: Project; screen: PresetScreen; approval: Approval | null; busy: boolean; agentsReady: boolean; on: PresetHandlers }) {
  const state = forProject(screen.saved, project.id);
  const preview = forProject(screen.preview, project.id);
  const plan = forProject(screen.plan, project.id);
  const aiRequest = forProject(screen.aiRequest, project.id);
  const { stage, pick, loading, error } = screen;
  const stageTitle = stages.find(item => item.id === stage);
  const catalog = state?.catalog.filter(item => item.preset.stage === stage) ?? [];
  const builtins = catalog.filter(item => item.origin === 'builtin');
  const stageDrafts = state?.drafts.filter(item => item.definition.stage === stage && item.projectId === project.id) ?? [];
  const summary = state ? selectionSummary(state) : [];
  const fixedFor = (id: TechStage) => plan?.applicable ? plan.choices.find(item => item.stage === id) : undefined;
  const picked = pick.draftId ? stageDrafts.find(item => item.id === pick.draftId) ?? forProject(screen.draft?.id === pick.draftId ? screen.draft : null, project.id) : null;
  const pickedPreset = pick.presetId ? catalog.find(item => item.preset.id === pick.presetId && item.preset.version === pick.version)?.preset : picked?.definition;
  const changed = !!state && !!plan?.applicable && selectionChanged(state, plan);
  const onTabKey = (event: KeyboardEvent<HTMLDivElement>) => {
    const next = tabKeyTarget(stages.findIndex(item => item.id === stage), event.key, stages.length);
    if (next === null) return;
    event.preventDefault();
    on.stage(stages[next].id);
    event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]')[next]?.focus();
  };
  return <section className="v4-presets" aria-label="애플리케이션 기술·구조 결정">
    <header className="v4-summary" aria-label="현재 상태">
      <p className="v4-eyebrow">현재 프로젝트</p>
      <h2>{project.name}</h2>
      <p className="v4-control">설정 &gt; 앱 기술·구조 · 선택은 이 프로젝트에만 저장되고, 계획을 세울 때 그 계획 차수에 고정됩니다.</p>
      {error && <p role="alert" className="v4-error">{error}</p>}
      {!state && !error && <p role="status">이 프로젝트의 선택을 불러오는 중입니다.</p>}
      {state && <>
        <div className="v4-status-grid">
          <div aria-label="활성 선택">
            <h3>활성 선택</h3>
            <dl className="v4-current">{summary.map(item => <div key={item.stage}><dt>{item.title}</dt><dd>{item.label}{item.version && <small>{item.version}</small>}</dd></div>)}</dl>
          </div>
          <div aria-label="계획에 고정된 버전" className="v4-fixed">
            <h3>계획에 고정된 버전</h3>
            {!plan && <p>진행 중인 계획 없음 · 다음 계획을 세울 때 그때의 활성 선택이 고정됩니다.</p>}
            {plan && !plan.applicable && <p>앱 산출물이 아닌 목표 · 고정하지 않음</p>}
            {plan?.applicable && <>
              <p>계획 {plan.planRevision}차 · {approvalLabel(approval)}</p>
              <ul>{stages.map(item => { const fixed = fixedFor(item.id); return <li key={item.id}>{item.title}: {fixed ? `${fixed.preset.name} · ${fixed.preset.id}/${fixed.preset.version}` : '고정 없음'}</li>; })}</ul>
              {changed && <p className="v4-warning">재계획 필요 · 활성 선택이 이 계획의 고정 버전과 다릅니다. 기존 승인 범위는 그대로입니다.</p>}
            </>}
          </div>
        </div>
        <p className="v4-next" role="status"><strong>다음 행동</strong> {nextAction({ state, plan, preview, aiRequest, projectId: project.id })}</p>
      </>}
    </header>
    {state && <>
      {aiRequest?.status === 'previewed' && <section className="v4-section v4-consent" aria-label="AI 초안 전송 승인"><h3>AI 초안 전송 승인</h3><p>아래 내용과 목적지로 AI에게 한 번 보냅니다. 받은 초안은 차이를 확인하고 저장하기 전까지 활성 선택과 계획 고정값을 바꾸지 않습니다.</p><p>팀원: {aiRequest.destination.agentName} · 연결: {aiRequest.destination.connectionName} · 종류: {aiRequest.destination.kind} · 모델: {aiRequest.destination.modelId} · 실행 파일 또는 주소: {aiRequest.destination.target}</p><details open><summary>보낼 내용 전문</summary><pre>{aiRequest.payload}</pre></details><div className="v4-actions"><button type="button" className="primary" disabled={loading} onClick={() => on.aiDecide(true)}>이 내용과 목적지로 전송 승인</button><button type="button" className="secondary" disabled={loading} onClick={() => on.aiDecide(false)}>전송 거절</button></div></section>}
      {aiRequest?.status === 'sending' && <p role="status">승인한 내용으로 한 번 전송했습니다. AI 초안 결과를 확인하는 중입니다.</p>}
      {screen.aiMessage && <p role="status">{screen.aiMessage}</p>}
      <section className="v4-section" aria-label="단계별 선택과 검사">
        <h3 className="v4-step">단계별 선택과 검사 <small>현재 조합: {statusText[state.report.status]}</small></h3>
        {!!state.report.issues.length && <ul className="v4-issues">{issueLines(state.report.issues)}</ul>}
        <div className="v4-stages" role="tablist" aria-label="결정 단계" onKeyDown={onTabKey}>{stages.map(item => <button key={item.id} type="button" role="tab" id={`v4-stage-${item.id}`} aria-controls="v4-stage-panel" aria-selected={stage === item.id} tabIndex={stage === item.id ? 0 : -1} onClick={() => on.stage(item.id)}>{item.title}<small>{summary.find(value => value.stage === item.id)?.label}</small></button>)}</div>
        <div id="v4-stage-panel" role="tabpanel" aria-labelledby={`v4-stage-${stage}`}>
          <h4>{stageTitle?.title} · {stageTitle?.question}</h4>
          <p>활성: {summary.find(item => item.stage === stage)?.label} · 계획 고정: {fixedFor(stage) ? `${fixedFor(stage)?.preset.id}/${fixedFor(stage)?.preset.version}` : '없음'} · 고르지 않아도 됩니다.</p>
          <div className="v4-options" aria-label="후보">
            <button type="button" aria-pressed={!pick.presetId && !pick.draftId} onClick={() => on.pick(emptyPick(stage))}><strong>선택 안 함</strong><small>계획에서 팀이 정하고 CEO가 검토합니다.</small></button>
            {builtins.map((item, index) => <button type="button" className={index === 0 ? 'v4-recommended' : undefined} key={`${item.preset.id}/${item.preset.version}`} aria-pressed={pick.presetId === item.preset.id && pick.version === item.preset.version} onClick={() => on.pick({ stage, presetId: item.preset.id, version: item.preset.version, values: {} })}><strong>{item.preset.name}{index === 0 ? ' · 권장 시작점' : ''}</strong><small>{item.preset.purpose}</small><small>제약 {presetConstraints(item.preset).length}개</small></button>)}
          </div>
          {!!stageDrafts.length && <div className="v4-drafts" aria-label="사용자 초안"><h4>사용자 초안 · 선택 전</h4>{stageDrafts.map(item => <button type="button" key={item.id} aria-pressed={pick.draftId === item.id} onClick={() => on.pick({ stage, draftId: item.id, values: {} })}>{item.definition.name ?? '이름 없음'} · {item.origin === 'ai' ? 'AI 제안' : '사용자 초안'} · {statusText[item.report.status]}</button>)}</div>}
          {(pick.presetId || pick.draftId) && <div className="v4-candidate" aria-label="고른 후보">
            <h4>고른 후보: {pickedPreset?.name ?? '이름 없음'}{pickedPreset?.id ? ` · ${pickedPreset.id}/${pickedPreset.version ?? '?'}` : ''}</h4>
            <p>결정: {pickedPreset?.decisions?.map(decision => `${decision.key}=${decision.value}`).join(', ') || '없음'}</p>
            <p>제약·호환 조건:</p><ul>{presetConstraints(pickedPreset ?? {}).map((line, index) => <li key={index}>{line}</li>)}{!presetConstraints(pickedPreset ?? {}).length && <li>추가 조건 없음</li>}</ul>
            {picked && <><strong>초안 검사: {statusText[picked.report.status]}</strong><ul>{issueLines(picked.report.issues)}</ul></>}
          </div>}
          <div className="v4-preview">
            <p>고르기만 해서는 저장되지 않습니다. 먼저 차이와 영향을 확인하세요.</p>
            <button type="button" className="secondary" disabled={loading || busy} onClick={on.preview}>선택 전 차이·영향 보기</button>
            {preview && <div aria-label="선택 미리보기" aria-live="polite">
              <h4>현재 선택 → 후보</h4>
              <ul>{preview.changes.map(change => <li key={change.stage}>{stages.find(item => item.id === change.stage)?.title}: {change.from ?? '선택 안 함'} → {change.to ?? '선택 안 함'}</li>)}{!preview.changes.length && <li>바뀌는 단계 없음</li>}</ul>
              <p className={canApplyPreview(preview, project.id) ? undefined : 'v4-warning'}>조합 검사: {statusText[preview.report.status]}</p>
              <ul>{issueLines(preview.report.issues)}</ul>
              <h4>예상 계획·산출물 영향</h4><p>{preview.planEffect}</p><ul>{preview.impact.map((item, index) => <li key={index}>{item}</li>)}</ul>
              <p>성능 목표: {stage === 'advanced' ? '미검증 · 측정 전 달성으로 표시하지 않음' : '해당 없음'}</p>
              <p>저장해도 이미 만든 계획은 바뀌지 않습니다. 다음 계획부터 반영됩니다.</p>
              <div className="v4-actions v4-save"><button type="button" className="primary" disabled={loading || busy || !canApplyPreview(preview, project.id)} onClick={on.save}>이 프로젝트의 선택으로 저장</button></div>
            </div>}
          </div>
        </div>
      </section>
      <details className="v4-section" aria-label="규격 전체"><summary>규격 전체 보기 · {stageTitle?.title}</summary>
        {catalog.map(item => <div key={`${item.preset.id}/${item.preset.version}`} className="v4-spec">
          <h4>{item.preset.name}{summary.find(value => value.stage === stage)?.version === `${item.preset.id}/${item.preset.version}` ? ' · 활성' : ''}{fixedFor(stage)?.preset.id === item.preset.id && fixedFor(stage)?.preset.version === item.preset.version ? ' · 계획 고정' : ''}</h4>
          <dl>
            <dt>ID·버전·단계</dt><dd>{item.preset.id} · {item.preset.version} · {stages.find(value => value.id === item.preset.stage)?.title}</dd>
            <dt>목적</dt><dd>{item.preset.purpose}</dd>
            <dt>결정 항목</dt><dd>{item.preset.decisions.map(decision => `${decision.key}=${decision.value} (${decision.reason})`).join(' · ') || '없음'}</dd>
            <dt>기본값</dt><dd>{Object.entries(item.preset.defaults).map(([key, value]) => `${key}=${value}`).join(', ') || '없음'}</dd>
            <dt>허용 범위</dt><dd>{item.preset.allowedRange.map(range => `${range.key}: ${range.values.join('/')}`).join(' · ') || '없음'}</dd>
            <dt>의존성</dt><dd>{item.preset.dependencies.map(rule => `${rule.requires.join(', ')} · ${rule.reason}`).join(' · ') || '없음'}</dd>
            <dt>충돌 규칙</dt><dd>{item.preset.conflicts.map(rule => `${rule.with} · ${rule.reason}`).join(' · ') || '없음'}</dd>
            <dt>산출물 반영</dt><dd>{item.preset.artifactRules.map(rule => `${rule.target}: ${rule.text}${rule.autoWrite ? '' : ' (자동 쓰기 없음)'}`).join(' · ') || '없음'}</dd>
            <dt>검증 기준</dt><dd>{item.preset.validation.join(' · ') || '없음'}</dd>
          </dl>
        </div>)}
        {!catalog.length && <p>이 단계의 프리셋이 없습니다.</p>}
        {plan?.applicable && !!plan.qaChecklist.length && <><h4>계획 {plan.planRevision}차 검증 기준</h4><ul>{plan.qaChecklist.map((line, index) => <li key={index}>{line}</li>)}</ul></>}
      </details>
      <details className="v4-section"><summary>직접 프리셋 만들기 (AI 또는 양식)</summary>
        <div className="v4-conversation">
          <label>요구와 제약<textarea value={screen.requirement} maxLength={3900} onChange={event => on.requirement(event.target.value)} placeholder="만들 앱, 대상, 예상 부하, 외부 서비스, 측정 환경을 적어 주세요." /></label>
          <div className="v4-actions"><button type="button" className="secondary" disabled={!agentsReady || screen.requirement.trim().length < 4 || loading || aiRequest?.status === 'sending'} onClick={on.aiPreview}>보낼 내용과 목적지 확인</button>
          <button type="button" className="secondary" disabled={!catalog.length} onClick={on.template}>직접 초안 양식 만들기</button></div>
          <p>확인만으로는 보내지 않습니다. AI가 작성한 규격을 검토하고 고칠 수 있으며, 보관해도 활성 선택은 바뀌지 않습니다.</p>
          <label>프리셋 규격 초안<textarea className="v4-code" value={screen.draftText} onChange={event => on.draftText(event.target.value)} placeholder="공통 규격 JSON" /></label>
          <div className="v4-actions"><button type="button" className="secondary" disabled={!screen.draftText.trim() || loading || busy} onClick={on.saveDraft}>초안 검사·보관</button></div>
        </div>
      </details>
    </>}
  </section>;
}
