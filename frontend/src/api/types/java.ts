/** Mirrors `crates/proto/src/java.rs`. */

export interface JavaRelease {
  major: number;
  lts: boolean;
}

export interface JavaRuntime {
  vendor: string;
  major: number;
  releaseName: string;
  home: string;
  executable: string;
  /** Whether an existing server or instance launches with this major. */
  inUse: boolean;
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
  alreadyInstalled: boolean;
}
