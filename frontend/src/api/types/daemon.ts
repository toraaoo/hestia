/** Mirrors `crates/proto/src/daemon.rs`. */

export interface DaemonStatus {
  pid: number;
  version: string;
  uptimeSeconds: number;
  home: string;
  log: string;
}
