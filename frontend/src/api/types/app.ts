/** Mirrors `crates/proto/src/app.rs` and `health.rs`. */

export interface AppInfo {
  name: string;
  version: string;
  id: string;
  vendor: string;
  channel: string;
}

export interface PingResult {
  status: string;
  pid: number;
}
