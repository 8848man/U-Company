import type { Agent, Goal, Meeting, MeetingPacket, Snapshot } from './types';

export const meetingStageLabels: Record<string, string> = {
  planning: '계획 세우기', plan_review: '계획 확인', plan_approved: '계획 승인됨',
  proposing: '각자 생각 내기', critiquing: '약한 곳 짚기', revising: '고치기', evaluating: '심사하기',
};

export type MeetingAttention = { label: string; detail: string; next: string; needsDecision: boolean; active: boolean };

/** Current packets of the stage being decided: the latest non-superseded revision per slot, in plan order. */
export function currentStagePackets(meeting: Meeting, packets: MeetingPacket[]): MeetingPacket[] {
  const stage = meeting.activeStage ?? '';
  const order = meeting.slots.map(slot => slot.id);
  return packets.filter(packet => packet.meetingId === meeting.id && packet.stage === stage && !packet.status.endsWith('superseded')).sort((a, b) => order.indexOf(a.slotId) - order.indexOf(b.slotId));
}

/** Office member names of a meeting by role in the plan, plus who is being called right now. */
export function meetingMembers(meeting: Meeting | undefined, packets: MeetingPacket[] = [], agents: Pick<Agent, 'id' | 'name'>[] = []): { chair: string; participants: string[]; evaluator: string; calling: string[] } {
  if (!meeting) return { chair: '', participants: [], evaluator: '', calling: [] };
  const name = (id: string) => meeting.slots.find(slot => slot.id === id)?.agentName || agents.find(agent => agent.id === id)?.name || id;
  const chair = meeting.slots.find(slot => slot.kind === 'chair');
  const participants = meeting.plan ? meeting.plan.participants.map(member => name(member.agentId)) : meeting.slots.filter(slot => slot.kind === 'participant').map(slot => slot.agentName || slot.id);
  const evaluator = meeting.plan?.evaluatorAgentId ? name(meeting.plan.evaluatorAgentId) : '';
  const calling = packets.filter(packet => packet.meetingId === meeting.id && packet.status === 'dispatching').map(packet => name(packet.slotId));
  return { chair: chair?.agentName || chair?.id || '', participants, evaluator, calling };
}

/** One shared interpretation of a meeting Goal for Home, Work, Office, open work and the workspace. */
export function meetingAttention(_goal: Goal, meeting: Meeting | undefined, packets: MeetingPacket[] = [], agents: Pick<Agent, 'id' | 'name'>[] = []): MeetingAttention {
  if (!meeting) return { label: '회의 기록 확인 필요', detail: '이 목표에 연결된 회의 기록을 찾지 못했습니다.', next: '회의 열기', needsDecision: true, active: false };
  const stage = meetingStageLabels[meeting.activeStage ?? ''] ?? '';
  const members = meetingMembers(meeting, packets, agents);
  switch (meeting.stage) {
    case 'planning': return { label: '의장이 회의 계획을 세우는 중', detail: `${members.chair || '의장'}이(가) 무엇을 정할지, 누가 참여할지 정하고 있어요.`, next: '기다리기', needsDecision: false, active: true };
    case 'plan_review': {
      const errors = meeting.plan?.errors.length ?? 0;
      return { label: '회의 계획 확인 필요', detail: errors ? `계획에 고칠 점 ${errors}개가 있어요.` : `참여 ${members.participants.join(', ') || '-'} · 심사 ${members.evaluator || '-'} · AI에게 최대 ${meeting.plan?.maxCalls ?? '-'}번 물어봐요`, next: errors ? '계획 고치기' : '계획 승인 또는 고치기', needsDecision: true, active: false };
    }
    case 'plan_approved': return { label: '계획 승인됨', detail: '첫 단계에 보낼 내용을 준비하고 있어요.', next: '기다리기', needsDecision: false, active: true };
    case 'awaiting_transfer_approval': {
      const current = currentStagePackets(meeting, packets);
      const pending = current.filter(packet => packet.status === 'pending').length;
      return pending > 0
        ? { label: '보낼 내용 확인 필요', detail: `${stage} 단계에서 AI에게 보낼 내용 ${pending}개를 확인해 주세요.`, next: `보낼 내용 ${pending}개 확인`, needsDecision: true, active: false }
        : { label: '보내기 대기', detail: `${stage} 단계에서 보낼 내용을 모두 확인했어요.`, next: '확인한 내용 보내기', needsDecision: true, active: false };
    }
    case 'proposing': case 'critiquing': case 'revising': case 'evaluating':
      return { label: `진행 중 · ${meetingStageLabels[meeting.stage]}`, detail: `${members.calling.length ? `${members.calling.join(', ')} 응답 대기 · ` : ''}AI에게 물어본 횟수 ${meeting.callsUsed}/${meeting.maxCalls}`, next: '기다리기', needsDecision: false, active: true };
    case 'blocked': return { label: '회의가 멈췄어요', detail: meeting.blockReason || '회의가 멈췄습니다.', next: '다시 시작하거나 지금까지 결과로 마치기', needsDecision: true, active: false };
    case 'stopped': return { label: '중단됨', detail: meeting.blockReason || 'CEO가 회의를 중단했습니다.', next: '다시 시작하거나 지금까지 결과로 마치기', needsDecision: true, active: false };
    case 'package_ready': return { label: '회의가 끝났어요 · 결정해 주세요', detail: meeting.package?.partial ? '일부만 끝났어요.' : '한 바퀴 토론을 마쳤어요.', next: '제안 고르기, 다시 논의, 보류 중 하나', needsDecision: true, active: false };
    case 'decision_recorded': return { label: '결정 기록됨', detail: meeting.decision?.kind === 'adopt' ? '고른 제안은 목표 초안으로만 넘어가요.' : '결정을 기록했어요.', next: '회의 결과 보기', needsDecision: false, active: false };
    case 'legacy_closed': return { label: '이전 방식 회의 · 읽기 전용', detail: '앱 업데이트 전 고정 관점 방식의 회의라 더 진행하지 않습니다.', next: '기록 보기', needsDecision: false, active: false };
    default: return { label: '상태 확인 필요', detail: meeting.stage, next: '회의 열기', needsDecision: true, active: false };
  }
}

export function meetingAttentionFor(goal: Goal, snapshot: Pick<Snapshot, 'meetings' | 'meetingPackets'> & { agents?: Agent[] }): MeetingAttention {
  return meetingAttention(goal, (snapshot.meetings ?? []).find(item => item.goalId === goal.id), snapshot.meetingPackets ?? [], snapshot.agents ?? []);
}
