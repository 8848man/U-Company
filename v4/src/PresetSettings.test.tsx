import { describe, expect, it } from 'vitest';
import { isValidElement, type ReactElement, type ReactNode } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { PlanPresetCard, PresetPanel, PresetSettings, canApplyPreview, canApproveAiRequest, forProject, initialScreen, manualDraftTemplate, nextAction, planFingerprintForApproval, presetActions, presetHandlers, presetScreen, selectionChanged, selectionSummary, targetLabel, type PresetEvent, type PresetScreen } from './PresetSettings';
import { V2SettingsTabs, tabKeyTarget } from '../../src/shell/V2Navigation';
import type { Agent, Approval, Goal, Project, TechAiDraftRequest, TechPlanSnapshot, TechPreset, TechPresetState, TechPreview } from '../../src/types';

const preset: TechPreset = { id: 'B-WEB', version: '1.0.0', stage: 'beginner', name: '웹 앱', purpose: '웹 화면', decisions: [], defaults: {}, allowedRange: [], dependencies: [], conflicts: [], artifactRules: [], validation: [] };
const state: TechPresetState = { projectId: 'one', catalog: [], drafts: [], selections: [{ projectId: 'one', stage: 'beginner', presetId: 'B-WEB', version: '1.0.0', digest: 'a', values: {}, selectedAt: '', actor: 'ceo' }], report: { status: 'pass', issues: [] } };
const plan: TechPlanSnapshot = { projectId: 'one', goalId: 'g', planRevision: 1, approvalId: 'approval-1', applicable: true, choices: [{ stage: 'beginner', preset, digest: 'a', values: {} }], resolved: {}, validation: { status: 'pass', issues: [] }, qaChecklist: [], impact: [], selectionFingerprint: 'selection', tasks: [], capturedAt: '', fingerprint: 'shown-fingerprint', targets: [] };
const preview: TechPreview = { projectId: 'one', changes: [], report: { status: 'pass', issues: [] }, impact: [], qaChecklist: [], planEffect: '', fingerprint: 'preview-fingerprint' };
const request: TechAiDraftRequest = { id: 'request-1', projectId: 'one', requirement: '웹 앱 구조', payload: 'exact payload', payloadSha256: 'payload-hash', destination: { agentId: 'cto', agentName: 'CTO', connectionId: 'connection', connectionName: 'Provider', kind: 'cli', modelId: 'model', target: 'provider.exe' }, destinationSha256: 'destination-hash', status: 'previewed', draftId: null, note: '', createdAt: '', decidedAt: null };
const goal: Goal = { id: 'g', projectId: 'one', intent: '', status: '', report: '', createdAt: '', resumedFrom: null, reviewLoopsUsed: 0, reviewLoopLimit: 2, workGroup: null };

describe('v4 preset screen flow', () => {
  it('creates a complete user ID template that can become a selection candidate', () => {
    const draft = manualDraftTemplate({ ...preset, decisions: [{ key: 'framework', value: 'React', reason: '웹 화면' }], defaults: { framework: 'React' }, allowedRange: [{ key: 'framework', kind: 'enum', values: ['React'], conditions: [] }], artifactRules: [{ target: 'plan', text: 'React를 사용', autoWrite: false }], validation: ['빌드'] }, '주문 화면', 123);
    expect(draft.id).toBe('U-BEGINNER-123');
    expect(draft.purpose).toBe('주문 화면');
    expect(draft.decisions[0].value).toBe(draft.defaults.framework);
    expect(draft.artifactRules[0].autoWrite).toBe(false);
  });
  it('keeps a draft inactive until a valid preview for the current project', () => {
    expect(canApplyPreview(null, 'one')).toBe(false);
    expect(canApplyPreview(preview, 'two')).toBe(false);
    expect(canApplyPreview({ ...preview, report: { status: 'question', issues: [] } }, 'one')).toBe(false);
    expect(canApplyPreview({ ...preview, report: { status: 'conflict', issues: [] } }, 'one')).toBe(false);
    expect(canApplyPreview(preview, 'one')).toBe(true);
  });
  it('cannot approve an AI send after project switch, refusal, or stale status', () => {
    expect(canApproveAiRequest(request, 'one')).toBe(true);
    expect(canApproveAiRequest(request, 'two')).toBe(false);
    expect(canApproveAiRequest({ ...request, status: 'refused' }, 'one')).toBe(false);
    expect(canApproveAiRequest({ ...request, status: 'stale' }, 'one')).toBe(false);
  });
  it('uses only the displayed plan fingerprint for this approval, goal and project', () => {
    expect(planFingerprintForApproval(plan, 'approval-1', 'one', 'g')).toBe('shown-fingerprint');
    expect(planFingerprintForApproval(plan, 'approval-2', 'one', 'g')).toBeNull();
    expect(planFingerprintForApproval(plan, 'approval-1', 'two', 'g')).toBeNull();
    expect(planFingerprintForApproval(plan, 'approval-1', 'one', 'other')).toBeNull();
    expect(selectionChanged(state, plan)).toBe(false);
    expect(selectionChanged({ ...state, selections: [{ ...state.selections[0], version: '2.0.0' }] }, plan)).toBe(true);
    expect(selectionChanged(state, { ...plan, applicable: false })).toBe(false);
  });
  it('shows saved numeric verification and evidence without claiming an unmeasured target is achieved', () => {
    const target = { presetId: 'A-LATENCY', key: 'p95_ms', target: '200', unit: 'ms', direction: 'max', conditions: { load: '100 users' }, status: 'unverified' as const, reason: '아직 측정하지 않았습니다' };
    expect(targetLabel(target)).toContain('미검증');
    expect(targetLabel(target)).not.toContain('달성');
    expect(targetLabel({ ...target, status: 'achieved', reason: 'qa/latency.json' })).toContain('달성 · qa/latency.json');
  });
  it('does not show another project plan card', () => {
    const html = renderToStaticMarkup(<PlanPresetCard project={{ id: 'two', name: 'Other', path: '', kind: 'folder', reviewLoopLimit: 2 }} goal={goal} approvals={[]} />);
    expect(html).toBe('');
  });
});

