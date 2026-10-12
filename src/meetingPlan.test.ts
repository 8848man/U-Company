import { describe, expect, it } from 'vitest';
import { inputOfDraft, planInputOf, requiredCalls } from './MeetingPanel';
import type { MeetingPlan } from './types';

const plan: MeetingPlan = { revision: 1, digest: 'd', objective: 'Pick the onboarding fix', constraints: ['No schema change'], participants: [{ agentId: 'cto', stance: 'engine risk' }, { agentId: 'dev-a', stance: 'UX' }], evaluatorAgentId: 'qa', protocol: 'propose_critique_revise', rounds: 1, maxCalls: 8, sources: [{ path: 'sdd/product.md', reason: 'scope' }], successCriteria: ['one recommendation'], origin: 'chair', errors: [], createdAt: '' };

describe('meeting plan editing', () => {
  it('counts the chair call, three calls per participant and one evaluation, within the hard cap of 12', () => {
    expect(requiredCalls(2)).toBe(8);
    expect(requiredCalls(3)).toBe(11);
  });
  it('round-trips a saved plan and parses `path | reason` source lines without inventing members', () => {
    const draft = { objective: ' Pick the onboarding fix ', constraints: 'No schema change\n\n', participants: [{ agentId: 'cto', stance: ' engine risk ' }, { agentId: 'dev-a', stance: 'UX' }, { agentId: '', stance: 'ignored' }], evaluatorAgentId: 'qa', maxCalls: 8, sources: 'sdd/product.md | scope\n', successCriteria: 'one recommendation' };
    expect(inputOfDraft(draft)).toEqual(planInputOf(plan));
  });
});
