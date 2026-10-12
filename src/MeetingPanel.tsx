import { useEffect, useMemo, useState } from 'react';
import { currentStagePackets, meetingAttention, meetingMembers, meetingStageLabels } from './meetingStatus';
import type { DecisionPackage, Goal, Meeting, MeetingContribution, MeetingPlan, MeetingReadiness, PlanInput, Snapshot, TraceNode } from './types';
import { DEBATE_STAGES, controlLine, headline, meetingIssues, plainDisposition, plainStageLabels, stageDigests } from './plainText';
import { SummaryText } from './SummaryText';
import { ArchiveGoalButton } from './ArchiveGoalButton';

type Handlers = {
  onReadiness: () => Promise<MeetingReadiness>;
  onUpdatePlan: (meetingId: string, baseRevision: number, plan: PlanInput) => Promise<MeetingPlan>;
  onApprovePlan: (meetingId: string, revision: number, digest: string, mode: 'plan' | 'per_stage') => Promise<void>;
  onRejectPlan: (meetingId: string) => Promise<void>;
  onRevokePlan: (meetingId: string) => Promise<void>;
  onDecide: (approvalId: string, previewRevision: number, payloadSha256: string, approved: boolean, confirmDuplicate: boolean) => Promise<void>;
  onDispatch: (meetingId: string) => Promise<void>;
  onStop: (meetingId: string) => Promise<void>;
  onResume: (meetingId: string) => Promise<void>;
  onRetryPacket: (meetingId: string, packetId: string, confirmDuplicate: boolean) => Promise<void>;
  onContinueSourceSnapshot: (meetingId: string) => Promise<void>;
  onClosePartial: (meetingId: string) => Promise<void>;
  onDecision: (meetingId: string, kind: 'adopt' | 'revise' | 'defer', recommendationIds: string[], note: string) => Promise<string | null>;
  onDraft: (draft: string, meetingId: string, packageSha256: string) => void;
};
type Props = Handlers & { snapshot: Snapshot; goal: Goal; onClose: () => void; backendBuild?: string; onArchive?: () => Promise<boolean> };
/** Three depths of the UI/UX philosophy (sdd/ux-principles.md): 1 한눈에 · 2 흐름 요약 · 3 자세히. */
type Depth = { level: 1 } | { level: 2 } | { level: 3; view: 'stage'; stage: string } | { level: 3; view: 'issue'; issue: number } | { level: 3; view: 'all' };

const STAGES = ['proposing', 'critiquing', 'revising', 'evaluating'];
const PROTOCOL = 'propose_critique_revise';
const HARD_CALL_CAP = 12;
/** Chair planning call + proposing/critiquing/revising per participant + one evaluation (ME-02). */
export const requiredCalls = (participants: number) => 1 + 3 * participants + 1;
export const profileLabel: Record<string, string> = { claude_cli_plan: 'Claude CLI · plan 모드(읽기 전용)', codex_cli_read_only: 'Codex CLI · read-only 샌드박스', offline_fixture: '오프라인 시험 응답(AI 아님)' };
const kindLabel: Record<string, string> = { source: '자료', claim: '주장', critique: '비판', response: '처리', revision: '수정', assessment: '평가', recommendation: '권고' };
const KIND_ORDER = ['recommendation', 'claim', 'critique', 'response', 'revision', 'assessment', 'source'];
const relLabel: Record<string, string> = { targets: '비판 대상', responds_to: '처리한 비판', revises: '수정한 주장', cites: '인용', supports: '근거 주장', assesses: '평가한 주장' };
const sourceBadge: Record<string, string> = { verified: '인용 확인됨', not_found: '인용을 자료에서 찾지 못함', no_quote: '인용 없음', unverified: '근거 미확인', none: '근거 없음' };
const verdictLabel: Record<string, string> = { supported: '평가자: 뒷받침됨', unsupported: '평가자: 뒷받침 부족', uncertain: '평가자: 불확실', not_assessed: '평가 없음' };
const dispositionLabel: Record<string, string> = { accept: '수용', rebut: '근거 있는 반박', defer: '보류', unanswered: '미처리', accept_but_flagged_by_evaluator: '수용했으나 평가자가 미해결로 표시' };

type Item = Record<string, unknown>;
const list = (value: unknown): Item[] => Array.isArray(value) ? value as Item[] : [];
const text = (value: unknown): string => typeof value === 'string' ? value : '';

function Sources({ items }: { items: unknown }) {
  const sources = list(items);
  if (!sources.length) return <small className="meeting-badge muted">근거 없음</small>;
  return <>{sources.map((source, index) => <small key={index} className={`meeting-badge ${text(source.status)}`}>{text(source.sourceId)} · {sourceBadge[text(source.status)] ?? text(source.status)}{text(source.quote) ? ` · “${text(source.quote)}”` : ''}</small>)}</>;
}

