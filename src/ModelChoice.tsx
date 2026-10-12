import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { AiConnection } from './types';

type Props = { connection: AiConnection | undefined; value: string; onChange: (value: string) => void };

export default function ModelChoice({ connection, value, onChange }: Props) {
  const [models, setModels] = useState<string[]>([]);
  const [error, setError] = useState('');
  useEffect(() => {
    let cancelled = false;
    setModels([]); setError('');
    if (connection?.kind === 'http') {
      void invoke<string[]>('list_connection_models', { connection }).then(found => { if (!cancelled) setModels(found); }).catch(cause => { if (!cancelled) setError(String(cause)); });
    }
    return () => { cancelled = true; };
  }, [connection]);
  if (!connection) return <label>모델<select value="" disabled><option>먼저 AI를 연결하세요</option></select></label>;
  const choices = Array.from(new Set([...models, connection.modelId, value].filter(Boolean)));
  return <label>모델 선택<select value={value} onChange={event => onChange(event.target.value)}><option value="">연결의 기본 모델 사용</option>{choices.map(model => <option value={model} key={model}>{model}</option>)}</select><small className="field-help">{connection.kind === 'http' ? error || `${models.length}개 모델 확인됨` : '이 CLI는 모델 목록을 제공하지 않아 기본 모델을 사용합니다.'}</small></label>;
}
