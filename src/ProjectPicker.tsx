import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { displayPath } from './paths';

type Props = {
  busy: boolean;
  onRegister: (path: string) => void;
  onError: (message: string) => void;
};

export default function ProjectPicker({ busy, onRegister, onError }: Props) {
  const [manual, setManual] = useState(false);
  const [path, setPath] = useState('');

  async function pick() {
    if (busy) { onError('다른 작업을 처리하고 있습니다. 잠시 후 다시 시도해 주세요.'); return; }
    try {
      const chosen = await invoke<string | null>('pick_project_folder');
      if (chosen) { setPath(chosen); onRegister(chosen); }
    } catch (error) { onError(`폴더 창을 열지 못했습니다: ${String(error)}. 아래에서 경로를 직접 입력할 수 있습니다.`); setManual(true); }
  }

  return <section className="card folder-picker-card">
    <div className="card-kicker">ADD PROJECT</div>
    <h2>프로젝트 폴더 선택</h2>
    <p>하위 폴더와 파일이 이미 있는 폴더도 프로젝트로 선택할 수 있습니다. 탐색기에서 원하는 폴더로 이동한 뒤 `이 폴더 선택`을 누르세요. 파일을 새로 만들거나 Git을 초기화하지 않습니다.</p>
    <button type="button" className="primary" onClick={() => void pick()}>폴더 선택하기 ↗</button>
    <button type="button" className="text-button" onClick={() => setManual(!manual)}>{manual ? '직접 입력 닫기' : '폴더 창이 열리지 않나요? 경로 직접 입력'}</button>
    {manual && <div className="manual-folder"><label>폴더 경로<input value={path} onChange={event => setPath(event.target.value)} placeholder="D:\\projects\\내 프로젝트" /></label><button type="button" className="secondary" onClick={() => path.trim() ? onRegister(path.trim()) : onError('선택할 폴더 경로를 입력해 주세요.')}>이 폴더 사용</button>{path && <small>{displayPath(path)}</small>}</div>}
  </section>;
}
