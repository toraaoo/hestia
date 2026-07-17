/**
 * The cross-feature slice of the static stand-in data, shaped after the
 * daemon's real surface (see `api/*` and `crates/proto`): the daemon status
 * the shell chrome shows, plus the content vocabulary every feature labels
 * content with. Feature-specific stand-ins live in each feature's own
 * `mock.ts`. Nothing talks to a backend.
 */

export type ContentKind =
  | 'mod'
  | 'resourcepack'
  | 'shader'
  | 'datapack'
  | 'modpack';

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
