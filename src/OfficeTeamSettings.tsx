import { useState } from 'react';
import { ListDisclosure, useLimitedItems } from './ContentDensity';
import type { Agent, AiConnection, SpecProfile, Task } from './types';

type Props = {
  agents: Agent[];
  connections: AiConnection[];
  tasks: Task[];
  importedSpec?: SpecProfile;
  busy: boolean;
  onAdd: () => void;
  onAssign: (connectionId: string, agentIds: string[]) => Promise<boolean>;
  onHide: (agent: Agent) => void;
  onRestore: (agent: Agent) => void;
  onExplain: (title: string, detail: string, next: string) => void;
};

export default function OfficeTeamSettings({ agents, connections, tasks, importedSpec, busy, onAdd, onAssign, onHide, onRestore, onExplain }: Props) {
  const active = agents.filter(agent => agent.enabled);
  const hidden = agents.filter(agent => !agent.enabled);
  const hiddenItems = useLimitedItems(hidden);
  const importedItems = useLimitedItems(importedSpec?.agents ?? []);
  const [selectedIds, setSelectedIds] = useState<string[]>(() => active.map(agent => agent.id));
  const [connectionId, setConnectionId] = useState('');
  const [confirm, setConfirm] = useState(false);
  const selected = active.filter(agent => selectedIds.includes(agent.id));
  const connection = connections.find(item => item.id === connectionId);

  function requestAssignment() {
    if (busy) { onExplain('잠시 기다려 주세요', '다른 저장 작업이 진행 중입니다.', '작업이 끝난 뒤 다시 시도하세요.'); return; }
    if (!connection) { onExplain('AI를 선택해 주세요', '일괄 배정할 등록된 AI가 선택되지 않았습니다.', 'AI 연결 메뉴에서 도구를 등록하고 여기서 선택하세요.'); return; }
    if (selected.length === 0) { onExplain('팀원을 선택해 주세요', '변경할 활성 팀원이 없습니다.', '아래 목록에서 한 명 이상 선택하세요.'); return; }
    if (connection.kind === 'http' && selected.some(agent => agent.role.toLowerCase().includes('implement') || agent.role.includes('개발'))) {
      onExplain('파일 작업 담당에게 배정할 수 없습니다', '선택한 HTTP 모델은 프로젝트 파일 편집을 지원하지 않습니다.', '개발 담당을 선택에서 빼거나 파일 작업 AI를 고르세요.'); return;
    }
    setConfirm(true);
  }

  return <div className="team-settings-layout">
    <section className="card team-settings-card">
      <div className="card-kicker">PROJECT ROLE SPEC</div>
      <h2>이 프로젝트의 팀원 명세</h2>
      <p>{importedSpec ? `적용된 외부 명세 v${importedSpec.revision} · 연결된 역할과 미연결 placeholder를 이 프로젝트에서 관리합니다.` : '아직 외부 명세가 없습니다. 프로젝트 메뉴에서 SDD를 가져올 수 있습니다.'}</p>
      {importedItems.visible.map(item => <details key={item.key}><summary>{item.key} · {agents.find(agent => agent.id === item.boundAgentId)?.name ?? '미연결 placeholder'}</summary><small>{item.source}</small><p>{item.instructions}</p></details>)}
      <ListDisclosure label="프로젝트 팀원 명세" total={importedItems.total} hidden={importedItems.hidden} expanded={importedItems.expanded} onToggle={importedItems.toggle} />
      {importedSpec && <p>업무 흐름 {importedSpec.workflow.length}개 · 예외 명세 {importedSpec.exceptions.length}개는 프로젝트 메뉴에서 확인할 수 있습니다. 이 문서는 아직 AI 호출에 전송되지 않습니다.</p>}
    </section>
    <section className="card team-settings-card">
      <div className="card-kicker">TEAM CONNECTION</div>
      <h2>팀 AI 일괄 변경</h2>
      <p>선택한 활성 팀원의 AI를 한 번에 바꿉니다. 각 팀원의 역할과 지난 작업 기록은 유지되고, 이전 AI의 모델 선택만 초기화됩니다.</p>
      <label>배정할 AI<select value={connectionId} onChange={event => setConnectionId(event.target.value)}><option value="">AI 선택</option>{connections.map(item => <option key={item.id} value={item.id}>{item.name} · {item.status === 'ready' ? '사용 가능' : item.status === 'needs_setup' ? '로그인 필요' : item.status === 'needs_auth_check' ? '로그인 확인 필요' : item.status}</option>)}</select></label>
      <div className="team-select-head"><strong>변경할 팀원 · {selected.length}/{active.length}명</strong><div><button type="button" className="text-button" onClick={() => setSelectedIds(active.map(agent => agent.id))}>전체 선택</button><button type="button" className="text-button" onClick={() => setSelectedIds([])}>선택 해제</button></div></div>
      <div className="team-select-list">{active.map(agent => <label key={agent.id}><input type="checkbox" checked={selectedIds.includes(agent.id)} onChange={event => setSelectedIds(current => event.target.checked ? [...current, agent.id] : current.filter(id => id !== agent.id))} /><span><strong>{agent.name}</strong><small>{agent.role} · {connections.find(item => item.id === agent.connectionId)?.name ?? 'AI 미배정'}</small></span></label>)}</div>
      <button type="button" className="primary" onClick={requestAssignment}>선택한 팀원의 AI 변경</button>
      {connection?.status === 'needs_setup' && <p className="team-connection-warning">이 AI는 현재 로그인이 필요한 상태로 보고됩니다. 배정은 저장되지만 로그인 확인 전에는 실행할 수 없습니다.</p>}
    </section>
    <section className="card team-settings-card">
      <div className="card-kicker">TEAM ROSTER</div>
      <h2>팀원 구성</h2>
      <p>팀원 추가와 숨김은 여기서 관리합니다. 캐릭터를 클릭하면 개인 설정과 활동을 볼 수 있습니다.</p>
      <button type="button" className="secondary" onClick={() => busy ? onExplain('잠시 기다려 주세요', '다른 작업을 처리하고 있습니다.', '잠시 후 다시 시도하세요.') : onAdd()}>+ 구성원 추가</button>
      <h3>활성 팀원</h3>
      <div className="team-manage-list">{active.filter(agent => agent.id.startsWith('developer') || agent.role.toLowerCase().includes('implement') || agent.role.includes('개발')).map(agent => {
        const unfinished = tasks.filter(task => task.assigneeId === agent.id && ['pending', 'running', 'awaiting_integration'].includes(task.status)).length;
        return <div key={agent.id}><span><strong>{agent.name}</strong><small>{unfinished ? `진행 중인 업무 ${unfinished}개` : agent.role}</small></span><button type="button" className="secondary" onClick={() => busy ? onExplain('잠시 기다려 주세요', '다른 작업을 처리하고 있습니다.', '잠시 후 다시 시도하세요.') : unfinished ? onExplain('팀원을 숨길 수 없습니다', `${agent.name}에게 끝나지 않은 업무 ${unfinished}개가 있습니다.`, '업무 완료 또는 재배정 후 다시 시도하세요.') : onHide(agent)}>숨기기</button></div>;
      })}</div>
      <h3>숨긴 팀원 · {hidden.length}명</h3>
      <p>이전 업무와 사용 기록은 보존됩니다.</p>
      <div className="team-manage-list">{hiddenItems.visible.map(agent => <div key={agent.id}><span><strong>{agent.name}</strong><small>{agent.role}</small></span><button type="button" className="secondary" onClick={() => busy ? onExplain('잠시 기다려 주세요', '다른 작업을 처리하고 있습니다.', '잠시 후 다시 시도하세요.') : onRestore(agent)}>다시 배치</button></div>)}</div>
      <ListDisclosure label="숨긴 팀원" total={hiddenItems.total} hidden={hiddenItems.hidden} expanded={hiddenItems.expanded} onToggle={hiddenItems.toggle} />
    </section>
    {confirm && connection && <div className="dialog-backdrop" onClick={() => setConfirm(false)}><section className="reason-dialog" role="dialog" aria-modal="true" aria-labelledby="team-assign-title" onClick={event => event.stopPropagation()}><div className="dialog-mark">?</div><h2 id="team-assign-title">팀 AI를 변경할까요?</h2><p>{selected.map(agent => agent.name).join(', ')} · 총 {selected.length}명에게 <strong>{connection.name}</strong>을 배정합니다.</p><p>각 팀원의 모델 선택은 새 AI의 기본값으로 바뀝니다. 진행 중인 호출과 지난 사용 기록은 수정되지 않습니다.</p><div className="dialog-actions"><button type="button" className="secondary" onClick={() => setConfirm(false)}>취소</button><button type="button" className="primary" autoFocus onClick={() => { setConfirm(false); void onAssign(connection.id, selected.map(agent => agent.id)); }}>변경 확인</button></div></section></div>}
  </div>;
}
