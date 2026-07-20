import { describe, expect, it } from 'vitest';
import { invalidationKeys } from './invalidation';
import { FOOTPRINT, keys } from './keys';

/** True when any returned key sweeps the expensive footprint (disk-walk) query. */
function touchesFootprint(topic: string, payload: Record<string, unknown>) {
  return invalidationKeys(topic, payload).some((key) => key[0] === FOOTPRINT);
}

describe('lifecycle invalidation', () => {
  it('never sweeps the footprint walk on a process lifecycle event', () => {
    const server = { id: 'server-smp-3f9a2c7d' };
    expect(touchesFootprint('process.started', server)).toBe(false);
    expect(touchesFootprint('process.exit', server)).toBe(false);

    const session = { id: 'instance-cozy-1a2b3c4d_2' };
    expect(touchesFootprint('process.started', session)).toBe(false);
    expect(
      touchesFootprint('instance.launch.done', { processId: session.id }),
    ).toBe(false);
  });

  it('targets only the named entry, not the whole subtree', () => {
    const mapped = invalidationKeys('process.started', {
      id: 'instance-cozy-1a2b3c4d_2',
    });
    expect(mapped).toContainEqual(keys.instances.list());
    expect(mapped).toContainEqual(keys.instances.detail('cozy-1a2b3c4d'));
    // Never the kind-wide `all`, which would pull every entry's detail.
    expect(mapped).not.toContainEqual(keys.instances.all);
  });
});
