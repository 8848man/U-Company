import { describe, expect, it } from 'vitest';
import { meetingBackendOutdated } from './backendProtocol';

describe('meeting backend protocol gate', () => {
  it('blocks protocol 22 and absent backend information', () => {
    expect(meetingBackendOutdated(22)).toBe(true);
    expect(meetingBackendOutdated(undefined)).toBe(true);
  });
  it('allows protocol 32 or later', () => {
    expect(meetingBackendOutdated(31)).toBe(true);
    expect(meetingBackendOutdated(32)).toBe(false);
  });
});
