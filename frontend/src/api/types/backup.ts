/** Mirrors `crates/proto/src/backup.rs`. */

export type BackupKind = 'manual' | 'scheduled' | 'update';

export interface BackupInfo {
  id: string;
  kind: BackupKind;
  createdUnix: number;
  size: number;
}
