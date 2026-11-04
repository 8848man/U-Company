import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AiConnection } from './types';

type Props = { connection: AiConnection | undefined; value: string; onChange: (value: string) => void };

export default function ModelChoice({ connection, value, onChange }: Props) {
  const [models, setModels] = useState<string[]>([]);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  useEffect(() => {
    let cancelled = false;
    setModels([]); setError(''); setLoading(false);
    if (connection?.kind === 'http' || (connection?.kind === 'cli' && ['claude', 'codex'].includes(connection.id))) {
      setLoading(true);
      void invoke<string[]>('list_connection_models', { connection })
        .then(found => { if (!cancelled) { if (connection.kind === 'cli' && found.length === 0) setError('새 모델 선택 기능을 사용하려면 앱을 재시작해 주세요.'); else setModels(found); } })
        .catch(cause => { if (!cancelled) setError(String(cause)); })
        .finally(() => { if (!cancelled) setLoading(false); });
    }
    return () => { cancelled = true; };
  }, [connection]);
  if (!connection) return <label>모델<select value="" disabled><option>먼저 AI를 연결하세요</option></select></label>;
  const choices = Array.from(new Set([...models, connection.modelId, value].filter(Boolean)));
  const help = loading ? '모델 목록을 확인하는 중입니다.'
    : error ? `모델 목록을 가져오지 못했습니다: ${error} ${connection.kind === 'http' ? '서버 상태를 확인해 주세요.' : '기본 모델은 계속 사용할 수 있습니다.'}`
    : connection.kind === 'http' ? `${models.length}개 모델 확인됨`
    : connection.id === 'codex' ? '설치된 Codex의 모델 목록입니다. 실제 사용 권한은 실행할 때 확인됩니다.'
    : connection.id === 'claude' ? 'Claude의 모델 별칭입니다. 계정에서 사용 가능한지는 실행할 때 확인됩니다.'
    : '이 연결은 모델 목록을 제공하지 않아 기본 모델을 사용합니다.';
  return <label>모델 선택<select value={value} onChange={event => onChange(event.target.value)}><option value="">연결의 기본 모델 사용</option>{choices.map(model => <option value={model} key={model}>{model}{models.includes(model) ? '' : loading || error ? ' (저장됨 · 목록 미확인)' : ' (저장됨 · 현재 목록에 없음)'}</option>)}</select><small className="field-help" aria-live="polite">{help}</small></label>;
}
