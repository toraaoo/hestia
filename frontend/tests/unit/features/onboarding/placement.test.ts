import { describe, expect, it } from 'vitest';

import { placeCard } from '@/features/onboarding/tour/placement';

const VIEWPORT = { width: 1000, height: 800 };
const CARD = { width: 300, height: 160 };

describe('placeCard', () => {
  it('centres a step with no target', () => {
    const placement = placeCard(null, CARD, VIEWPORT);
    expect(placement).toEqual({ top: 320, left: 350, side: 'center' });
  });

  it('prefers below the target', () => {
    const target = { top: 100, left: 400, width: 120, height: 40 };
    const { side, top } = placeCard(target, CARD, VIEWPORT);
    expect(side).toBe('bottom');
    expect(top).toBeGreaterThan(target.top + target.height);
  });

  it('flips above when there is no room below', () => {
    const target = { top: 700, left: 400, width: 120, height: 40 };
    const { side, top } = placeCard(target, CARD, VIEWPORT);
    expect(side).toBe('top');
    expect(top + CARD.height).toBeLessThan(target.top);
  });

  it('goes beside a target that spans the height', () => {
    const target = { top: 0, left: 0, width: 200, height: 800 };
    expect(placeCard(target, CARD, VIEWPORT).side).toBe('right');
  });

  it('keeps the card inside the viewport', () => {
    const target = { top: 20, left: 960, width: 30, height: 30 };
    const { top, left } = placeCard(target, CARD, VIEWPORT);
    expect(left).toBeGreaterThanOrEqual(0);
    expect(left + CARD.width).toBeLessThanOrEqual(VIEWPORT.width);
    expect(top).toBeGreaterThanOrEqual(0);
    expect(top + CARD.height).toBeLessThanOrEqual(VIEWPORT.height);
  });
});
