import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AiConnection } from './types';
import { displayPath } from './paths';

type Props = {
  onSave: (connection: AiConnection) => Promise<boolean>;
  onClose: () => void;
  onError: (message: string) => void;
};

const blank = (): AiConnection => ({ id: crypto.randomUUID(), name: '', kind: 'cli', executable: '', endpoint: '', args: [], modelId: '', status: 'needs_auth_check', capabilities: [] });

export default function ConnectionDialog({ onSave, onClose, onError }: Props) {
  const [draft, setDraft] = useState<AiConnection>(blank);
  const [models, setModels] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => { if (event.key === 'Escape') onClose(); };
    document.addEventListener('keydown', closeOnEscape);
    return () => document.removeEventListener('keydown', closeOnEscape);
  }, [onClose]);

  async function loadModels() {
    if (!draft.endpoint.trim()) { onError('먼저 모델 서버 주소를 입력해 주세요.'); return; }
    setLoading(true);
    try {
      const found = await invoke<string[]>('list_connection_models', { connection: draft });
      setModels(found);
      setDraft(current => ({ ...current, modelId: found.includes(current.modelId) ? current.modelId : found[0] ?? '' }));
    } catch (error) { setModels([]); onError(String(error)); }
    finally { setLoading(false); }
  }

  async function save() {
    if (!draft.name.trim()) { onError('연결 이름을 입력해 주세요.'); return; }
    if (draft.kind === 'http' && (!draft.endpoint.trim() || !models.includes(draft.modelId))) { onError('서버 주소를 입력하고 모델 목록에서 사용할 모델을 선택해 주세요.'); return; }
    if (draft.kind !== 'http' && !draft.executable.trim()) { onError('실행 파일 경로를 입력해 주세요.'); return; }
    if (await onSave(draft)) onClose();
  }

  return <div className="dialog-backdrop" onClick={onClose}>
    <section className="connection-dialog card" role="dialog" aria-modal="true" aria-labelledby="connection-dialog-title" onClick={event => event.stopPropagation()}>
      <div className="dialog-heading"><div><div className="card-kicker">CUSTOM AI</div><h2 id="connection-dialog-title">다른 AI 연결</h2></div><button type="button" className="dialog-x" aria-label="닫기" onClick={onClose}>×</button></div>
      <p>연결 정보를 저장하면 AI가 없는 팀원에게 자동으로 배정됩니다. 기존 배정은 유지됩니다.</p>
      <div className="form-grid"><label>이름<input autoFocus value={draft.name} onChange={event => setDraft({ ...draft, name: event.target.value })} placeholder="예: 우리 팀 AI" /></label><label>연결 방식<select value={draft.kind} onChange={event => { setModels([]); setDraft({ ...draft, kind: event.target.value as AiConnection['kind'], modelId: '', args: [] }); }}><option value="cli">CLI 실행 파일</option><option value="http">OpenAI 호환 API</option><option value="plugin">외부 어댑터</option></select></label></div>
      {draft.kind === 'http' ? <><label>API 주소<input value={draft.endpoint} onChange={event => { setModels([]); setDraft({ ...draft, endpoint: event.target.value, modelId: '' }); }} placeholder="http://127.0.0.1:1234/v1" /></label><label>인증 토큰 환경 변수 이름 (선택)<input value={draft.args[0] ?? ''} onChange={event => { setModels([]); setDraft({ ...draft, args: event.target.value ? [event.target.value] : [], modelId: '' }); }} placeholder="예: MY_AI_API_KEY" /></label><button type="button" className="secondary" onClick={() => void loadModels()}>{loading ? '모델 확인 중…' : '사용 가능한 모델 찾기'}</button><label>모델 선택<select value={draft.modelId} onChange={event => setDraft({ ...draft, modelId: event.target.value })}><option value="">{models.length ? '모델을 선택하세요' : '먼저 모델을 찾아 주세요'}</option>{models.map(model => <option key={model} value={model}>{model}</option>)}</select></label></> : <><label>실행 파일 경로<input value={draft.executable} onChange={event => setDraft({ ...draft, executable: event.target.value })} placeholder="실행 파일의 절대 경로" /></label>{draft.executable && <small className="path-line">{displayPath(draft.executable)}</small>}<label>인자 (줄마다 하나)<textarea className="small-textarea" value={draft.args.join('\n')} onChange={event => setDraft({ ...draft, args: event.target.value.split('\n').filter(Boolean) })} /></label></>}
      {draft.kind === 'plugin' && <p>외부 어댑터는 현재 사용자 권한으로 실행됩니다. 신뢰하는 파일만 등록하세요.</p>}
      <div className="dialog-actions"><button type="button" className="secondary" onClick={onClose}>취소</button><button type="button" className="primary" onClick={() => void save()}>연결 저장</button></div>
    </section>
  </div>;
}
