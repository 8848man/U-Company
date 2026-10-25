import type { CommandRequest, CommandRule } from './types';

/** ADR-0058: the same key the backend uses (`Bash(npm test)`, or the tool alone). */
export function ruleKey(rule: CommandRule): string {
  return rule.prefix ? `${rule.tool}(${rule.prefix})` : rule.tool;
}

/** What the CEO reads: the command itself, or the tool name for non-command tools. */
export function ruleLabel(rule: CommandRule): string {
  if (rule.prefix) return rule.prefix;
  return rule.tool === 'WebFetch' || rule.tool === 'WebSearch' ? `인터넷 접속 (${rule.tool})` : rule.tool;
}

export function groupRules(request: CommandRequest) {
  return {
    safe: request.rules.filter(rule => rule.class === 'safe'),
    individual: request.rules.filter(rule => rule.class === 'individual'),
    blocked: request.rules.filter(rule => rule.class !== 'safe' && rule.class !== 'individual'),
  };
}

/** Keys sent for `모두 승인`: every safe rule plus the individual ones the CEO ticked. */
export function approveAllKeys(request: CommandRequest, chosenIndividual: string[]): string[] {
  const { safe, individual } = groupRules(request);
  return [...safe.map(ruleKey), ...individual.map(ruleKey).filter(key => chosenIndividual.includes(key))];
}

/** One line for a plan card: what approving the plan allows, and what will be asked later. */
export function planCommandLine(request: CommandRequest | undefined): string {
  if (!request || request.source !== 'plan') return '';
  const { safe, individual, blocked } = groupRules(request);
  const parts = [];
  if (safe.length) parts.push(`승인하면 이 대화에서 실행 허용: ${safe.map(ruleLabel).join(', ')}`);
  const later = [...individual, ...blocked];
  if (later.length) parts.push(`필요할 때 따로 물어요: ${later.map(ruleLabel).join(', ')}`);
  return parts.join(' · ');
}