function Contribution({ item }: { item: MeetingContribution }) {
  const content = item.content;
  return <div className="meeting-contribution">
    {item.offlineFixture && <p className="meeting-warning">오프라인 시험 응답입니다. 실제 AI 의견이 아닙니다.</p>}
    {(item.normalizations ?? []).length > 0 && <small>표기 정규화: {item.normalizations!.join(' · ')}</small>}
    {list(content.claims).map(claim => <div key={text(claim.id)} className="meeting-claim"><strong>{text(claim.id)}</strong> {text(claim.text)}<div><Sources items={claim.sources} /></div>{list(claim.assumptions).length > 0 && <small>가정: {(claim.assumptions as string[]).join(' · ')}</small>}{list(claim.unknowns).length > 0 && <small>미확인: {(claim.unknowns as string[]).join(' · ')}</small>}</div>)}
    {list(content.alternatives).length > 0 && <small>검토한 대안: {(content.alternatives as string[]).join(' · ')}</small>}
    {list(content.critiques).map(critique => <div key={text(critique.id)} className="meeting-claim"><strong>{text(critique.id)} → {text(critique.targetClaimId)}</strong> {text(critique.text)}<div><Sources items={critique.sources} /></div>{text(critique.proposedChange) && <small>수정 제안: {text(critique.proposedChange)}</small>}</div>)}
    {list(content.responses).map(response => <div key={text(response.critiqueId)} className="meeting-claim"><strong>{text(response.critiqueId)} · {dispositionLabel[text(response.disposition)] ?? text(response.disposition)}</strong> {text(response.text)}<div><Sources items={response.sources} /></div></div>)}
    {list(content.revisedClaims).map(claim => <div key={`r-${text(claim.id)}`} className="meeting-claim"><strong>수정된 {text(claim.id)}</strong> {text(claim.text)}<div><Sources items={claim.sources} /></div></div>)}
    {list(content.assessments).map(assessment => <div key={text(assessment.claimId)} className="meeting-claim"><strong>{text(assessment.claimId)} · {verdictLabel[text(assessment.verdict)] ?? text(assessment.verdict)}</strong> {text(assessment.note)}</div>)}
    {list(content.recommendations).map(recommendation => <div key={text(recommendation.id)} className="meeting-claim"><strong>권고 {text(recommendation.id)}</strong> {text(recommendation.text)}</div>)}
    <details><summary>제공자 응답 원문</summary><pre className="meeting-payload">{item.rawResponse}</pre></details>
  </div>;
}

function PackageView({ pkg, meeting, selected, onToggle }: { pkg: DecisionPackage; meeting: Meeting; selected: string[]; onToggle: (id: string) => void }) {
  const viewpoint = (slotId: string) => { const slot = meeting.slots.find(item => item.id === slotId); return slot?.agentName || slot?.viewpoint || slotId; };
  const reported = pkg.usage.filter(item => item.usageSource === 'reported');
  const estimated = pkg.usage.filter(item => item.usageSource !== 'reported');
  const sum = (items: typeof pkg.usage) => items.reduce((total, item) => total + item.inputTokens + item.outputTokens, 0);
  return <section className="card meeting-package" aria-label="결정 자료">
    <div className="v2-row"><h3>결정 자료{pkg.partial ? ' · 부분 결과' : ''}</h3><small>SHA-256 {pkg.sha256.slice(0, 12)}…</small></div>
    <p><strong>종료 이유:</strong> {pkg.terminationReason}</p>
    <p className="meeting-warning">{pkg.independenceNote}</p>
    <h4>권고 {pkg.recommendations.length}개</h4>
    {pkg.recommendations.length === 0 && <p>평가 단계까지 진행하지 않아 권고가 없습니다. 아래 주장과 반론만 근거로 판단하세요.</p>}
    {pkg.recommendations.map(item => <label key={item.id} className="meeting-recommendation"><input type="checkbox" checked={selected.includes(item.id)} disabled={meeting.stage !== 'package_ready'} onChange={() => onToggle(item.id)} /><span><strong>{item.id}</strong> {item.text}<small> 근거 주장: {item.claimIds.join(', ') || '없음'}</small></span></label>)}
    <h4>미해결 반론 {pkg.dissent.length}개</h4>
    {pkg.dissent.length === 0 ? <p>남은 반론이 없습니다.</p> : pkg.dissent.map(item => <div key={item.critiqueId} className="meeting-claim"><strong>{item.critiqueId} ({viewpoint(item.criticSlotId)}) → {item.targetClaimId} · {dispositionLabel[item.disposition] ?? item.disposition}</strong> {item.text}{item.response && <small>응답: {item.response}</small>}</div>)}
    <h4>주장과 근거</h4>
    {pkg.claims.map(claim => <div key={claim.id} className="meeting-claim"><strong>{claim.id} · {viewpoint(claim.slotId)}{claim.revised ? ' · 수정됨' : ''}</strong> {claim.text}<div><small className={`meeting-badge ${claim.sourceStatus}`}>{sourceBadge[claim.sourceStatus]}</small><small className="meeting-badge muted">{verdictLabel[claim.evaluatorVerdict] ?? claim.evaluatorVerdict}</small></div>{claim.assumptions.length > 0 && <small>가정: {claim.assumptions.join(' · ')}</small>}{claim.unknowns.length > 0 && <small>미확인: {claim.unknowns.join(' · ')}</small>}{claim.evaluatorNote && <small>평가 메모: {claim.evaluatorNote}</small>}</div>)}
    {pkg.alternatives.length > 0 && <><h4>검토한 대안</h4><ul>{pkg.alternatives.map((item, index) => <li key={index}>{item}</li>)}</ul></>}
    {pkg.options.length > 0 && <p>평가자가 제시한 선택지: {pkg.options.join(' · ')}</p>}
    <details><summary>모델 출처와 사용량</summary>
      {pkg.provenance.map(item => <p key={item.slotId}>{item.viewpoint}: {item.connectionName} · {item.modelId || '기본 모델'} · 연결 지문 {item.connectionFingerprint.slice(0, 12)}…{item.offlineFixture ? ' · 오프라인 시험(AI 아님)' : ''}</p>)}
      <p>호출 {pkg.usage.length}회 · 제공자 보고 {sum(reported).toLocaleString()} 토큰 · 추정 {sum(estimated).toLocaleString()} 토큰 (추정치는 요금이나 한도가 아닙니다)</p>
    </details>
  </section>;
}


