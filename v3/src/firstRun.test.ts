import { describe, expect, it } from 'vitest';
import { firstRunChecks, SAMPLE_APP_GOAL } from './V3Home';
import type { Agent, AiConnection, Project } from '../../src/types';

const project = (kind: string) => ({ id: 'p', name: '샘플', kind }) as Project;
const agent = (id: string, connectionId: string, enabled = true) => ({ id, name: id.toUpperCase(), connectionId, enabled }) as Agent;
const connection = (id: string, status: string) => ({ id, status }) as AiConnection;

describe('first-run check', () => {
  it('passes when every active teammate has a usable connection, and treats a plain folder as fine', () => {
    const checks = firstRunChecks(project('folder'), [agent('cto', 'mock'), agent('pm', 'mock'), agent('old', '', false)], [connection('mock', 'ready')]);
    expect(checks.map(check => [check.label, check.ok])).toEqual([['작업할 폴더', true], ['Git', false], ['AI 연결', true]]);
    expect(checks[1].detail).toContain('괜찮아요');
    expect(checks[2].action).toBeUndefined();
  });

  it('names teammates without a usable AI and offers the connection screen', () => {
    const checks = firstRunChecks(project('git'), [agent('cto', 'claude'), agent('qa', '')], [connection('claude', 'needs_setup')]);
    expect(checks[2]).toMatchObject({ ok: false, action: 'connections' });
    expect(checks[2].detail).toBe('팀원 2명이 쓸 AI가 없어요: CTO, QA');
  });

  it('offers a sample-app goal long enough to send', () => {
    expect(SAMPLE_APP_GOAL.length).toBeGreaterThan(10);
    expect(SAMPLE_APP_GOAL).toContain('node --test');
  });
});
