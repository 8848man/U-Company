import { describe, expect, it } from 'vitest';
import { displayPath } from './paths';

describe('displayPath', () => {
  it('hides Windows verbatim prefixes without changing the underlying path', () => {
    expect(displayPath('\\\\?\\D:\\projects\\샘플')).toBe('D:\\projects\\샘플');
    expect(displayPath('\\\\?\\UNC\\server\\share\\팀')).toBe('\\\\server\\share\\팀');
    expect(displayPath('/home/team/project')).toBe('/home/team/project');
  });
});
