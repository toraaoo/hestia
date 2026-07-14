/** Mirrors `crates/proto/src/download.rs`. */

export type HashAlgorithm = 'sha1' | 'sha256';

export interface Checksum {
  algorithm: HashAlgorithm;
  hex: string;
}

export interface DownloadSpec {
  id?: string;
  url: string;
  dest: string;
  checksum?: Checksum;
}

export interface DownloadProgress {
  downloaded: number;
  total: number;
}
