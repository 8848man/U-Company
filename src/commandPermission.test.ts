import { describe, expect, it } from 'vitest';
import { approveAllKeys, groupRules, planCommandLine, ruleKey, ruleLabel } from './commandPermission';
import type { CommandRequest } from './types';

const request = (source: string): CommandRequest => ({ id: 'a', goalId: 'g', conversationId: 'g', source, granted: [], taskIds: [], createdAt: 't', rules: [
  { tool: 'Bash', prefix: 'node --test', class: 'safe', reason: '' },
  { tool: 'Bash', prefix: 'npm install', class: 'individual', reason: '설치' },
  { tool: 'WebFetch', prefix: '', class: 'individual', reason: '인터넷' },
  { tool: 'Write', prefix: '', class: 'blocked', reason: '밖' },
] });

describe('command permission card (ADR-0058)', () => {
  it('uses the backend keys and plain labels', () => {
    const rules = request('blocked').rules;
    expect(rules.map(ruleKey)).toEqual(['Bash(node --test)', 'Bash(npm install)', 'WebFetch', 'Write']);
    expect(ruleLabel(rules[2])).toBe('인터넷 접속 (WebFetch)');
  });
  it('approve-all sends safe rules plus only the individual rules the CEO ticked, never blocked ones', () => {
    expect(approveAllKeys(request('blocked'), [])).toEqual(['Bash(node --test)']);
    expect(approveAllKeys(request('blocked'), ['Bash(npm install)', 'Write'])).toEqual(['Bash(node --test)', 'Bash(npm install)']);
    expect(groupRules(request('blocked')).blocked.map(ruleKey)).toEqual(['Write']);
  });
  it('summarises plan commands in one line', () => {
    expect(planCommandLine(request('plan'))).toBe('승인하면 이 대화에서 실행 허용: node --test · 필요할 때 따로 물어요: npm install, 인터넷 접속 (WebFetch), Write');
    expect(planCommandLine(request('blocked'))).toBe('');
    expect(planCommandLine(undefined)).toBe('');
  });
});
