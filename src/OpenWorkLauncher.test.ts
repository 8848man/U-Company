import { describe, expect, it } from 'vitest';
import { clampLauncher } from './OpenWorkLauncher';

describe('open-work launcher placement', () => {
  it('keeps the dragged button inside the window', () => {
    expect(clampLauncher({ right: -50, bottom: 9999 }, 1200, 800)).toEqual({ right: 8, bottom: 800 - 46 - 8 });
    expect(clampLauncher({ right: 5000, bottom: 2 }, 1200, 800)).toEqual({ right: 1200 - 150 - 8, bottom: 8 });
    expect(clampLauncher({ right: 300.4, bottom: 200.6 }, 1200, 800)).toEqual({ right: 300, bottom: 201 });
  });
});
