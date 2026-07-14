/** Mirrors `crates/proto/src/backup.rs`. */

export type BackupKind = 'manual' | 'scheduled' | 'update';

export interface BackupInfo {
  id: string;
  kind: BackupKind;
  created_unix: number;
  size: number;
}