const lines = (value: string) => value.split(/\r?\n/).map(item => item.trim()).filter(Boolean);
export function planInputOf(plan: MeetingPlan): PlanInput {
  return { objective: plan.objective, constraints: plan.constraints, participants: plan.participants, evaluatorAgentId: plan.evaluatorAgentId, protocol: plan.protocol || PROTOCOL, rounds: plan.rounds || 1, maxCalls: plan.maxCalls, sources: plan.sources, successCriteria: plan.successCriteria };
}
type PlanDraft = { objective: string; constraints: string; participants: { agentId: string; stance: string }[]; evaluatorAgentId: string; maxCalls: number; sources: string; successCriteria: string };
const draftOf = (plan: MeetingPlan): PlanDraft => ({ objective: plan.objective, constraints: plan.constraints.join('\n'), participants: plan.participants.map(item => ({ ...item })), evaluatorAgentId: plan.evaluatorAgentId, maxCalls: plan.maxCalls, sources: plan.sources.map(item => item.reason ? `${item.path} | ${item.reason}` : item.path).join('\n'), successCriteria: plan.successCriteria.join('\n') });
export function inputOfDraft(draft: PlanDraft): PlanInput {
  return { objective: draft.objective.trim(), constraints: lines(draft.constraints), participants: draft.participants.filter(item => item.agentId).map(item => ({ agentId: item.agentId, stance: item.stance.trim() })), evaluatorAgentId: draft.evaluatorAgentId, protocol: PROTOCOL, rounds: 1, maxCalls: draft.maxCalls, sources: lines(draft.sources).map(line => { const [path, ...reason] = line.split('|'); return { path: path.trim(), reason: reason.join('|').trim() }; }), successCriteria: lines(draft.successCriteria) };
}

function PlanCard({ meeting, readiness, pending, onSave, onApprove, onReject }: { meeting: Meeting; readiness: MeetingReadiness | null; pending: boolean; onSave: (input: PlanInput) => void; onApprove: (mode: 'plan' | 'per_stage') => void; onReject: () => void }) {
  const plan = meeting.plan!;
  const [draft, setDraft] = useState<PlanDraft>(() => draftOf(plan));
  useEffect(() => { setDraft(draftOf(plan)); }, [plan.revision, plan.digest]);
  const dirty = JSON.stringify(inputOfDraft(draft)) !== JSON.stringify(inputOfDraft(draftOf(plan)));
  const members = readiness?.members ?? [];
  const member = (id: string) => members.find(item => item.agentId === id);
  const nameOf = (id: string) => member(id)?.name ?? meeting.slots.find(slot => slot.id === id)?.agentName ?? id;
  const chosen = new Set([...draft.participants.map(item => item.agentId), draft.evaluatorAgentId, meeting.chairAgentId ?? '']);
  const needed = requiredCalls(draft.participants.filter(item => item.agentId).length);
  const option = (id: string, current: string) => { const item = member(id); return <option key={id} value={id} disabled={id !== current && (chosen.has(id) || (item ? !item.eligible : false))}>{nameOf(id)}{item ? ` · ${item.role}${item.eligible ? ` · ${item.connectionName}` : ` · 불가: ${item.reason}`}` : ''}</option>; };
  const ids = members.length ? members.map(item => item.agentId) : meeting.slots.map(slot => slot.id);
  const update = (patch: Partial<PlanDraft>) => setDraft(current => ({ ...current, ...patch }));
  const disclosed = [...draft.participants.map(item => item.agentId), draft.evaluatorAgentId].filter(Boolean);
  return <section className="card v2-stack meeting-plan" aria-label="회의 계획">
    <div className="v2-row"><h3>회의 계획 r{plan.revision}</h3><small>{plan.origin === 'chair' ? `의장 ${nameOf(meeting.chairAgentId ?? '')} 제안` : 'CEO 수정'} · 지문 {plan.digest.slice(0, 12)}…</small></div>
    {plan.errors.length > 0 && <div className="meeting-warning" role="alert"><strong>승인 전에 고칠 문제 {plan.errors.length}개</strong><ul>{plan.errors.map((error, index) => <li key={index}>{error}</li>)}</ul></div>}
    <label>회의 목표<textarea value={draft.objective} onChange={event => update({ objective: event.target.value })} /></label>
    <label>제약 (한 줄에 하나)<textarea value={draft.constraints} onChange={event => update({ constraints: event.target.value })} /></label>
    <fieldset className="meeting-members"><legend>토론 참여자 2~3명과 입장</legend>
      {draft.participants.map((item, index) => <div className="meeting-member-row" key={index}>
        <select aria-label={`참여자 ${index + 1}`} value={item.agentId} onChange={event => update({ participants: draft.participants.map((row, at) => at === index ? { ...row, agentId: event.target.value } : row) })}><option value="">선택</option>{ids.map(id => option(id, item.agentId))}</select>
        <input aria-label={`참여자 ${index + 1} 입장`} value={item.stance} placeholder="이 팀원이 맡을 입장" onChange={event => update({ participants: draft.participants.map((row, at) => at === index ? { ...row, stance: event.target.value } : row) })} />
        <button type="button" className="secondary" disabled={draft.participants.length <= 2} onClick={() => update({ participants: draft.participants.filter((_, at) => at !== index) })}>빼기</button>
      </div>)}
      {draft.participants.length < 3 && <button type="button" className="secondary" onClick={() => update({ participants: [...draft.participants, { agentId: '', stance: '' }] })}>참여자 추가</button>}
    </fieldset>
    <label>평가자 (토론에 참여하지 않는 팀원)<select value={draft.evaluatorAgentId} onChange={event => update({ evaluatorAgentId: event.target.value })}><option value="">선택</option>{ids.map(id => option(id, draft.evaluatorAgentId))}</select></label>
    <label>호출 상한 (필요 {needed}회 + 형식 수정 예비 {Math.max(0, draft.maxCalls - needed)}회 · 최대 {HARD_CALL_CAP}회, 의장 계획 1회 포함)<input type="number" min={needed} max={HARD_CALL_CAP} value={draft.maxCalls} onChange={event => update({ maxCalls: Number(event.target.value) })} /></label>
    <label>보낼 프로젝트 자료 (한 줄에 `경로 | 이유`, UTF-8 텍스트, 파일당 16,000자·합계 26,000자 이하. 고른 파일은 참여자마다 전문이 전송됩니다)<textarea value={draft.sources} onChange={event => update({ sources: event.target.value })} placeholder="sdd/product.md | 제품 범위 확인" /></label>
    {meeting.sources.length > 0 && <div className="v2-stack"><small>자료 예산: {meeting.sources.reduce((total, source) => total + source.text.length, 0).toLocaleString()} / 26,000자</small><progress max={26000} value={meeting.sources.reduce((total, source) => total + source.text.length, 0)} />{meeting.sources.map(source => <div className="v2-row" key={source.id}><small>{source.id} {source.path} · {source.text.length.toLocaleString()}자 · SHA-256 {source.sha256.slice(0, 10)}…</small><button type="button" className="secondary" onClick={() => update({ sources: lines(draft.sources).filter(line => line.split('|')[0].trim() !== source.path).join('\n') })}>빼기</button></div>)}</div>}
    <label>성공 기준 (한 줄에 하나)<textarea value={draft.successCriteria} onChange={event => update({ successCriteria: event.target.value })} /></label>
    <div className="meeting-disclosure"><strong>승인하면 전송되는 곳</strong>
      {disclosed.map((id, index) => { const item = member(id); return <small key={`${index}-${id}`}>{nameOf(id)} → {item?.connectionName || '연결 없음'} · {item?.modelId || '기본 모델'} · {item?.eligible ? profileLabel[item.profile] ?? item.profile : `참여 불가: ${item?.reason ?? '확인 필요'}`}</small>; })}
      <small>안건, 계획, 위 자료 원문, 다른 참여자의 발언이 이 AI들에 전송됩니다. 각 AI는 목표 수행과 같은 읽기 전용 경계에서 프로젝트 폴더를 읽을 수 있지만 파일을 바꾸거나 명령을 실행할 권한은 받지 않습니다. 이미 보낸 내용은 회수할 수 없습니다.</small>
    </div>
    {dirty && <p className="meeting-note">고친 내용은 저장해야 새 계획 버전이 되고, 승인은 저장된 버전에만 적용됩니다.</p>}
    <div className="button-row">
      <button className="secondary" disabled={pending || !dirty} onClick={() => onSave(inputOfDraft(draft))}>고친 계획 저장</button>
      <button className="secondary" disabled={pending} onClick={onReject}>계획 거절 (보류로 종료)</button>
      <button className="secondary" disabled={pending || dirty || plan.errors.length > 0} onClick={() => onApprove('per_stage')}>단계마다 패킷 확인하며 진행</button>
      <button className="primary" disabled={pending || dirty || plan.errors.length > 0} onClick={() => onApprove('plan')}>계획 승인 후 진행</button>
    </div>
    <small>기본은 한 번 승인하면 제안 → 비판 → 수정 → 평가가 이 계획의 범위 안에서만 이어집니다. 계획에 없는 팀원·자료·호출은 막힙니다.</small>
  </section>;
}

