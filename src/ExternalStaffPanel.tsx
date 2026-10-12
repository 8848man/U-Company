import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Agent, AiConnection, CharacterPreview } from './types';
import { externalStatusLabels, isExternal } from './squad';
import PixelCharacter from './office/PixelCharacter';
import { generatedAppearance } from './office/metrocity';

type Props = {
  agents: Agent[];
  connections: AiConnection[];
  busy: boolean;
  onAction: <T>(command: string, args: Record<string, unknown>, success: string) => Promise<T | undefined>;
  onError: (message: string) => void;
};

const usable = (connection: AiConnection, slot: 'writer' | 'reviewer') => !['missing', 'failed', 'needs_setup'].includes(connection.status) && (slot === 'reviewer' || connection.kind !== 'http');

/** "외부 인력 가져오기" (ADR-0041): preview an untrusted character file, bind it to a local
 * connection, and manage goal-only contracts. Nothing is sent to an AI here. */
export function ExternalStaffPanel({ agents, connections, busy, onAction, onError }: Props) {
  const [file, setFile] = useState<{ name: string; text: string } | null>(null);
  const [preview, setPreview] = useState<CharacterPreview | null>(null);
  const [connectionId, setConnectionId] = useState('');
  const [modelId, setModelId] = useState('');
  const external = agents.filter(agent => agent.external);
  const exportable = agents.filter(agent => agent.enabled && !isExternal(agent) && (/implement|review|validation/i.test(agent.role) || agent.id === 'qa') && !['cto', 'pm'].includes(agent.id));

  async function choose(selected: File | undefined) {
    setPreview(null); setFile(null);
    if (!selected) return;
    if (selected.size > 64 * 1024) { onError('캐릭터 파일은 64KB 이하여야 합니다.'); return; }
    const text = await selected.text();
    try {
      const result = await invoke<CharacterPreview>('preview_external_staff', { fileName: selected.name, text });
      setFile({ name: selected.name, text }); setPreview(result);
      setConnectionId(connections.find(item => usable(item, result.slot))?.id ?? '');
    } catch (error) { onError(String(error)); }
  }

  async function exportOne(agent: Agent) {
    try {
      const text = await invoke<string>('export_character', { agentId: agent.id });
      const url = URL.createObjectURL(new Blob([text], { type: 'application/json' }));
      const link = document.createElement('a');
      link.href = url; link.download = `${agent.name.replace(/[^\p{L}\p{N}_-]+/gu, '_')}.character.json`; link.click();
      URL.revokeObjectURL(url);
    } catch (error) { onError(String(error)); }
  }

  return <section className="external-staff card">
    <div className="card-kicker">EXTERNAL STAFF</div>
    <h2>외부 인력 가져오기</h2>
    <p>다른 사람이 만든 캐릭터 파일(.json)을 가져옵니다. 이름·역할·지시문만 들어오고, 실행 경로·연결·키는 들어오지 않습니다. 가져온 인력은 대기 상태로 있다가, 계획 승인 카드에서 CEO가 스쿼드에 넣을 때만 그 목표 동안 일합니다.</p>
    <label className="file-pick">캐릭터 파일 선택<input type="file" accept=".json,application/json" onChange={event => { void choose(event.target.files?.[0]); event.target.value = ''; }} /></label>
    {preview && file && <div className="character-preview">
      <div className="appearance-preview"><PixelCharacter index={0} working={false} appearance={preview.appearance ?? generatedAppearance(preview.sha256)} /></div>
      <strong>{preview.name}</strong>{preview.title && <span> · {preview.title}</span>}
      <small>자리: {preview.slot === 'writer' ? '작성자' : '검토자'}{preview.capabilities.length ? ` · 업무 능력: ${preview.capabilities.join(', ')}` : ''}{preview.author ? ` · 만든 사람: ${preview.author}` : ''}</small>
      <pre className="character-instructions">{preview.instructions}</pre>
      {preview.ignoredFields.length > 0 && <small className="squad-shortfall">가져오지 않은 항목: {preview.ignoredFields.join(', ')} (실행·연결 정보는 항상 무시합니다)</small>}
      <small>파일 지문 {preview.sha256.slice(0, 12)} · 지시문은 AI에게 '외부에서 온 참고 메모'로만 전달되고 배정·파일 범위·안전 규칙을 바꿀 수 없습니다.</small>
      <label>쓸 AI 연결<select value={connectionId} onChange={event => setConnectionId(event.target.value)}>
        <option value="">연결 선택</option>
        {connections.filter(item => usable(item, preview.slot)).map(item => <option key={item.id} value={item.id}>{item.name}</option>)}
      </select></label>
      <label>모델(선택)<input value={modelId} maxLength={100} placeholder="비우면 연결 기본값" onChange={event => setModelId(event.target.value)} /></label>
      <div className="dialog-actions"><button type="button" className="secondary" onClick={() => { setPreview(null); setFile(null); }}>취소</button><button type="button" className="primary" disabled={busy || !connectionId} onClick={async () => { const saved = await onAction<Agent>('import_external_staff', { fileName: file.name, text: file.text, connectionId, modelId }, `${preview.name}을(를) 외부 인력(대기)으로 가져왔습니다.`); if (saved) { setPreview(null); setFile(null); } }}>대기 인력으로 가져오기</button></div>
    </div>}
    {external.length > 0 && <ul className="external-list">{external.map(agent => {
      const contract = agent.external!;
      return <li key={agent.id}>
        <span><strong>{agent.name}</strong> <small>{contract.slot === 'writer' ? '작성자' : '검토자'} · {externalStatusLabels[contract.status] ?? contract.status} · {contract.sourceName}</small></span>
        <span className="external-actions">
          {contract.status !== 'hired' && <button type="button" className="secondary" disabled={busy} onClick={() => void onAction('hire_external_staff', { agentId: agent.id }, `${agent.name}을(를) 정식 채용했습니다. 이제 일반 팀원처럼 배정됩니다.`)}>정식 채용</button>}
          {['standby', 'contracted'].includes(contract.status) && <button type="button" className="secondary" disabled={busy} onClick={() => void onAction('end_external_contract', { agentId: agent.id }, `${agent.name}의 계약을 끝냈습니다. 기록은 남습니다.`)}>{contract.status === 'standby' ? '돌려보내기' : '계약 종료'}</button>}
        </span>
      </li>;
    })}</ul>}
    {exportable.length > 0 && <details className="external-export"><summary>내 팀원을 캐릭터 파일로 내보내기</summary>
      <p>이름·역할·지시문만 담기며 연결 정보는 담기지 않습니다.</p>
      <div className="external-actions">{exportable.map(agent => <button key={agent.id} type="button" className="secondary" onClick={() => void exportOne(agent)}>{agent.name}</button>)}</div>
    </details>}
  </section>;
}
