import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ListDisclosure, useLimitedItems } from './ContentDensity';
import type { Agent, Project, SpecDraft, SpecProfile, SpecSource } from './types';

const template = JSON.stringify({
  schemaVersion: 1,
  project: { summary: '이 프로젝트의 목적, 제약, 완료 기준' },
  agents: [
    { key: 'cto', role: '기술 책임자', instructions: '이 프로젝트의 기술 판단 기준' },
    { key: 'pm', role: '업무 기획자', instructions: '업무 분해와 인계 기준' },
    { key: 'developer', role: '구현 담당', instructions: '구현 및 검증 기준' },
    { key: 'qa', role: '품질 검증자', instructions: '검토 및 반려 기준' },
  ],
  workflow: [{ title: '프로젝트 고유 흐름', description: '기존 승인·QA 단계 안에서 참고할 업무 순서' }],
  exceptions: [{ title: '기존 하네스 규칙', scope: 'harness_reference', content: '변경하지 않고 참고할 예외 규칙' }],
}, null, 2);

function downloadJson(name: string, content: string) {
  const url = URL.createObjectURL(new Blob([content], { type: 'application/json;charset=utf-8' }));
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

type Props = {
  project: Project | null;
  agents: Agent[];
  profile?: SpecProfile;
  savedDraft?: SpecDraft;
  busy: boolean;
  backendOutdated: boolean;
  onApplied: () => Promise<void>;
  onError: (message: string) => void;
};

type PortableStatus = { state: 'absent' | 'current' | 'changed' | 'invalid'; path: string; detail: string };

export default function SpecImportPanel({ project, agents, profile, savedDraft, busy, backendOutdated, onApplied, onError }: Props) {
  const [sources, setSources] = useState<SpecSource[]>([]);
  const [pasted, setPasted] = useState('');
  const [preview, setPreview] = useState<SpecDraft | null>(null);
  const [bindings, setBindings] = useState<Record<string, string>>({});
  const [working, setWorking] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [publishConfirm, setPublishConfirm] = useState(false);
  const [draftFileConfirm, setDraftFileConfirm] = useState(false);
  const [portable, setPortable] = useState<PortableStatus | null>(null);
  const draft = preview?.projectId === project?.id ? preview : savedDraft;
  const blocked = busy || working || backendOutdated;
  const visibleAgents = useLimitedItems(draft?.profile.agents ?? []);
  const visibleWorkflow = useLimitedItems(draft?.profile.workflow ?? []);
  const visibleExceptions = useLimitedItems(draft?.profile.exceptions ?? []);

  useEffect(() => {
    let active = true;
    setPortable(null);
    if (project && !backendOutdated) void invoke<PortableStatus>('get_portable_spec_status', { projectId: project.id })
      .then(result => { if (active) setPortable(result); })
      .catch(error => { if (active) setPortable({ state: 'invalid', path: '.ucompany/spec.json', detail: String(error) }); });
    return () => { active = false; };
  }, [project?.id, profile?.revision, backendOutdated]);

  async function refreshPortable() {
    if (!project) return;
    try { setPortable(await invoke<PortableStatus>('get_portable_spec_status', { projectId: project.id })); }
    catch (error) { onError(String(error)); }
  }

  async function previewPortable() {
    if (!project) return;
    if (blocked) { onError(backendOutdated ? '앱을 최신 백엔드로 다시 시작해 주세요.' : '진행 중인 작업을 마친 뒤 다시 시도해 주세요.'); return; }
    setWorking(true);
    try {
      const result = await invoke<SpecDraft>('preview_portable_spec', { projectId: project.id });
      setPreview(result);
      setBindings({});
      await onApplied();
    } catch (error) { onError(String(error)); }
    finally { setWorking(false); }
  }

  async function publishPortable() {
    if (!project) return;
    if (blocked) { onError(backendOutdated ? '앱을 최신 백엔드로 다시 시작해 주세요.' : '진행 중인 작업을 마친 뒤 다시 시도해 주세요.'); return; }
    setWorking(true);
    try {
      setPortable(await invoke<PortableStatus>('create_portable_spec', { projectId: project.id }));
      setPublishConfirm(false);
    } catch (error) { onError(String(error)); }
    finally { setWorking(false); }
  }

  async function publishDraft() {
    if (!draft || !project) return;
    if (blocked) { onError(backendOutdated ? '앱을 최신 백엔드로 다시 시작해 주세요.' : '진행 중인 작업을 마친 뒤 다시 시도해 주세요.'); return; }
    setWorking(true);
    try {
      await invoke<PortableStatus>('create_portable_spec_from_draft', { draftId: draft.id });
      const fromFile = await invoke<SpecDraft>('preview_portable_spec', { projectId: project.id });
      setPreview(fromFile);
      setBindings({});
      setDraftFileConfirm(false);
      await refreshPortable();
      await onApplied();
    } catch (error) { onError(String(error)); }
    finally { setWorking(false); }
  }

  async function readFiles(files: FileList | null) {
    if (!files) return;
    if (files.length > 20) { onError('명세 파일은 한 번에 최대 20개까지 선택할 수 있습니다.'); return; }
    const chosen = Array.from(files);
    if (chosen.reduce((total, file) => total + file.size, 0) > 512 * 1024) { onError('명세 전체 크기는 512 KiB 이하여야 합니다.'); return; }
    try {
      const selected = await Promise.all(chosen.map(async file => ({ name: file.name, content: await file.text() })));
      setSources(selected);
      setPreview(null);
      setBindings({});
    } catch (error) { onError(`파일을 읽지 못했습니다: ${String(error)}`); }
  }

  async function createPreview() {
    if (!project) { onError('먼저 프로젝트를 선택해 주세요.'); return; }
    if (blocked) { onError(backendOutdated ? '앱을 최신 백엔드로 다시 시작해 주세요.' : '진행 중인 작업을 마친 뒤 다시 시도해 주세요.'); return; }
    const input = [...sources, ...(pasted.trim() ? [{ name: '붙여넣은 SDD.md', content: pasted.trim() }] : [])];
    if (!input.length) { onError('SDD 파일을 선택하거나 문서 내용을 붙여 넣어 주세요.'); return; }
    setWorking(true);
    try {
      const result = await invoke<SpecDraft>('preview_spec_import', { projectId: project.id, sources: input });
      setPreview(result);
      setBindings({});
      await onApplied();
    } catch (error) { onError(String(error)); }
    finally { setWorking(false); }
  }

  async function apply() {
    if (!draft || !project || draft.projectId !== project.id) return;
    if (blocked) { onError(backendOutdated ? '앱을 최신 백엔드로 다시 시작해 주세요.' : '진행 중인 작업을 마친 뒤 다시 시도해 주세요.'); return; }
    setWorking(true);
    try {
      await invoke<SpecProfile>('apply_spec_import', { draftId: draft.id, bindings: draft.profile.agents.map(item => ({ key: item.key, agentId: bindings[item.key] || null })) });
      setConfirm(false);
      setPreview(null);
      await onApplied();
    } catch (error) { onError(String(error)); }
    finally { setWorking(false); }
  }

  return <section className="card spec-import-card">
    <div className="card-kicker">PROJECT SDD</div>
    <h2>외부 명세 가져오기</h2>
    <p>프로젝트·팀원·업무 흐름을 미리보고 적용합니다. 기존 하네스 파일과 연결된 AI는 변경하지 않습니다.</p>
    <p>명세 원본은 프로젝트 폴더에 둡니다. DB에는 기본 프리셋, 검토 후 적용한 읽기용 사본, 실행 기록만 저장합니다. AI에게 자동 전달하거나 현재 실행 중인 목표의 절차를 바꾸지 않습니다.</p>
    <p>자격 증명과 API 키는 제거한 사본을 선택하세요. 앱은 알려진 비밀 값 형식을 일부 차단하지만 모든 비밀 값을 식별할 수는 없습니다.</p>
    <p><strong>대상:</strong> {project?.name ?? '프로젝트를 먼저 선택하세요'} · 현재 명세 v{profile?.revision ?? 0}</p>
    {project && <div className="spec-preview">
      <h3>도구 간 공통 명세</h3>
      <p><strong>.ucompany/spec.json</strong> · {portable?.detail ?? '파일 상태를 확인하는 중입니다'}</p>
      <p>이 파일은 프로젝트·역할·업무 흐름의 공유 명세 원본입니다. 다른 도구가 수정하면 새로고침 후 파일을 미리보고 적용합니다. 목표 기록, 승인, 로그인 정보는 포함하지 않습니다.</p>
      <div className="button-row">
        <button type="button" className="secondary" onClick={() => void refreshPortable()}>파일 상태 새로고침</button>
        {portable?.state === 'absent' && profile && !draft && <button type="button" className="secondary" onClick={() => blocked ? onError('앱 재시작이나 진행 중인 작업이 끝난 뒤 파일을 만드세요.') : setPublishConfirm(true)}>기존 적용 명세를 프로젝트 파일로 옮기기</button>}
        {portable?.state === 'changed' && <button type="button" className="secondary" onClick={() => void previewPortable()}>프로젝트 파일 미리보기</button>}
      </div>
    </div>}
    <details><summary>팀원 명세 템플릿 보기</summary><pre className="report">{template}</pre><div className="button-row"><button type="button" className="secondary" onClick={() => { setSources([]); setPasted(template); setPreview(null); }}>템플릿을 입력란에 채우기</button><button type="button" className="secondary" onClick={() => downloadJson('ai-org-spec-template.json', template)}>템플릿 내려받기</button></div></details>
    {profile && <button type="button" className="text-button" onClick={() => downloadJson(`ai-org-spec-${profile.revision}.json`, JSON.stringify({ schemaVersion: 1, project: { summary: profile.projectSummary }, agents: profile.agents, workflow: profile.workflow, exceptions: profile.exceptions }, null, 2))}>적용된 명세 JSON 내보내기 ↗</button>}
    <label>SDD, AGENTS.md, CLAUDE.md 또는 JSON 파일<input type="file" multiple accept=".md,.txt,.json" onChange={event => void readFiles(event.target.files)} /></label>
    {sources.length > 0 && <small>선택한 파일: {sources.map(item => item.name).join(', ')}</small>}
    <label>문서 내용 직접 붙여넣기<textarea rows={5} value={pasted} onChange={event => setPasted(event.target.value)} placeholder="외부 SDD 내용을 붙여 넣으세요" /></label>
    <button type="button" className="secondary" onClick={() => void createPreview()}>명세 미리보기 만들기</button>
    {draft && <div className="spec-preview">
      <h3>적용 전 검토 · v{draft.baseRevision + 1}</h3>
      <p>프로젝트 설명 {draft.profile.projectSummary ? '있음' : '없음'} · 팀원 {draft.profile.agents.length} · 업무 흐름 {draft.profile.workflow.length} · 예외 {draft.profile.exceptions.length}</p>
      <p>출처: {draft.profile.sources.join(', ')}</p>
      {draft.profile.projectSummary && <details><summary>프로젝트 명세</summary><pre className="report">{draft.profile.projectSummary}</pre></details>}
      {visibleAgents.visible.map(item => <details className="spec-mapping" key={`${item.source}:${item.key}`}><summary>{item.key} · {item.role} · {item.source}</summary><p>{item.instructions}</p><label>연결할 팀원<select value={bindings[item.key] ?? ''} onChange={event => setBindings(current => ({ ...current, [item.key]: event.target.value }))}><option value="">미연결 placeholder</option>{agents.filter(agent => agent.enabled).map(agent => <option key={agent.id} value={agent.id}>{agent.name} · {agent.role}</option>)}</select></label></details>)}
      <ListDisclosure label="팀원 명세" total={visibleAgents.total} hidden={visibleAgents.hidden} expanded={visibleAgents.expanded} onToggle={visibleAgents.toggle} />
      {visibleWorkflow.visible.map((item, index) => <details key={`${item.source}:${index}`}><summary>업무 흐름: {item.title} · {item.source}</summary><pre className="report">{item.description}</pre></details>)}
      <ListDisclosure label="업무 흐름" total={visibleWorkflow.total} hidden={visibleWorkflow.hidden} expanded={visibleWorkflow.expanded} onToggle={visibleWorkflow.toggle} />
      {visibleExceptions.visible.map((item, index) => <details key={`${item.source}:${index}`}><summary>예외 명세: {item.title} · {item.source}</summary><small>{item.scope}</small><pre className="report">{item.content}</pre></details>)}
      <ListDisclosure label="예외 명세" total={visibleExceptions.total} hidden={visibleExceptions.hidden} expanded={visibleExceptions.expanded} onToggle={visibleExceptions.toggle} />
      {draft.warnings.length > 0 && <p>{draft.warnings.join(' · ')}</p>}
      {draft.portableSource ? <button type="button" className="primary" onClick={() => blocked ? onError('앱 재시작이나 진행 중인 작업이 끝난 뒤 적용해 주세요.') : setConfirm(true)}>프로젝트 파일을 앱에 적용</button>
        : portable?.state === 'absent' ? <button type="button" className="primary" onClick={() => blocked ? onError('앱 재시작이나 진행 중인 작업이 끝난 뒤 파일을 만드세요.') : setDraftFileConfirm(true)}>이 초안으로 프로젝트 파일 만들기</button>
        : <p>이 초안은 프로젝트 파일의 원본이 아닙니다. 내용을 .ucompany/spec.json에 병합한 뒤 `파일 상태 새로고침`과 `프로젝트 파일 미리보기`를 사용하세요.</p>}
    </div>}
    {confirm && draft && <div className="dialog-backdrop" onClick={() => setConfirm(false)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="spec-apply-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">?</div><h2 id="spec-apply-title">이 프로젝트에 명세를 적용할까요?</h2><p>{project?.name}의 외부 명세를 v{draft.baseRevision + 1}로 저장합니다. 현재 실행 중인 AI 호출이나 다른 프로젝트의 설정은 바뀌지 않습니다.</p><p>팀원 연결 {Object.values(bindings).filter(Boolean).length}개 · 예외 명세 {draft.profile.exceptions.length}개</p><div className="dialog-actions"><button type="button" className="secondary" onClick={() => setConfirm(false)}>취소</button><button type="button" className="primary" autoFocus onClick={() => void apply()}>명세 적용</button></div></section></div>}
    {publishConfirm && <div className="dialog-backdrop" onClick={() => setPublishConfirm(false)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="spec-publish-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">?</div><h2 id="spec-publish-title">프로젝트에 공통 명세를 만들까요?</h2><p>{project?.name} 폴더에 .ucompany/spec.json을 새로 만듭니다. 기존 파일이 있으면 덮어쓰지 않습니다. 프로젝트 파일이므로 다른 도구에서도 읽고 편집할 수 있습니다.</p><div className="dialog-actions"><button type="button" className="secondary" onClick={() => setPublishConfirm(false)}>취소</button><button type="button" className="primary" autoFocus onClick={() => void publishPortable()}>파일 만들기</button></div></section></div>}
    {draftFileConfirm && draft && <div className="dialog-backdrop" onClick={() => setDraftFileConfirm(false)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="spec-draft-file-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">?</div><h2 id="spec-draft-file-title">이 초안으로 프로젝트 명세 파일을 만들까요?</h2><p>{project?.name} 폴더에 .ucompany/spec.json을 새로 만듭니다. 기존 파일은 덮어쓰지 않습니다. 생성 후 파일 내용을 다시 미리보며, 앱에 적용하려면 별도로 확인해야 합니다.</p><div className="dialog-actions"><button type="button" className="secondary" onClick={() => setDraftFileConfirm(false)}>취소</button><button type="button" className="primary" autoFocus onClick={() => void publishDraft()}>파일 만들기</button></div></section></div>}
  </section>;
}
