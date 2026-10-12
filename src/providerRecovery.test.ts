import { describe, expect, it } from 'vitest';
import { canOpenProviderLogin, connectionRecovery, runtimeRecovery } from './providerRecovery';
import type { AiConnection } from './types';

const claude: AiConnection = { id: 'claude', name: 'Claude', kind: 'cli', executable: 'C:\\claude.cmd', endpoint: '', args: [], modelId: '', status: 'needs_setup', capabilities: [] };

describe('provider recovery', () => {
  it('offers login only for a saved built-in CLI', () => {
    expect(canOpenProviderLogin(claude)).toBe(true);
    expect(canOpenProviderLogin({ ...claude, id: 'custom' })).toBe(false);
    expect(canOpenProviderLogin({ ...claude, executable: '' })).toBe(false);
    expect(connectionRecovery(claude)).toContain('Claude 로그인 창');
  });

  it('classifies common runtime errors without using error text as an action', () => {
    expect(runtimeRecovery('Blocked: Provider=claude Runtime=ProcessRuntime Operation=execute: authentication required')).toContain('Claude 로그인 창');
    expect(runtimeRecovery('Provider=codex: rate limit 429')).toContain('요청 한도');
    expect(runtimeRecovery('Provider=gemini: model unavailable')).toContain('모델');
    expect(runtimeRecovery('Provider=opencode: connection refused')).toContain('네트워크');
    expect(runtimeRecovery('Provider=claude: organization disabled')).toContain('ANTHROPIC_API_KEY');
    expect(runtimeRecovery('Provider=gemini: named user entitlement')).toContain('GOOGLE_CLOUD_PROJECT');
    expect(runtimeRecovery('Provider=opencode: provider not found')).toContain('AI 제공자');
    expect(runtimeRecovery('Provider=custom: login required')).toBeNull();
  });
});
