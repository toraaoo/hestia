import { describe, expect, it } from 'vitest';
import type { ProvisionPhase, ProvisionProgress } from '@/api';
import {
  isMeasurable,
  overallRatio,
  phaseLabel,
} from '@/features/shared/entry/components';

const progress = (over: Partial<ProvisionProgress> = {}): ProvisionProgress =>
  ({ phase: 'client', current: 0, total: 0, ...over }) as ProvisionProgress;

describe('overallRatio', () => {
  it('is the unit fraction for a single-unit phase', () => {
    expect(overallRatio(progress({ current: 50, total: 200 }))).toBe(0.25);
  });

  it('is zero when the extent is unknown', () => {
    expect(overallRatio(progress({ current: 100, total: 0 }))).toBe(0);
  });

  it('never exceeds one, even if the daemon overshoots', () => {
    expect(overallRatio(progress({ current: 300, total: 200 }))).toBe(1);
  });

  it('advances across a batch rather than resetting per unit', () => {
    const atUnitStart = overallRatio(
      progress({ item: 3, items: 4, current: 0, total: 100 }),
    );
    const midUnit = overallRatio(
      progress({ item: 3, items: 4, current: 50, total: 100 }),
    );
    expect(atUnitStart).toBe(0.5);
    expect(midUnit).toBe(0.625);
    expect(midUnit).toBeGreaterThan(atUnitStart);
  });

  it('advances on a cached unit that reports no bytes', () => {
    expect(
      overallRatio(progress({ item: 2, items: 4, current: 0, total: 0 })),
    ).toBe(0.25);
  });

  it('is complete on the last unit of a batch', () => {
    expect(
      overallRatio(progress({ item: 4, items: 4, current: 100, total: 100 })),
    ).toBe(1);
  });
});

describe('isMeasurable', () => {
  it('is true once there is an extent to fill', () => {
    expect(isMeasurable(progress({ total: 100 }))).toBe(true);
    expect(isMeasurable(progress({ items: 3 }))).toBe(true);
  });

  it('is false when the step has no known extent', () => {
    expect(isMeasurable(progress())).toBe(false);
  });
});

describe('phaseLabel', () => {
  const phases: ProvisionPhase[] = [
    'resolving',
    'backup',
    'java',
    'server',
    'client',
    'libraries',
    'assets',
    'content',
    'overrides',
    'archive',
    'extract',
  ];

  it('has a written label for every phase the wire defines', () => {
    for (const phase of phases) {
      const label = phaseLabel(phase);
      expect(label).not.toBe('');
      expect(label).not.toContain('{');
    }
  });

  it('falls back to the raw id for a phase it has not met', () => {
    expect(phaseLabel('brand_new' as ProvisionPhase)).toBe('brand_new');
  });
});