describe('설정 > 앱 기술·구조 tab', () => {
  it('lists the dedicated tab right after 프로젝트 only when v4 supplies it, with one focusable tab', () => {
    const v4 = renderToStaticMarkup(<V2SettingsTabs section="tech" onSection={() => {}} extra={[{ id: 'tech', label: '앱 기술·구조' }]} />);
    expect(v4.indexOf('프로젝트')).toBeLessThan(v4.indexOf('앱 기술·구조'));
    expect(v4.indexOf('앱 기술·구조')).toBeLessThan(v4.indexOf('AI 연결'));
    expect(v4).toMatch(/id="settings-tab-tech" aria-selected="true" tabindex="0"/);
    expect(v4).toMatch(/id="settings-tab-projects" aria-selected="false" tabindex="-1"/);
    expect(renderToStaticMarkup(<V2SettingsTabs section="projects" onSection={() => {}} />)).not.toContain('앱 기술·구조');
  });
  it('moves between tabs with arrow, Home and End keys and ignores other keys', () => {
    expect(tabKeyTarget(0, 'ArrowRight', 3)).toBe(1);
    expect(tabKeyTarget(0, 'ArrowLeft', 3)).toBe(2);
    expect(tabKeyTarget(2, 'ArrowDown', 3)).toBe(0);
    expect(tabKeyTarget(1, 'Home', 3)).toBe(0);
    expect(tabKeyTarget(0, 'End', 3)).toBe(2);
    expect(tabKeyTarget(0, 'Enter', 3)).toBeNull();
  });
  it('with no project routes to 프로젝트 and invents no selection, save or AI send', () => {
    const html = renderToStaticMarkup(<PresetSettings project={null} goals={[]} approvals={[]} agents={[]} busy={false} refresh={async () => {}} onSelectProject={() => {}} />);
    expect(html).toContain('프로젝트를 먼저 선택하세요');
    expect(html).toContain('프로젝트 선택</button>');
    expect(html).toContain('웹 앱, API 서버, 크로스플랫폼 앱');
    expect(html).not.toContain('현재 선택');
    expect(html).not.toContain('이 프로젝트의 선택으로 저장');
    expect(html).not.toContain('전송 승인');
    expect(html).not.toContain('계획에 고정');
  });
  it('a project opens on its own loading state, not on stored values from elsewhere', () => {
    const html = renderToStaticMarkup(<PresetSettings project={{ id: 'two', name: 'Second', path: '', kind: 'folder', reviewLoopLimit: 2 }} goals={[]} approvals={[]} agents={[]} busy={false} refresh={async () => {}} />);
    expect(html).toContain('현재 프로젝트</p><h2>Second</h2>');
    expect(html).toContain('불러오는 중');
    expect(html).not.toContain('이 프로젝트의 선택으로 저장');
  });
  it('drops a previous project state, preview, AI consent and plan after a switch', () => {
    expect(forProject(state, 'two')).toBeNull();
    expect(forProject(preview, 'two')).toBeNull();
    expect(forProject(request, 'two')).toBeNull();
    expect(forProject(plan, 'two')).toBeNull();
    expect(forProject(preview, 'one')).toBe(preview);
    const second = { ...state, projectId: 'two', selections: [] };
    expect(nextAction({ state: second, plan, preview, aiRequest: request, projectId: 'two' })).toBe(nextAction({ state: second, plan: null, preview: null, aiRequest: null, projectId: 'two' }));
  });
  it('shows the current choice by name and the one next action first', () => {
    const named = { ...state, catalog: [{ preset, digest: 'a', origin: 'builtin', projectId: null }] };
    expect(selectionSummary(named).map(item => item.label)).toEqual(['웹 앱', '선택 안 함', '선택 안 함']);
    expect(selectionSummary({ ...named, selections: [] }).every(item => item.label === '선택 안 함')).toBe(true);
    expect(nextAction({ state, plan, preview: null, aiRequest: request, projectId: 'one' })).toContain('전송을 승인하거나 거절');
    expect(nextAction({ state, plan, preview: { ...preview, report: { status: 'conflict', issues: [] } }, aiRequest: null, projectId: 'one' })).toContain('저장할 수 없습니다');
    expect(nextAction({ state, plan, preview, aiRequest: null, projectId: 'one' })).toContain('직접 저장');
    expect(nextAction({ state: { ...state, selections: [] }, plan, preview: null, aiRequest: null, projectId: 'one' })).toContain('계획을 다시 세워 승인');
    expect(nextAction({ state, plan, preview: null, aiRequest: null, projectId: 'one' })).toContain('그대로 두세요');
  });
  it('plan card offers the dedicated tab without touching the fixed plan or approval', () => {
    const approval = { id: 'approval-1', goalId: 'g', operation: 'project_write', status: 'pending' } as Approval;
    const project = { id: 'one', name: 'One', path: '', kind: 'folder' as const, reviewLoopLimit: 2 };
    const html = renderToStaticMarkup(<PlanPresetCard project={project} goal={goal} approvals={[approval]} onOpenSettings={() => {}} />);
    expect(html).toContain('설정 &gt; 앱 기술·구조 열기');
    expect(html).toContain('고정값과 승인 대기는 바뀌지 않습니다');
    expect(renderToStaticMarkup(<PlanPresetCard project={project} goal={goal} approvals={[approval]} />)).not.toContain('앱 기술·구조 열기');
  });
});

