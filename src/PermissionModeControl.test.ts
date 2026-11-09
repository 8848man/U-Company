import { describe, expect, it } from 'vitest';
import { effectivePermissionMode } from './PermissionModeControl';

describe('permission mode selection', () => {
  it('uses the project default and a conversation override', () => {
    const settings = [
      { scope: 'project' as const, id: 'p', mode: 'auto' as const, updatedAt: 't' },
      { scope: 'conversation' as const, id: 'g', mode: 'full_auto' as const, updatedAt: 't' },
    ];
    expect(effectivePermissionMode(settings, 'p')).toEqual({ mode: 'auto', override: 'inherit' });
    expect(effectivePermissionMode(settings, 'p', 'g')).toEqual({ mode: 'full_auto', override: 'full_auto' });
    expect(effectivePermissionMode([], 'p', 'g')).toEqual({ mode: 'manual', override: 'inherit' });
  });
});