function TraceExplorer({ trace, nameOf }: { trace: TraceNode[]; nameOf: (id: string) => string }) {
  const [focus, setFocus] = useState<string>(() => trace.find(node => node.kind === 'recommendation')?.id ?? trace.find(node => node.kind === 'claim')?.id ?? '');
  const byId = useMemo(() => new Map(trace.map(node => [node.id, node])), [trace]);
  const node = byId.get(focus);
  const incoming = trace.filter(item => item.links.some(link => link.target === focus));
  const jump = (id: string, label: string) => <button type="button" className="text-button" key={`${label}-${id}`} onClick={() => setFocus(id)} disabled={!byId.has(id)}>{label} {id}</button>;
  return <section className="card meeting-trace" aria-label="주장 추적">
    <h4>주장·비판·근거 추적</h4>
    <label>추적할 항목<select value={focus} onChange={event => setFocus(event.target.value)}>{[...trace].sort((x, y) => KIND_ORDER.indexOf(x.kind) - KIND_ORDER.indexOf(y.kind)).map(item => <option key={item.id} value={item.id}>{kindLabel[item.kind] ?? item.kind} {item.id} · {nameOf(item.author)}</option>)}</select></label>
    {node && <div className="meeting-claim">
      <strong>{kindLabel[node.kind] ?? node.kind} {node.id} · {nameOf(node.author)}{node.stage ? ` · ${meetingStageLabels[node.stage] ?? node.stage}` : ''}</strong>
      <span>{node.text}</span>
      {node.packetSha256 && <small>출처 패킷 {node.packetId.slice(0, 8)}… · SHA-256 {node.packetSha256.slice(0, 12)}…</small>}
      {node.links.length > 0 && <div><small>이 항목이 가리키는 곳:</small> {node.links.map(link => jump(link.target, relLabel[link.rel] ?? link.rel))}</div>}
      {incoming.length > 0 && <div><small>이 항목을 가리키는 곳:</small> {incoming.map(item => jump(item.id, kindLabel[item.kind] ?? item.kind))}</div>}
    </div>}
  </section>;
}

