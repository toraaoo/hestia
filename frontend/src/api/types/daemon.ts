/** Mirrors `crates/proto/src/daemon.rs`. */

export interface DaemonStatus {
  pid: number;
  version: string;
  uptime_seconds: number;
  home: string;
  log: string;
}