// Drives the real reducer, command layer and view the way a click or key press would, with a recorded fake engine.
type El = ReactElement<Record<string, any>>;
function walk(node: ReactNode, out: El[] = []): El[] {
  if (Array.isArray(node)) node.forEach(child => walk(child, out));
  else if (isValidElement(node)) { out.push(node as El); walk((node.props as { children?: ReactNode }).children, out); }
  return out;
}
function text(node: ReactNode): string {
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  if (Array.isArray(node)) return node.map(text).join('');
  return isValidElement(node) ? text((node.props as { children?: ReactNode }).children) : '';
}
function inOrder(html: string, parts: string[]) {
  let at = -1;
  for (const part of parts) { const next = html.indexOf(part, at + 1); expect(next, part).toBeGreaterThan(at); at = next; }
}
const flush = () => new Promise(resolve => setTimeout(resolve, 0));
const one: Project = { id: 'one', name: 'One', path: '', kind: 'folder', reviewLoopLimit: 2 };
const two: Project = { id: 'two', name: 'Second', path: '', kind: 'folder', reviewLoopLimit: 2 };
const api: TechPreset = { ...preset, id: 'B-API', name: 'API 서버', purpose: 'HTTP API' };
const modular: TechPreset = { ...preset, id: 'I-MODULAR', stage: 'intermediate', name: '모듈형 단일 앱', purpose: '모듈 경계', decisions: [{ key: 'layout', value: 'modules', reason: '경계 유지' }], defaults: { layout: 'modules' }, allowedRange: [{ key: 'layout', kind: 'enum', values: ['modules', 'single'], conditions: [] }], dependencies: [{ when: 'always', requires: ['beginner'], reason: '초급 기술 세트가 필요합니다' }], conflicts: [{ with: 'B-CROSS', reason: '크로스플랫폼 앱과 함께 쓰지 않습니다' }], artifactRules: [{ target: 'plan', text: '모듈 경계를 계획에 적음', autoWrite: false }], validation: ['모듈 경계 검사'] };
const entry = (item: TechPreset) => ({ preset: item, digest: item.id, origin: 'builtin', projectId: null });
const webSelection = { projectId: 'one', stage: 'beginner' as const, presetId: 'B-WEB', version: '1.0.0', digest: 'B-WEB', values: {}, selectedAt: '', actor: 'ceo' };
const saved = (projectId: string, selections: TechPresetState['selections'], drafts: TechPresetState['drafts'] = []): TechPresetState => ({ projectId, catalog: [entry(preset), entry(api), entry(modular)], drafts, selections, report: { status: 'pass', issues: [] } });
const fixedPlan: TechPlanSnapshot = { ...plan, choices: [{ stage: 'beginner', preset, digest: 'B-WEB', values: {} }], qaChecklist: ['웹 빌드 통과'] };
const pending = { id: 'approval-1', goalId: 'g', operation: 'project_write', status: 'pending' } as Approval;
const agents = [{ id: 'cto', enabled: true } as Agent];

