import { describe, expect, it } from 'vitest';

// Windows and macOS file systems ignore case, so `GoalChat.tsx` and `goalChat.ts` resolve to the same
// extensionless import there and the app breaks only on those machines. Keep stems unique ignoring case.
const files = Object.keys({
  ...import.meta.glob('./**/*.{ts,tsx,css}'),
  ...import.meta.glob('../v2/src/**/*.{ts,tsx,css}'),
  ...import.meta.glob('../v3/src/**/*.{ts,tsx,css}'),
});

describe('source file names', () => {
  it('never differ only by letter case within a folder', () => {
    const stems = new Map<string, Set<string>>();
    for (const path of files) {
      const slash = path.lastIndexOf('/');
      const key = `${path.slice(0, slash).toLowerCase()}/${path.slice(slash + 1).split('.')[0].toLowerCase()}`;
      if (!stems.has(key)) stems.set(key, new Set());
      stems.get(key)!.add(path.slice(0, slash + 1) + path.slice(slash + 1).split('.')[0]);
    }
    expect(files.length).toBeGreaterThan(50);
    expect([...stems.values()].filter(set => set.size > 1).map(set => [...set].join(' / '))).toEqual([]);
  });
});
