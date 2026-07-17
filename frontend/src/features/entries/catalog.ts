/**
 * Static flavor + version catalogue for the create wizards, shaped after the
 * daemon's `{server,instance}.flavors|versions` surface. Nothing talks to a
 * backend — the wizards mirror the real create flow over this stand-in.
 */

import { m } from '@/paraglide/messages.js';

export interface CatalogFlavor {
  id: string;
  name: string;
  summary: () => string;
}

export type VersionKind = 'release' | 'snapshot';

export interface CatalogVersion {
  id: string;
  kind: VersionKind;
}

export const flavors: CatalogFlavor[] = [
  { id: 'vanilla', name: 'Vanilla', summary: m['flavor.vanilla_summary'] },
  { id: 'fabric', name: 'Fabric', summary: m['flavor.fabric_summary'] },
];

/** Newest first, the way a provider's catalogue reports them. */
export const gameVersions: CatalogVersion[] = [
  { id: '1.21.4', kind: 'release' },
  { id: '25w03a', kind: 'snapshot' },
  { id: '1.21.3', kind: 'release' },
  { id: '1.21.1', kind: 'release' },
  { id: '1.21', kind: 'release' },
  { id: '24w45a', kind: 'snapshot' },
  { id: '1.20.6', kind: 'release' },
  { id: '1.20.4', kind: 'release' },
  { id: '1.20.1', kind: 'release' },
  { id: '1.19.4', kind: 'release' },
  { id: '1.18.2', kind: 'release' },
  { id: '1.16.5', kind: 'release' },
  { id: '1.12.2', kind: 'release' },
];

/** Fabric loader builds, newest first. */
export const loaderVersions = ['0.16.9', '0.16.5', '0.15.11'];

/** Fabric ships no snapshots or ancient releases in this stand-in. */
export function versionsFor(flavor: string): CatalogVersion[] {
  if (flavor === 'fabric') {
    return gameVersions.filter((v) => v.kind === 'release');
  }
  return gameVersions;
}
