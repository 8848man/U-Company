import { useState } from 'react';
import type { PermissionMode, PermissionModeSetting } from './types';

export function effectivePermissionMode(settings: PermissionModeSetting[], projectId: string, conversationId?: string): { mode: PermissionMode; override: PermissionMode | 'inherit' } {
  const project = settings.find(item => item.scope === 'project' && item.id === projectId)?.mode;
  const override = conversationId ? settings.find(item => item.scope === 'conversation' && item.id === conversationId)?.mode ?? 'inherit' : 'inherit';
  const mode = override !== 'inherit' ? override : project && project !== 'inherit' ? project : 'manual';
  return { mode, override };
}

const labels: Record<PermissionMode, string> = { manual: '수동', auto: '오토', full_auto: '풀 오토' };
const descriptions: Record<PermissionMode, string> = {
  manual: '필요한 명령을 기존 승인 흐름에서 확인합니다.',
  auto: '단순 로컬 조회 명령을 해당 업무에서 자동 허용합니다.',
  full_auto: '오토 범위와 승인된 파일의 단순 삭제·이동을 자동 허용합니다.',
};

export function PermissionModeControl({ scope, mode, override = 'inherit', busy = false, onChange }: { scope: 'project' | 'conversation'; mode: PermissionMode; override?: PermissionMode | 'inherit'; busy?: boolean; onChange: (mode: PermissionMode | 'inherit') => Promise<boolean> }) {
  const [saving, setSaving] = useState(false);
  const selected = scope === 'conversation' ? override : mode;
  const choices: (PermissionMode | 'inherit')[] = scope === 'conversation' ? ['inherit', 'manual', 'auto', 'full_auto'] : ['manual', 'auto', 'full_auto'];
  return <section className={`permission-mode-control ${scope === 'conversation' ? 'is-chat' : 'is-settings'}`} aria-label="AI 실행 권한">
    <div className="permission-mode-heading"><strong>AI 실행 권한 · {labels[mode]}</strong><small>{scope === 'conversation' ? '이 목표 대화에 적용' : '이 프로젝트의 기본값'}</small></div>
    <div className="permission-mode-choices" role="radiogroup" aria-label="AI 실행 권한 모드">{choices.map(choice => <button type="button" role="radio" aria-checked={selected === choice} className={selected === choice ? 'primary' : 'secondary'} key={choice} disabled={busy || saving} onClick={() => { if (selected === choice) return; setSaving(true); void onChange(choice).finally(() => setSaving(false)); }}>{choice === 'inherit' ? '프로젝트 설정 따름' : labels[choice]}</button>)}</div>
    <small>{descriptions[mode]} 시험·빌드, 웹 접속·설치·push·배포와 승인 범위 밖 파일은 별도로 확인합니다. 변경은 다음 권한 판단부터 적용됩니다.</small>
  </section>;
}
