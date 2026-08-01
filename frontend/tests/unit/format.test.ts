import { describe, expect, it } from 'vitest';

import { memGb } from '../src/lib/format';

describe('memGb', () => {
  it('reads the unit rather than assuming gigabytes', () => {
    expect(memGb('8G')).toBe(8);
    expect(memGb('8192M')).toBe(8);
    expect(memGb('8388608K')).toBe(8);
    expect(memGb('4g')).toBe(4);
  });

  it('rounds to the gigabyte the surfaces offer, never below one', () => {
    expect(memGb('6656M')).toBe(7);
    expect(memGb('512M')).toBe(1);
  });

  it('falls back to 4 for anything unparseable', () => {
    expect(memGb('')).toBe(4);
    expect(memGb('8')).toBe(4);
    expect(memGb('4GB')).toBe(4);
  });
});
