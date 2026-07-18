/** Mirrors `crates/proto/src/process.rs`. */

export type RestartPolicy = 'never' | 'on_failure';

/** Externally-tagged serde enum: the unit variant is a bare string. */
export type LogSource = 'capture' | { file: string };

export interface ProcessSpec {
  /** Client-supplied id; empty asks the daemon to allocate one. */
  id?: string;
  program: string;
  args?: string[];
  cwd?: string;
  env?: Record<string, string>;
  restart?: RestartPolicy;
  log?: LogSource;
}

export type ProcessState = 'running' | 'exited' | 'killed';

export interface ProcessInfo {
  id: string;
  pid: number;
  program: string;
  args: string[];
  state: ProcessState;
  exit_code?: number;
  started_unix: number;
}

export type LogStream = 'stdout' | 'stderr';

export interface ProcessLogLine {
  stream: LogStream;
  line: string;
}

export interface ProcessExit {
  id: string;
  state: ProcessState;
  exit_code?: number;
  success: boolean;
}

/** One running process's resource sample; `cpu_pct` is 100 per full core. */
export interface ProcessMetrics {
  id: string;
  cpu_pct: number;
  mem_bytes: number;
}

export interface ProcessMetricsEvent {
  samples: ProcessMetrics[];
}