function crumbLabel(depth: Depth): string {
  if (depth.level === 2) return '요약';
  if (depth.level !== 3) return '';
  if (depth.view === 'stage') return `요약 › ${plainStageLabels[depth.stage] ?? depth.stage}`;
  return depth.view === 'issue' ? '요약 › 쟁점' : '모든 기록';
}

export function MeetingPanel(props: Props) {
  const { snapshot, goal, onClose } = props;
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [selected, setSelected] = useState<string[]>([]);
  const [note, setNote] = useState('');
  const [readiness, setReadiness] = useState<MeetingReadiness | null>(null);
  const [depth, setDepth] = useState<Depth>({ level: 1 });
  const meeting = (snapshot.meetings ?? []).find(item => item.goalId === goal.id);
  // UXP-4: the evaluator's recommended proposal starts selected; the CEO still presses the decision.
  const recommendedId = meeting?.stage === 'package_ready' ? meeting.package?.recommendedId ?? '' : '';
  useEffect(() => { if (recommendedId) setSelected(current => current.length ? current : [recommendedId]); }, [recommendedId, meeting?.package?.sha256]);
  const packets = (snapshot.meetingPackets ?? []).filter(packet => packet.meetingId === meeting?.id);
  const approvals = (snapshot.transmissionApprovals ?? []).filter(approval => approval.meetingId === meeting?.id);
  const contributions = (snapshot.meetingContributions ?? []).filter(item => item.meetingId === meeting?.id);
  const reviewing = meeting?.stage === 'plan_review';
  useEffect(() => { if (reviewing) void props.onReadiness().then(setReadiness).catch(cause => setError(String(cause))); }, [reviewing, meeting?.plan?.revision]);

  async function request<T>(operation: () => Promise<T>): Promise<T | undefined> {
    setPending(true); setError('');
    try { return await operation(); }
    catch (cause) { setError(String(cause)); return undefined; }
    finally { setPending(false); }
  }

  if (!meeting) return <div className="dialog-backdrop meeting-backdrop" onClick={onClose}><section className="reason-dialog meeting-panel" role="dialog" aria-modal="true" onClick={event => event.stopPropagation()}><p>회의 기록을 불러오지 못했습니다.</p><button className="secondary" onClick={onClose}>닫기</button></section></div>;

  const legacy = meeting.stage === 'legacy_closed' || (meeting.modelVersion ?? 0) < 2;
  const status = meetingAttention(goal, meeting, packets, snapshot.agents);
  const members = meetingMembers(meeting, packets, snapshot.agents);
  const current = currentStagePackets(meeting, packets);
  const allApproved = meeting.stage === 'awaiting_transfer_approval' && current.length > 0 && current.every(packet => packet.status === 'approved' || packet.status === 'sent');
  const stoppable = ['planning', 'plan_review', 'awaiting_transfer_approval', ...STAGES].includes(meeting.stage);
  const revocable = meeting.planApproval?.status === 'approved' && ['awaiting_transfer_approval', ...STAGES].includes(meeting.stage);
  const dispatched = packets.filter(packet => packet.dispatch);
  const retryCandidates = packets.filter(packet => packet.stage === meeting.activeStage && (packet.status === 'failed' || packet.status === 'outcome_unknown' || (packet.status === 'sent_invalid' && Boolean(packet.repairOf))));
  const flow = ['planning', 'plan_review', ...STAGES];
  const at = flow.indexOf(['awaiting_transfer_approval', 'plan_approved'].includes(meeting.stage) ? meeting.activeStage ?? '' : meeting.stage);
  const finished = ['package_ready', 'decision_recorded'].includes(meeting.stage);
  const nameOf = (id: string) => meeting.slots.find(slot => slot.id === id)?.agentName || meeting.slots.find(slot => slot.id === id)?.viewpoint || id;
  const digests = stageDigests(meeting, contributions, nameOf);
  const issues = meeting.package ? meetingIssues(meeting.package) : [];
  const slotLine = (id: string) => { const slot = meeting.slots.find(item => item.id === id); return slot ? `${nameOf(id)} → ${snapshot.connections.find(connection => connection.id === slot.connectionId)?.name ?? slot.connectionId} · ${slot.modelId || '기본 모델'}${slot.profile ? ` · ${profileLabel[slot.profile] ?? slot.profile}` : ''}` : id; };

  return <div className="dialog-backdrop meeting-backdrop" onClick={onClose}><section className="reason-dialog meeting-panel" role="dialog" aria-modal="true" aria-labelledby="meeting-title" onClick={event => event.stopPropagation()}>
    <div className="v2-row"><div><div className="eyebrow">OFFICE MEETING</div><h2 id="meeting-title" className="meeting-title"><SummaryText value={headline(meeting.topic, meeting.topicSummary, 80)} /></h2></div><button className="secondary" onClick={onClose}>닫기</button></div>
    {depth.level > 1 && <nav className="meeting-crumbs" aria-label="회의 안 위치"><button type="button" className="text-button" onClick={() => setDepth(depth.level === 3 && depth.view !== 'all' ? { level: 2 } : { level: 1 })}>← 뒤로</button><span>회의 › {crumbLabel(depth)}</span></nav>}

    {depth.level === 1 && <>
    <div className="v2-summary meeting-status" aria-live="polite"><strong>{status.label}</strong><small>{status.detail}</small><small>다음: {status.next}</small></div>
    {!legacy && <p className="meeting-control"><span>내가 정한 것: {controlLine(meeting)}</span><button type="button" className="text-button" onClick={() => setDepth({ level: 3, view: 'all' })}>보낸 내용 보기</button></p>}
    {legacy && <p className="meeting-warning" role="status">이 회의는 앱 업데이트 전 고정 관점 방식으로 만들어져 읽기 전용으로 닫혔습니다. 아래 기록은 그대로 볼 수 있지만 더 진행하거나 새로 전송하지 않습니다. 같은 안건은 홈에서 `회의` 모드로 다시 시작하세요.</p>}
    {!legacy && <ol className="meeting-stages">{flow.map((stage, index) => <li key={stage} className={finished || index < at ? 'done' : index === at ? 'current' : ''}>{meetingStageLabels[stage]}</li>)}<li className={meeting.stage === 'decision_recorded' ? 'done' : meeting.stage === 'package_ready' ? 'current' : ''}>내 결정</li></ol>}

    {meeting.stage === 'planning' && <p role="status">{slotLine(meeting.chairAgentId ?? meeting.slots[0]?.id ?? '')}에게 회의 계획을 요청했습니다. 계획이 오면 검토 카드가 열리며, 그 전에는 토론 호출이 없습니다.</p>}
    {reviewing && meeting.plan && <PlanCard meeting={meeting} readiness={readiness} pending={pending}
      onSave={input => void request(() => props.onUpdatePlan(meeting.id, meeting.plan!.revision, input))}
      onApprove={mode => void request(() => props.onApprovePlan(meeting.id, meeting.plan!.revision, meeting.plan!.digest, mode))}
      onReject={() => void request(() => props.onRejectPlan(meeting.id))} />}

    {meeting.stage === 'awaiting_transfer_approval' && <section className="card v2-stack" aria-label="보낼 내용 확인">
      <h3>{meetingStageLabels[meeting.activeStage ?? ''] ?? ''} 단계에서 AI에게 보낼 내용 확인</h3>
      <p>아래 원문 그대로, 표시된 팀원의 AI에게만 보냅니다. 보내지 않기로 하면 회의가 멈춥니다. 앞선 요청의 결과를 모르면 승인 방식과 관계없이 다시 확인을 받습니다.</p>
      {current.map(packet => { const approval = approvals.find(item => item.packetId === packet.id && !['superseded', 'void'].includes(item.status)); return <article className="card meeting-packet" key={packet.id}>
        <strong>{slotLine(packet.slotId)}</strong>
        <small>상태: {packet.status} · revision {packet.previewRevision} · {packet.payload.length.toLocaleString()}자 · 계획 지문 {(packet.planDigest ?? '').slice(0, 10)}… · 연결 지문 {packet.destinationFingerprint.slice(0, 12)}…{packet.repairOf ? ` · 형식 수정 요청 (${packet.repairOf.slice(0, 8)}…)` : ''}</small>
        {packet.priorOutcomeUnknown && <p className="meeting-warning">이전 revision은 앱 종료로 전송 여부를 알 수 없습니다. 승인하면 같은 내용이 제공자에게 두 번 갈 수 있습니다.</p>}
        <details><summary>전송할 원문 전체 · SHA-256 {packet.payloadSha256.slice(0, 16)}…</summary><pre className="meeting-payload">{packet.payload}</pre></details>
        {approval?.status === 'pending' && <div className="button-row">
          <button className="secondary" disabled={pending} onClick={() => void request(() => props.onDecide(approval.id, packet.previewRevision, packet.payloadSha256, false, false))}>이 내용 보내지 않기</button>
          <button className="primary" disabled={pending} onClick={() => { if (packet.priorOutcomeUnknown && !window.confirm('같은 내용이 두 번 전송될 수 있습니다. 이 패킷을 승인할까요?')) return; void request(() => props.onDecide(approval.id, packet.previewRevision, packet.payloadSha256, true, Boolean(packet.priorOutcomeUnknown))); }}>이 내용 보내기 승인</button>
        </div>}
      </article>; })}
      {allApproved && <button className="primary" disabled={pending} onClick={() => void request(() => props.onDispatch(meeting.id))}>확인한 내용 보내기</button>}
    </section>}

    {(meeting.stage === 'blocked' || meeting.stage === 'stopped') && <section className="card v2-stack"><p role="status">{meeting.blockReason || '회의가 멈췄습니다.'} 결과를 알 수 없는 호출은 자동으로 다시 보내지 않습니다.</p>{meeting.blockedAt && <small>기록 시각: {new Date(meeting.blockedAt).toLocaleString()}</small>}{meeting.blockBuild && props.backendBuild && meeting.blockBuild !== props.backendBuild && <small>이전 버전에서 기록된 이유입니다. 재개하면 현재 규칙으로 다시 확인합니다.</small>}<small>{meeting.plan ? '계획을 다시 승인하면 완료된 발언은 건너뛰고 남은 단계부터 이어갑니다.' : '재개하면 의장에게 새 계획 요청을 한 번 보냅니다.'}</small>{meeting.stage === 'blocked' && retryCandidates.map(packet => <button key={packet.id} className="secondary" disabled={pending || meeting.callsUsed >= meeting.maxCalls} onClick={() => { const duplicate = packet.status === 'outcome_unknown'; if (duplicate && !window.confirm('같은 내용이 두 번 전송될 수 있습니다. 이 패킷을 다시 요청할까요?')) return; void request(() => props.onRetryPacket(meeting.id, packet.id, duplicate)); }}>{slotLine(packet.slotId)} · 이 단계 패킷 재요청{packet.status === 'outcome_unknown' ? ' (중복 가능)' : ''}</button>)}{meeting.stage === 'blocked' && meeting.activeStage === 'proposing' && meeting.blockReason?.includes('자료') && meeting.planApproval?.status === 'approved' && <button className="secondary" disabled={pending} onClick={() => void request(() => props.onContinueSourceSnapshot(meeting.id))}>승인 시점 내용으로 계속</button>}<div className="button-row"><button className="primary" disabled={pending} onClick={() => void request(() => props.onResume(meeting.id))}>{meeting.plan ? '계획 검토로 재개' : '의장 계획 다시 요청'}</button><button className="secondary" disabled={pending} onClick={() => void request(() => props.onClosePartial(meeting.id))}>지금까지 결과로 마치기</button></div></section>}
    {meeting.sourceChangeNoted && <p className="meeting-warning">원본 파일이 승인 뒤 바뀌었습니다. 회의는 승인 시점의 자료 스냅샷을 사용합니다.</p>}

    {STAGES.includes(meeting.stage) && <p role="status">{meetingStageLabels[meeting.stage]} 단계 진행 중{members.calling.length ? ` · ${members.calling.join(', ')} 응답 대기` : ''}. 중단하면 새 호출을 시작하지 않지만 이미 시작된 호출은 회수되지 않습니다.</p>}
    {!legacy && (stoppable || revocable) && <div className="button-row">{revocable && <button className="secondary" disabled={pending} onClick={() => void request(() => props.onRevokePlan(meeting.id))}>계획 승인 철회</button>}{stoppable && <button className="secondary" disabled={pending} onClick={() => void request(() => props.onStop(meeting.id))}>회의 중단</button>}</div>}

    {meeting.package && <section className="card meeting-glance" aria-label="회의 결과">
      <h3>회의 결과{meeting.package.partial ? ' · 일부만 끝났어요' : ''}</h3>
      <SummaryText as="p" value={meeting.package.overview ? { text: meeting.package.overview, source: 'ai', raw: '' } : { text: `${members.participants.length}명이 생각 ${meeting.package.claims.length}개를 내고 서로 약한 곳을 짚었어요.`, source: 'full', raw: '' }} />
      <small>끝난 이유: {meeting.package.terminationReason}</small>
      {meeting.package.provenance.some(item => item.offlineFixture) && <p className="meeting-warning">시험용 응답이 섞여 있어요. 실제 AI 의견이 아니에요.</p>}
      <h4>제안 {meeting.package.recommendations.length}개</h4>
      {meeting.package.recommendations.length === 0 && <p>심사까지 가지 않아 제안이 없어요. 아래 쟁점과 기록을 보고 정해 주세요.</p>}
      <div className="meeting-proposals">{meeting.package.recommendations.slice(0, 5).map(item => { const recommended = item.id === meeting.package!.recommendedId; return <label key={item.id} className={`meeting-proposal${recommended ? ' recommended' : ''}${selected.includes(item.id) ? ' chosen' : ''}`}><input type="checkbox" checked={selected.includes(item.id)} disabled={meeting.stage !== 'package_ready'} onChange={() => setSelected(current => current.includes(item.id) ? current.filter(id => id !== item.id) : [...current, item.id])} /><span>{recommended && <b className="meeting-recommended">★ 권장</b>}{item.text}</span></label>; })}</div>
      {issues.length > 0 ? <p className="meeting-issues-chip"><span>아직 의견이 갈리는 점 {issues.length}개</span><button type="button" className="text-button" onClick={() => setDepth({ level: 2 })}>쟁점 보기</button></p> : <p className="meeting-issues-chip"><span>남은 반대 의견이 없어요</span></p>}
    </section>}
      {meeting.stage === 'package_ready' && <section className="card v2-stack" aria-label="내 결정">
        <p>골라도 바로 실행되지 않아요. 고른 제안은 홈의 <strong>고칠 수 있는 목표 초안</strong>으로만 들어가고, 직접 보내야 일이 시작돼요.</p>
        <label>메모 (선택)<textarea value={note} onChange={event => setNote(event.target.value)} /></label>
        <div className="button-row">
          <button className="primary" disabled={pending || selected.length === 0} onClick={() => void request(async () => { const draft = await props.onDecision(meeting.id, 'adopt', selected, note); if (draft && meeting.package) props.onDraft(draft, meeting.id, meeting.package.sha256); })}>고른 제안으로 결정 → 초안 만들기</button>
          <button className="secondary" disabled={pending} onClick={() => void request(() => props.onDecision(meeting.id, 'revise', [], note))}>다시 논의해 달라고 기록</button>
          <button className="secondary" disabled={pending} onClick={() => void request(() => props.onDecision(meeting.id, 'defer', [], note))}>나중에 정하기(보류)</button>
        </div>
      </section>}
    {meeting.decision && <p role="status">CEO 결정: {meeting.decision.kind === 'adopt' ? `채택 (${meeting.decision.recommendationIds.join(', ')})` : meeting.decision.kind === 'revise' ? '수정 요청' : '보류'} · {new Date(meeting.decision.decidedAt).toLocaleString()}{meeting.decision.note ? ` · ${meeting.decision.note}` : ''}</p>}
    <div className="button-row">{contributions.length > 0 && <button type="button" className="secondary meeting-deeper" onClick={() => setDepth({ level: 2 })}>회의 요약 보기 →</button>}{props.onArchive && <ArchiveGoalButton goal={goal} tasks={[]} meeting={meeting} onArchive={props.onArchive} />}</div>
    </>}

    {depth.level === 2 && <section className="meeting-flow" aria-label="회의 요약">
      <ol className="meeting-flow-stages">{digests.map(digest => <li key={digest.stage} className={digest.done ? 'done' : digest.current ? 'current' : ''}>
        <div className="v2-row"><strong>{DEBATE_STAGES.indexOf(digest.stage) + 1}. {digest.label}</strong><small>{digest.done ? '끝남' : digest.current ? '진행 중' : '아직'}</small></div>
        {digest.lines.length === 0 ? <small>아직 기록이 없어요</small> : digest.lines.map((line, index) => <p key={index}><b>{line.who}</b> {line.text}{line.ai && <small className="summary-badge ai">AI 요약</small>}</p>)}
        {digest.lines.length > 0 && <button type="button" className="text-button" onClick={() => setDepth({ level: 3, view: 'stage', stage: digest.stage })}>이 단계 자세히 →</button>}
      </li>)}</ol>
      <h3>쟁점 {issues.length}개</h3>
      {issues.length === 0 ? <p>남은 반대 의견이 없어요.</p> : <ul className="meeting-issue-list">{issues.map((issue, index) => <li key={index}><span className={`meeting-issue-status ${issue.status === '남음' ? 'open' : 'check'}`}>{issue.status}</span><span>{issue.text}{issue.ai && <small className="summary-badge ai">AI 요약</small>}</span><small>관련 의견 {issue.items.length}개</small><button type="button" className="text-button" onClick={() => setDepth({ level: 3, view: 'issue', issue: index })}>자세히 →</button></li>)}</ul>}
      <button type="button" className="secondary" onClick={() => setDepth({ level: 3, view: 'all' })}>모든 기록 보기 (원문·근거·전송 내역)</button>
    </section>}

    {depth.level === 3 && depth.view === 'stage' && <section className="meeting-detail" aria-label="단계 자세히">
      {contributions.filter(item => item.stage === depth.stage).map(item => <div key={item.id} className="card"><strong>{nameOf(item.slotId)}</strong>{depth.stage === 'planning' ? <pre className="meeting-payload">{JSON.stringify(item.content, null, 2)}</pre> : <Contribution item={item} />}</div>)}
    </section>}

    {depth.level === 3 && depth.view === 'issue' && issues[depth.issue] && <section className="meeting-detail" aria-label="쟁점 자세히">
      <h3>{issues[depth.issue].text}</h3>
      {issues[depth.issue].items.length === 0 && <p>이 쟁점에 연결된 개별 반대 의견이 없어요. 심사 담당의 정리만 있어요.</p>}
      {issues[depth.issue].items.map(item => <div key={item.critiqueId} className="meeting-claim"><strong>{nameOf(item.criticSlotId)}이(가) 짚은 점</strong><span>{item.text}</span><small>대상 생각: {meeting.package?.claims.find(claim => claim.id === item.targetClaimId)?.text ?? item.targetClaimId}</small><small>답: {plainDisposition[item.disposition] ?? item.disposition}{item.response ? ` — ${item.response}` : ''}</small><small className="meeting-ids">{item.critiqueId} → {item.targetClaimId}</small></div>)}
    </section>}

    {depth.level === 3 && depth.view === 'all' && <>
      {meeting.package && <>
      <PackageView pkg={meeting.package} meeting={meeting} selected={selected} onToggle={id => setSelected(current => current.includes(id) ? current.filter(item => item !== id) : [...current, id])} />
      {(meeting.package.trace ?? []).length > 0 && <TraceExplorer trace={meeting.package.trace!} nameOf={nameOf} />}
      </>}
    <details><summary>회의 원기록 · 발언 {contributions.length}개</summary>{['planning', ...STAGES].map(stage => { const items = contributions.filter(item => item.stage === stage); return items.length ? <div key={stage}><h4>{meetingStageLabels[stage]}</h4>{items.map(item => <div key={item.id}><strong>{nameOf(item.slotId)}</strong>{stage === 'planning' ? <pre className="meeting-payload">{JSON.stringify(item.content, null, 2)}</pre> : <Contribution item={item} />}</div>)}</div> : null; })}</details>
    <details><summary>전송 기록 · {dispatched.length}건</summary>{dispatched.length === 0 ? <p>아직 제공자에게 보낸 패킷이 없습니다.</p> : <table className="meeting-log"><thead><tr><th>시작</th><th>단계·팀원</th><th>수신 연결</th><th>SHA-256</th><th>결과</th><th>시간</th><th>토큰</th></tr></thead><tbody>{dispatched.map(packet => <tr key={packet.id}><td>{new Date(packet.dispatch!.startedAt).toLocaleTimeString()}</td><td>{meetingStageLabels[packet.stage] ?? packet.stage} · {nameOf(packet.slotId)}</td><td>{packet.dispatch!.connectionName} · {packet.dispatch!.modelId || '기본'} · {profileLabel[packet.dispatch!.isolationPolicy] ?? packet.dispatch!.isolationPolicy}</td><td>{packet.payloadSha256.slice(0, 12)}…</td><td>{packet.dispatch!.outcome}{packet.dispatch!.error ? ` — ${packet.dispatch!.error}` : ''}</td><td>{packet.dispatch!.durationMs ?? '?'}ms · {packet.dispatch!.commandChars.toLocaleString()}자</td><td>{packet.dispatch!.inputTokens}/{packet.dispatch!.outputTokens} ({packet.dispatch!.usageSource || '미확정'})</td></tr>)}</tbody></table>}</details>
    <details><summary>참여 팀원과 자료</summary>{meeting.slots.map(slot => <p key={slot.id}>{slot.kind === 'chair' ? '의장' : slot.kind === 'evaluator' ? '평가자' : slot.kind === 'participant' ? '참여자' : '관점'} · {slotLine(slot.id)}{slot.viewpoint && slot.kind === 'participant' ? ` · 입장: ${slot.viewpoint}` : ''}</p>)}{meeting.sources.length ? meeting.sources.map(source => <p key={source.id}>{source.id} {source.path} · SHA-256 {source.sha256.slice(0, 16)}…</p>) : <p>선택한 자료 없음</p>}</details>
    </>}
    {error && <p className="meeting-error" role="alert">{error}</p>}
  </section></div>;
}

export type MeetingHandlers = Handlers;
