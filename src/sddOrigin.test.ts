import { describe, expect, it } from 'vitest';
import { PLANNING_CONTROL_LINE, answerLabel, changePreviews, connectionGap, originChoices, originMessage, planRows, planSummary, planningLead, type OriginStatus } from './sddOrigin';

const status = (patch: Partial<OriginStatus> = {}): OriginStatus => ({
  mode: 'bootstrap', scope: '.', evidence: [], followed: null, conflict: false, toolRules: [],
  actions: ['bootstrap'], nextTask: null, scopes: [], review: [], ...patch,
});

describe('originMessage', () => {
  it('offers to set up a structure without the jargon word on the first line', () => {
    const message = originMessage(status());
    expect(message.headline).toContain('구조를 잡아드릴까요');
    expect(message.headline).not.toContain('SDD');
    expect(message.detail).toContain('기존 파일은 바뀌지 않아요');
  });
  it('names the followed rule file and says AGENTS.md wins a conflict', () => {
    const message = originMessage(status({ mode: 'existing', evidence: ['AGENTS.md', 'CLAUDE.md'], followed: 'AGENTS.md', conflict: true, toolRules: ['CLAUDE.md'], actions: ['convert'] }));
    expect(message.headline).toContain('작업 규칙(AGENTS.md)');
    expect(message.notes[0]).toContain('더 범용인 AGENTS.md');
    expect(message.detail).toContain('.ucompany/legacy');
  });
  it('labels a tool-only rule file by its tool', () => {
    expect(originMessage(status({ mode: 'existing', evidence: ['CLAUDE.md'], followed: 'CLAUDE.md', actions: ['convert'] })).headline).toContain('Claude 전용 작업 규칙');
  });
  it('keeps the root untouched for rules found only in a subfolder', () => {
    const message = originMessage(status({ mode: 'existing', scope: 'packages/api', evidence: ['packages/api/AGENTS.md'], actions: [] }));
    expect(message.headline).toContain('packages/api');
    expect(message.detail).toContain('아무것도 만들지 않아요');
  });
  it('shows the next task while continuing', () => {
    expect(originMessage(status({ mode: 'continue', actions: [], nextTask: 'T-003 overdue 구현' })).detail).toBe('다음 작업: overdue 구현'); // no IDs on depth 1 (UXP-7)
  });
});

describe('originChoices', () => {
  it('recommends adding a section, never converting', () => {
    const choices = originChoices(status({ mode: 'ask', actions: ['add_section', 'convert'] }));
    expect(choices.map(choice => [choice.label, choice.primary])).toEqual([['절차 추가', true], ['U 컴퍼니 구조로 바꾸기', false]]);
  });
  it('offers converting without a recommendation when it is the only write', () => {
    expect(originChoices(status({ mode: 'existing', actions: ['convert'] }))).toEqual([{ action: 'convert', label: 'U 컴퍼니 구조로 바꾸기', primary: false }]);
  });
  it('offers nothing for a subfolder scope or an existing structure', () => {
    expect(originChoices(status({ mode: 'existing', scope: 'packages/api', actions: [] }))).toEqual([]);
    expect(originChoices(status({ mode: 'continue', actions: [] }))).toEqual([]);
  });
});

describe('plan display', () => {
  const plan = { action: 'convert' as const, digest: 'd', changes: [
    { path: 'AGENTS.md', change: 'replace' as const },
    { path: '.ucompany/sdd/00_index.md', change: 'create' as const },
    { path: '.ucompany/sdd/features/.gitkeep', change: 'create' as const },
    { path: '.ucompany/legacy/(적용 시각)/AGENTS.md', change: 'backup' as const },
  ] };
  it('counts changes in apply order', () => {
    expect(planSummary(plan)).toBe('백업 1 · 새로 만듦 2 · 바꿈 1');
  });
  it('lists backups first and hides placeholder files', () => {
    expect(planRows(plan).map(row => row.label)).toEqual(['백업', '새로 만듦', '바꿈']);
  });
});

describe('originMessage notes', () => {
  it('does not repeat a conflicting tool file as an extra found file', () => {
    const notes = originMessage(status({ mode: 'existing', evidence: ['AGENTS.md', 'CLAUDE.md', 'specs/'], followed: 'AGENTS.md', conflict: true, toolRules: ['CLAUDE.md'], actions: ['convert'] })).notes;
    expect(notes.filter(note => note.includes('CLAUDE.md'))).toHaveLength(1);
    expect(notes).toContain('함께 찾은 파일: specs/');
  });
});

describe('planning card (ADR-0045)', () => {
  const origin = { goalId: 'g', projectId: 'p', workKind: 'development', offered: true, mode: 'bootstrap', advice: null, decision: null, decidedAt: null, createdAt: 't' };
  it('leads with the CTO judgement and never shows IDs', () => {
    expect(planningLead(origin)).toContain('개발 작업');
    expect(planningLead({ ...origin, mode: 'ask' })).toContain('작업 규칙');
    expect(planningLead(origin)).not.toContain('?');
    expect(PLANNING_CONTROL_LINE).toContain('계획 승인은 따로');
  });
  it('turns a recorded answer into a plain sentence', () => {
    expect(answerLabel(null)).toBeNull();
    expect(answerLabel('applied:add_section')).toContain('절차 추가');
    expect(answerLabel('declined')).toContain('잡지 않기로');
    expect(answerLabel('muted')).toContain('설정 › 프로젝트 명세');
  });
  it('asks to connect an AI only when the squad has a shortfall', () => {
    expect(connectionGap([])).toBeNull();
    expect(connectionGap(undefined)).toBeNull();
    expect(connectionGap(['검토자 1명 중 0명만 가능합니다'])).toContain('AI를 연결할까요?');
  });
  it('compares before and after only for changed entry files', () => {
    const previews = changePreviews({ action: 'add_section', digest: 'd', changes: [
      { path: 'AGENTS.md', change: 'append', before: '# Style\n', after: '# Style\n<!-- U-SDD:START -->' },
      { path: '.ucompany/sdd/00_index.md', change: 'create' },
      { path: 'CLAUDE.md', change: 'create' },
    ] });
    expect(previews).toEqual([{ path: 'AGENTS.md', label: '끝에 덧붙임', before: '# Style\n', after: '# Style\n<!-- U-SDD:START -->' }]);
  });
});
