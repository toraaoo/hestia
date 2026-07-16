/**
 * The cross-feature slice of the static stand-in data, shaped after the
 * daemon's real surface (see `api/*` and `crates/proto`): the signed-in
 * accounts and the daemon status the shell chrome shows, plus the content
 * vocabulary every feature labels content with. Feature-specific stand-ins
 * live in each feature's own `mock.ts`. Nothing talks to a backend.
 */

export type ContentKind =
  | 'mod'
  | 'resourcepack'
  | 'shader'
  | 'datapack'
  | 'modpack';

export interface Account {
  name: string;
  uuid: string;
  active: boolean;
}

export const accounts: Account[] = [
  {
    name: 'toraaoo',
    uuid: '8f0e1c2a-3b4d-5e6f-7a8b-9c0d1e2f3a4b',
    active: true,
  },
  {
    name: 'hearthguest',
    uuid: '11112222-3333-4444-5555-666677778888',
    active: false,
  },
];

export const account = accounts.find((a) => a.active) ?? accounts[0];

export interface DaemonStatus {
  connected: boolean;
  version: string;
  socket: string;
  uptime: string;
}

export const daemon: DaemonStatus = {
  connected: true,
  version: '0.0.1',
  socket: 'hestiad.sock',
  uptime: '3h 12m',
};
