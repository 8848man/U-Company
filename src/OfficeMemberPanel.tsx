import ModelChoice from './ModelChoice';
import { AppearanceEditor } from './office/AppearanceEditor';
import { OfficeActivity } from './OfficeActivity';
import { ListDisclosure, useLimitedItems } from './ContentDensity';
import type { MemberActivity } from './progress';
import type { Agent, AiConnection, RolePreset, Task, TokenUsageRecord } from './types';

type Props = {
  agents: Agent[];
  connections: AiConnection[];
  presets: RolePreset[];
  selectedId: string | null;
  draft: Agent | null;
  activity: MemberActivity | undefined;
  tasks: Task[];
  usage: TokenUsageRecord[];
  mode: 'activity' | 'settings';
  presetId: string;
  busy: boolean;
  onSelect: (id: string) => void;
  onMode: (mode: 'activity' | 'settings') => void;
  onDraft: (agent: Agent) => void;
  onPreset: (id: string) => void;
  onSave: () => void;
  onExplain: (title: string, detail: string, next: string) => void;
  onNotice: (message: string) => void;
  onOpenGoal: (id: string) => void;
};

export default function OfficeMemberPanel({ agents, connections, presets, selectedId, draft, activity, tasks, usage, mode, presetId, busy, onSelect, onMode, onDraft, onPreset, onSave, onExplain, onNotice, onOpenGoal }: Props) {
  const roster = useLimitedItems(agents.filter(agent => agent.enabled));
  const active = agents.find(agent => agent.id === selectedId);
  return <section className="card office-detail">
    <div className="list-head"><h2>팀원 활동</h2><small>팀 전체 변경은 위의 팀 설정에서 관리합니다.</small></div>
    <div className="office-roster">{roster.visible.map(agent => <button type="button" key={agent.id} className={selectedId === agent.id ? 'selected' : ''} onClick={() => onSelect(agent.id)}>{agent.name}</button>)}</div>
    <ListDisclosure label="팀원" total={roster.total} hidden={roster.hidden} expanded={roster.expanded} onToggle={roster.toggle} />
    {draft ? <>
      <div className="detail-tabs" role="tablist" aria-label="선택한 팀원 정보"><button type="button" role="tab" aria-selected={mode === 'activity'} className={mode === 'activity' ? 'selected' : ''} onClick={() => onMode('activity')}>활동</button><button type="button" role="tab" aria-selected={mode === 'settings'} className={mode === 'settings' ? 'selected' : ''} onClick={() => onMode('settings')}>개인 설정</button></div>
      {mode === 'activity' && active && activity ? <OfficeActivity agent={active} activity={activity} tasks={tasks} usage={usage} onOpenGoal={onOpenGoal} /> : <div className="agent-settings">
        <div className="card-kicker">{active?.name ?? 'MEMBER'}</div>
        <div className="form-grid">
          <label>이름<input value={draft.name} onChange={event => onDraft({ ...draft, name: event.target.value })} /></label>
          <label>담당 역할<input value={draft.role} onChange={event => onDraft({ ...draft, role: event.target.value })} /></label>
          <label>보고 대상<select value={draft.reportsTo ?? ''} onChange={event => onDraft({ ...draft, reportsTo: event.target.value || null })}><option value="">CEO</option>{agents.filter(agent => agent.id !== draft.id).map(agent => <option value={agent.id} key={agent.id}>{agent.name}</option>)}</select></label>
          <label>사용할 AI<select value={draft.connectionId} onChange={event => onDraft({ ...draft, connectionId: event.target.value, modelId: '' })}><option value="">AI 미배정</option>{connections.map(connection => <option value={connection.id} key={connection.id}>{connection.name} · {connection.status}</option>)}</select></label>
        </div>
        <label>역할 프리셋<select value={presetId} onChange={event => onPreset(event.target.value)}><option value="">선택해 적용</option>{presets.map(preset => <option value={preset.id} key={preset.id}>{preset.name} — {preset.summary}</option>)}</select></label>
        <button type="button" className="secondary" onClick={() => { const preset = presets.find(item => item.id === presetId); if (!preset) { onExplain('프리셋을 골라 주세요', '적용할 역할 프리셋이 선택되지 않았습니다.', '위 목록에서 역할을 고르세요.'); return; } onDraft({ ...draft, role: preset.role, instructions: preset.instructions }); onNotice('프리셋을 불러왔습니다. 변경 저장을 눌러 적용하세요.'); }}>프리셋 적용</button>
        <label>세부 역할 지침<textarea className="small-textarea" value={draft.instructions} onChange={event => onDraft({ ...draft, instructions: event.target.value })} /></label>
        <ModelChoice connection={connections.find(connection => connection.id === draft.connectionId)} value={draft.modelId} onChange={modelId => onDraft({ ...draft, modelId })} />
        <AppearanceEditor draft={draft} onDraft={onDraft} />
        <button type="button" className="primary" onClick={() => busy ? onExplain('잠시 기다려 주세요', '다른 작업이 진행 중입니다.', '잠시 후 다시 저장하세요.') : !draft.name.trim() || !draft.role.trim() ? onExplain('이름과 역할이 필요합니다', '팀원의 이름이나 담당 역할이 비어 있습니다.', '이름과 역할을 적어 주세요.') : onSave()}>변경 저장</button>
      </div>}
    </> : <div className="empty-state compact">왼쪽 캐릭터 또는 위 이름을 눌러 활동과 개인 설정을 확인하세요.</div>}
  </section>;
}