function screenFor(project: Project, results: Record<string, unknown> = {}, approval: Approval | null = pending) {
  let screen: PresetScreen = initialScreen(project.id);
  const calls: { command: string; args?: Record<string, unknown> }[] = [];
  const dispatch = (event: PresetEvent) => { screen = presetScreen(screen, event); };
  const call = async <T,>(command: string, args?: Record<string, unknown>) => {
    calls.push({ command, args });
    const result = results[command];
    if (result instanceof Error) throw result;
    return result as T;
  };
  const read = () => screen;
  const on = presetHandlers(dispatch, presetActions({ projectId: project.id, call, dispatch, read, refresh: async () => {}, agents }), read);
  const view = () => walk(PresetPanel({ project, screen, approval, busy: false, agentsReady: true, on }));
  const html = () => renderToStaticMarkup(<PresetPanel project={project} screen={screen} approval={approval} busy={false} agentsReady on={on} />);
  const find = (label: string) => view().find(el => el.type === 'button' && text(el).includes(label));
  const click = async (label: string) => { const found = find(label); expect(found, label).toBeDefined(); expect(found?.props.disabled, label).toBeFalsy(); found?.props.onClick(); await flush(); };
  const type = (label: string, value: string) => { const field = view().find(el => el.type === 'label' && text(el).startsWith(label))?.props.children.find((child: ReactNode) => isValidElement(child)); expect(field, label).toBeDefined(); field.props.onChange({ target: { value } }); };
  const commands = () => calls.map(item => item.command);
  return { dispatch, calls, commands, view, html, find, click, type, read };
}
function opened(project = one, results: Record<string, unknown> = {}, approval: Approval | null = pending) {
  const ui = screenFor(project, results, approval);
  ui.dispatch({ type: 'loaded', state: saved(project.id, project.id === 'one' ? [webSelection] : []) });
  ui.dispatch({ type: 'plan', plan: approval && project.id === 'one' ? fixedPlan : null });
  return ui;
}

