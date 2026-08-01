import { describe, expect, it } from 'vitest';

import { supportsQuickPlay } from '@/lib/quick-play';

describe('supportsQuickPlay', () => {
  it('admits 1.20 and everything after it', () => {
    expect(supportsQuickPlay('1.20')).toBe(true);
    expect(supportsQuickPlay('1.20.1')).toBe(true);
    expect(supportsQuickPlay('1.21.1')).toBe(true);
    expect(supportsQuickPlay('2.0')).toBe(true);
  });

  it('refuses versions older than the arguments themselves', () => {
    expect(supportsQuickPlay('1.19.4')).toBe(false);
    expect(supportsQuickPlay('1.7.10')).toBe(false);
  });

  it('refuses anything it cannot place, as the daemon does', () => {
    expect(supportsQuickPlay('23w14a')).toBe(false);
    expect(supportsQuickPlay('')).toBe(false);
    expect(supportsQuickPlay('1')).toBe(false);
  });
});
