import { describe, expect, it } from 'vitest';
import { meetingBackendOutdated } from './backendProtocol';

describe('meeting backend protocol gate', () => {
  it('blocks protocol 22 and absent backend information', () => {
    expect(meetingBackendOutdated(22)).toBe(true);
    expect(meetingBackendOutdated(undefined)).toBe(true);
  });
  it('requires the CEO pause and cancel commands', () => {
    expect(meetingBackendOutdated(42)).toBe(true);
    expect(meetingBackendOutdated(43)).toBe(false);
  });
});
