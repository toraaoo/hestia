/** Mirrors `crates/proto/src/java.rs`. */

export interface JavaRelease {
  major: number;
  lts: boolean;
}

export interface JavaRuntime {
  vendor: string;
  major: number;
  release_name: string;
  home: string;
  executable: string;
}

export type JavaInstallPhase = 'resolving' | 'downloading' | 'extracting';

export interface JavaInstallProgress {
  phase: JavaInstallPhase;
  current: number;
  total: number;
}

export interface JavaInstallDone {
  id: string;
  runtime: JavaRuntime;
  already_installed: boolean;
}
