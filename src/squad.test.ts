import { describe, expect, it } from 'vitest';
import { officeBadges, openSlots, squadSummary, swapCandidates } from './squad';
import type { Agent, AiConnection, Goal, Squad } from './types';

const agent = (id: string, role: string, extra: Partial<Agent> = {}): Agent => ({ id, name: id.toUpperCase(), role, instructions: '', reportsTo: null, connectionId: 'cli', modelId: '', x: 0, y: 0, enabled: true, ...extra });
const connections: AiConnection[] = [{ id: 'cli', name: 'Claude', kind: 'cli', executable: 'claude', endpoint: '', args: [], modelId: '', status: 'ready', capabilities: [] }, { id: 'http', name: 'Local', kind: 'http', executable: '', endpoint: 'http://x', args: [], modelId: '', status: 'ready', capabilities: [] }];
const external = (id: string, slot: 'writer' | 'reviewer', status: 'standby' | 'contracted' | 'released' | 'hired' = 'standby') => agent(id, slot === 'writer' ? 'implementation (external)' : 'validation and independent review (external)', { external: { status, goalId: status === 'contracted' ? 'g1' : null, slot, title: '', author: '', capabilities: ['document'], sourceName: 'a.json', sourceSha256: 'x', importedAt: 't', releasedAt: null } });
const agents = [agent('cto', 'architecture'), agent('pm', 'planning'), agent('developer-a', 'implementation'), agent('developer-b', 'implementation'), agent('developer-c', 'implementation', { connectionId: 'http' }), agent('qa', 'validation and independent review'), agent('qa2', 'validation and independent review'), external('ext-w', 'writer'), external('ext-r', 'reviewer'), external('ext-old', 'writer', 'released')];
const squad: Squad = { goalId: 'g1', structure: 'standard', structureReason: '문서 2개', members: [{ agentId: 'cto', slot: 'lead', duty: 'analysis', reason: '' }, { agentId: 'pm', slot: 'lead', duty: 'planning', reason: '' }, { agentId: 'developer-a', slot: 'writer', duty: '', reason: '' }, { agentId: 'qa', slot: 'reviewer', duty: '', reason: '' }], status: 'proposed', shortfall: [], approvalId: null, changes: [], createdAt: 't', lockedAt: null, releasedAt: null, digest: 'd' };
const goal: Goal = { id: 'g1', projectId: 'p', intent: '출시 준비 문서 세 개 만들기', status: 'awaiting_approval', report: '', createdAt: 't', resumedFrom: null, reviewLoopsUsed: 0, reviewLoopLimit: null, workGroup: null };

describe('goal squads (ADR-0041)', () => {
  it('summarises slots with duties and external marks', () => {
    expect(squadSummary(squad, agents)).toBe('리드 CTO(분석), PM(계획) · 작성 DEVELOPER-A · 검토 QA');
  });
  it('offers same-slot, usable, standby candidates only', () => {
    expect(swapCandidates(squad, 'writer', agents, connections).map(item => item.id)).toEqual(['developer-b', 'ext-w']);
    expect(swapCandidates(squad, 'reviewer', agents, connections).map(item => item.id)).toEqual(['qa2', 'ext-r']);
    expect(swapCandidates(squad, 'lead', agents, connections)).toEqual([]);
  });
  it('badges squad members and external staff from saved records', () => {
    const badges = officeBadges([squad], [goal], agents);
    expect(badges['developer-a']).toBe('작성 · 출시 준비 문서 세 개…');
    expect(badges['ext-w']).toBe('외부 · 대기');
    expect(badges['developer-b']).toBeUndefined();
    expect(officeBadges([squad], [{ ...goal, status: 'completed' }], agents)['qa']).toBeUndefined();
  });
});

describe('open squad slots', () => {
  it('counts empty places and only offers tasks from writers holding two or more', () => {
    const task = (id: string, assigneeId: string) => ({ id, goalId: 'g1', title: id, instructions: '', files: [], assigneeId, status: 'pending', dependsOn: [], worktree: null, result: '', sessionId: null });
    const launch = { ...squad, structure: 'launch_prep' };
    const open = openSlots(launch, [task('t1', 'developer-a'), task('t2', 'developer-a'), task('t3', 'developer-a')] as never);
    expect(open.writer).toBe(2);
    expect(open.reviewer).toBe(1);
    expect(open.movable.map(item => item.id)).toEqual(['t1', 't2', 't3']);
    expect(openSlots(squad, [task('t1', 'developer-a')] as never)).toEqual({ writer: 0, reviewer: 0, movable: [] });
  });
});
