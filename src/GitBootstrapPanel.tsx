import { useState } from 'react';

export type GitBootstrapPreview = {
  projectId: string; root: string; fingerprint: string; initialized: boolean;
  suggestedAuthorName: string; suggestedAuthorEmail: string;
  files: { path: string; bytes: number; sha256: string }[];
};

export type GitBootstrapResult = { commitId: string; branch: string; committedFiles: string[]; remainingChanges: number; qaStarted: boolean };

type Props = {
  onPreview: () => Promise<GitBootstrapPreview | undefined>;
  onApply: (preview: GitBootstrapPreview, selectedFiles: string[], authorName: string, authorEmail: string, message: string) => Promise<GitBootstrapResult | undefined>;
};

export function GitBootstrapPanel({ onPreview, onApply }: Props) {
  const [preview, setPreview] = useState<GitBootstrapPreview | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [authorName, setAuthorName] = useState('');
  const [authorEmail, setAuthorEmail] = useState('');
  const [message, setMessage] = useState('chore: initialize repository');
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<GitBootstrapResult | null>(null);

  async function openPreview() {
    if (busy) return;
    setBusy(true);
    try {
      const next = await onPreview();
      if (next) {
        setPreview(next); setSelected([]); setAuthorName(next.suggestedAuthorName); setAuthorEmail(next.suggestedAuthorEmail);
      }
    } finally { setBusy(false); }
  }

  async function approve() {
    if (!preview || busy || selected.length === 0) return;
    setBusy(true);
    try {
      const completed = await onApply(preview, selected, authorName, authorEmail, message);
      if (completed) { setResult(completed); setPreview(null); }
    } finally { setBusy(false); }
  }

  return <section className="chat-context git-bootstrap-panel" aria-label="Git 저장소 초기화">
    <strong>앱에서 Git 저장소 시작</strong>
    {result ? <><p>main에 첫 커밋을 만들었습니다: <code>{result.commitId.slice(0, 12)}</code> · {result.committedFiles.length}개 파일.</p><p>{result.remainingChanges === 0 ? '남은 Git 변경 사항이 없습니다.' : `선택하지 않은 변경 사항 ${result.remainingChanges}개가 남았습니다.`} {result.qaStarted ? '기존 업무 범위와 일치해 팀 QA를 시작했습니다.' : '기존 목표의 실패 업무와 QA는 아직 별도로 확인해야 합니다.'}</p></> : <p>커밋할 파일과 작성자를 직접 확인한 뒤 앱이 프로젝트 폴더에서 고정된 Git 작업만 실행합니다.</p>}
    {!preview && !result && <button type="button" className="secondary" disabled={busy} onClick={() => void openPreview()}>{busy ? '파일 확인 중…' : '범위 확인 →'}</button>}
    {preview && <div className="git-bootstrap-review">
      <p><strong>대상 폴더</strong><br /><code>{preview.root}</code></p>
      <p>브랜치: <strong>main</strong> · {preview.initialized ? '초기화된 저장소에서 첫 커밋 재시도' : '새 저장소 초기화'} · 원격 추가와 push 없음</p>
      <div className="chat-inline-actions"><button type="button" className="text-button" onClick={() => setSelected(preview.files.map(file => file.path))}>전체 선택</button><button type="button" className="text-button" onClick={() => setSelected([])}>선택 해제</button><small>{selected.length}/{preview.files.length}개 선택</small></div>
      <div className="git-bootstrap-files" role="group" aria-label="첫 커밋에 포함할 파일">{preview.files.map(file => <label key={file.path}><input type="checkbox" checked={selected.includes(file.path)} onChange={event => setSelected(current => event.target.checked ? [...current, file.path] : current.filter(path => path !== file.path))} /><span>{file.path}</span><small>{file.bytes.toLocaleString()} B</small></label>)}</div>
      <div className="git-bootstrap-fields"><label>작성자 이름<input value={authorName} maxLength={120} onChange={event => setAuthorName(event.target.value)} /></label><label>작성자 이메일<input value={authorEmail} maxLength={180} onChange={event => setAuthorEmail(event.target.value)} /></label><label>커밋 메시지<input value={message} maxLength={200} onChange={event => setMessage(event.target.value)} /></label></div>
      <p><small>Git 설정에서 읽은 작성자 정보가 있다면 미리 채웠습니다. AI 답변의 작성자 정보는 사용하지 않습니다. 선택한 파일만 스테이징하며, Git 속성에 따른 인덱스 줄바꿈 처리가 적용될 수 있습니다.</small></p>
      <div className="chat-inline-actions"><button type="button" className="secondary" disabled={busy} onClick={() => setPreview(null)}>취소</button><button type="button" className="primary" disabled={busy || selected.length === 0 || !authorName.trim() || !authorEmail.trim() || !message.trim()} onClick={() => void approve()}>{busy ? 'Git 작업 중…' : `선택한 ${selected.length}개 파일로 첫 커밋 승인·실행`}</button></div>
    </div>}
  </section>;
}
