import type { SyncUnit, UnitState } from '@/api';
import { m } from '@/paraglide/messages.js';

export const SYNC_UNITS: SyncUnit[] = [
  'options',
  'servers',
  'commands',
  'hotbars',
];

export const unitLabel: Record<SyncUnit, () => string> = {
  options: () => m['domain.sync_unit.options.label'](),
  servers: () => m['domain.sync_unit.servers.label'](),
  commands: () => m['domain.sync_unit.commands.label'](),
  hotbars: () => m['domain.sync_unit.hotbars.label'](),
};

export const unitHint: Record<SyncUnit, () => string> = {
  options: () => m['domain.sync_unit.options.hint'](),
  servers: () => m['domain.sync_unit.servers.hint'](),
  commands: () => m['domain.sync_unit.commands.hint'](),
  hotbars: () => m['domain.sync_unit.hotbars.hint'](),
};

export const stateLabel: Record<UnitState, () => string> = {
  synced: () => m['domain.sync_state.synced'](),
  pending: () => m['domain.sync_state.pending'](),
  overridden: () => m['domain.sync_state.overridden'](),
  off: () => m['domain.sync_state.off'](),
  era_bound: () => m['domain.sync_state.era_bound'](),
};

export const stateTone: Record<UnitState, 'on' | 'off' | 'warn'> = {
  synced: 'on',
  pending: 'off',
  overridden: 'off',
  off: 'off',
  era_bound: 'warn',
};