describe('v4 preset screen as the user uses it', () => {
  it('opens on the current project, active choices, plan-fixed versions and the next action, before stages and full definitions', () => {
    const html = opened().html();
    inOrder(html, ['현재 프로젝트', 'One', '활성 선택', '웹 앱', 'B-WEB/1.0.0', '계획에 고정된 버전', '계획 1차 · 승인 대기', '초급: 웹 앱 · B-WEB/1.0.0', '다음 행동', '그대로 두세요', '단계별 선택과 검사', '규격 전체 보기', '직접 프리셋 만들기']);
    expect(html).not.toContain('재계획 필요');
    expect(opened(one, {}, null).html()).toContain('진행 중인 계획 없음');
  });
  it('steps through a stage: choice, its constraints, the combination check, then the full common definition', async () => {
    const conflict: TechPreview = { ...preview, changes: [{ stage: 'intermediate', from: null, to: '모듈형 단일 앱' }], report: { status: 'conflict', issues: [{ severity: 'error', presetId: 'I-MODULAR', field: 'conflicts', reason: '크로스플랫폼 앱과 충돌' }] } };
    const ui = opened(one, { preview_tech_preset: conflict });
    await ui.click('중급');
    expect(ui.html()).toContain('코드를 어떻게 나눌까요?');
    await ui.click('모듈형 단일 앱');
    expect(ui.commands()).toEqual([]);
    expect(ui.html()).toContain('필요: beginner · 초급 기술 세트가 필요합니다');
    expect(ui.html()).toContain('충돌: B-CROSS · 크로스플랫폼 앱과 함께 쓰지 않습니다');
    expect(ui.find('이 프로젝트의 선택으로 저장')).toBeUndefined();
    await ui.click('선택 전 차이·영향 보기');
    expect(ui.calls).toEqual([{ command: 'preview_tech_preset', args: { projectId: 'one', pick: { stage: 'intermediate', presetId: 'I-MODULAR', version: '1.0.0', values: {} } } }]);
    expect(ui.html()).toContain('중급: 선택 안 함 → 모듈형 단일 앱');
    expect(ui.html()).toContain('조합 검사: 충돌');
    expect(ui.html()).toContain('크로스플랫폼 앱과 충돌');
    expect(ui.find('이 프로젝트의 선택으로 저장')?.props.disabled).toBe(true);
    const spec = ui.html().slice(ui.html().indexOf('규격 전체 보기'));
    inOrder(spec, ['ID·버전·단계', 'I-MODULAR · 1.0.0 · 중급', '목적', '결정 항목', 'layout=modules', '기본값', '허용 범위', 'modules/single', '의존성', '충돌 규칙', '산출물 반영', '자동 쓰기 없음', '검증 기준', '모듈 경계 검사']);
  });
  it('a recommended choice and a passing preview change nothing until the explicit save, which uses the shown fingerprint', async () => {
    const ui = opened(one, { preview_tech_preset: preview, select_tech_preset: null, get_tech_presets: saved('one', [{ ...webSelection, presetId: 'B-API', digest: 'B-API' }]) });
    await ui.click('API 서버');
    await ui.click('선택 전 차이·영향 보기');
    expect(ui.read().saved?.selections[0].presetId).toBe('B-WEB');
    expect(ui.html()).toContain('직접 저장하세요');
    expect(ui.commands()).toEqual(['preview_tech_preset']);
    await ui.click('이 프로젝트의 선택으로 저장');
    expect(ui.calls[1]).toEqual({ command: 'select_tech_preset', args: { projectId: 'one', pick: { stage: 'beginner', presetId: 'B-API', version: '1.0.0', values: {} }, previewFingerprint: 'preview-fingerprint' } });
    inOrder(ui.html(), ['활성 선택', 'API 서버', '계획에 고정된 버전', '초급: 웹 앱 · B-WEB/1.0.0', '재계획 필요']);
    expect(ui.find('이 프로젝트의 선택으로 저장')).toBeUndefined();
  });
  it('a changed pick or a late preview answer for an earlier pick cannot be saved', async () => {
    const ui = opened(one, { preview_tech_preset: preview });
    await ui.click('API 서버');
    await ui.click('선택 전 차이·영향 보기');
    await ui.click('선택 안 함');
    expect(ui.find('이 프로젝트의 선택으로 저장')).toBeUndefined();
    ui.dispatch({ type: 'previewed', preview, pick: { stage: 'beginner', presetId: 'B-API', version: '1.0.0', values: {} } });
    expect(ui.read().preview).toBeNull();
  });
  it('an AI draft is sent only on approval of the shown payload and stays a candidate that changes no selection or plan', async () => {
    const ui = opened(one, { preview_tech_preset_ai_draft: request, approve_tech_preset_ai_draft: { ...request, status: 'sending' } });
    ui.type('요구와 제약', '주문 관리 웹 앱');
    await ui.click('보낼 내용과 목적지 확인');
    expect(ui.commands()).toEqual(['preview_tech_preset_ai_draft']);
    expect(ui.html()).toContain('exact payload');
    expect(ui.html()).toContain('전송을 승인하거나 거절');
    await ui.click('이 내용과 목적지로 전송 승인');
    expect(ui.calls[1]).toEqual({ command: 'approve_tech_preset_ai_draft', args: { projectId: 'one', requestId: 'request-1', payloadSha256: 'payload-hash', destinationSha256: 'destination-hash' } });
    const aiDraft = { id: 'draft-ai', projectId: 'one', origin: 'ai' as const, definition: { ...preset, id: 'U-BEGINNER-1', name: 'AI 주문 앱' }, report: { status: 'pass' as const, issues: [] }, updatedAt: '' };
    ui.dispatch({ type: 'loaded', state: saved('one', [webSelection], [aiDraft]) });
    ui.dispatch({ type: 'aiFinished', message: 'request request-1: completed' });
    await ui.click('AI 주문 앱');
    const html = ui.html();
    inOrder(html, ['활성 선택', '웹 앱', '계획에 고정된 버전', 'B-WEB/1.0.0', '고른 후보: AI 주문 앱']);
    expect(html).not.toContain('재계획 필요');
    expect(ui.commands()).not.toContain('select_tech_preset');
  });
  it('editing the requirement or refusing withdraws the send approval without a call', async () => {
    const ui = opened(one, { preview_tech_preset_ai_draft: request, refuse_tech_preset_ai_draft: { ...request, status: 'refused' } });
    ui.type('요구와 제약', '주문 관리 웹 앱');
    await ui.click('보낼 내용과 목적지 확인');
    ui.type('요구와 제약', '주문 관리 웹 앱 v2');
    expect(ui.find('전송 승인')).toBeUndefined();
    await ui.click('보낼 내용과 목적지 확인');
    await ui.click('전송 거절');
    expect(ui.commands()).toEqual(['preview_tech_preset_ai_draft', 'preview_tech_preset_ai_draft', 'refuse_tech_preset_ai_draft']);
    expect(ui.html()).toContain('전송하지 않았습니다.');
  });
  it('after a project switch the previous draft, preview and pending send never appear, even if answers arrive late', async () => {
    const first = opened(one, { preview_tech_preset: preview, preview_tech_preset_ai_draft: request });
    first.type('요구와 제약', '주문 관리 웹 앱');
    first.type('프리셋 규격 초안', '{"name":"old"}');
    await first.click('API 서버');
    await first.click('선택 전 차이·영향 보기');
    await first.click('보낼 내용과 목적지 확인');
    expect(first.html()).toContain('선택 미리보기');
    const element = PresetSettings({ project: two, goals: [], approvals: [], agents, busy: false, refresh: async () => {} }) as ReactElement;
    expect(element.key).toBe('two');
    const second = opened(two);
    second.dispatch({ type: 'loaded', state: saved('one', [webSelection]) });
    second.dispatch({ type: 'plan', plan: fixedPlan });
    second.dispatch({ type: 'previewed', preview, pick: second.read().pick });
    second.dispatch({ type: 'aiPreviewed', request });
    const html = second.html();
    expect(html).toContain('Second');
    expect(html).toContain('진행 중인 계획 없음');
    for (const old of ['exact payload', '전송 승인', '선택 미리보기', '&quot;old&quot;', '주문 관리 웹 앱', 'B-WEB/1.0.0', '고른 후보']) expect(html).not.toContain(old);
    expect(second.read().saved?.projectId).toBe('two');
  });
  it('stages, options, preview and save stay keyboard-reachable, and the narrow layout keeps save in view', async () => {
    const ui = opened(one, { preview_tech_preset: preview });
    const keys = (key: string) => ui.view().find(el => el.props.role === 'tablist')?.props.onKeyDown({ key, preventDefault: () => {}, currentTarget: { querySelectorAll: () => [] } });
    keys('ArrowRight');
    expect(ui.read().stage).toBe('intermediate');
    keys('End');
    expect(ui.read().stage).toBe('advanced');
    keys('Home');
    expect(ui.view().filter(el => el.props.role === 'tab').map(el => el.props.tabIndex)).toEqual([0, -1, -1]);
    await ui.click('API 서버');
    await ui.click('선택 전 차이·영향 보기');
    const actions = ui.view().filter(el => el.type === 'button' && el.props.role !== 'tab');
    expect(actions.every(el => el.props.type === 'button' && el.props.tabIndex === undefined)).toBe(true);
    expect(actions.map(text)).toEqual(expect.arrayContaining(['선택 전 차이·영향 보기', '이 프로젝트의 선택으로 저장']));
    // Vitest replaces CSS imports with empty modules, so read the shipped stylesheet itself.
    const { readFileSync } = await import(/* @vite-ignore */ `node:${'fs'}`) as { readFileSync: (path: URL, encoding: 'utf8') => string };
    const css = readFileSync(new URL('./v4.css', import.meta.url), 'utf8');
    const narrow = css.slice(css.indexOf('@media (max-width: 1000px)'));
    expect(narrow).toMatch(/\.v4-current, \.v4-status-grid, \.v4-spec dl \{ grid-template-columns: 1fr; \}/);
    expect(narrow).toMatch(/\.v4-save \{ position: sticky; bottom: 0;/);
    expect(css).toMatch(/\.v4-presets button:focus-visible/);
  });
});
