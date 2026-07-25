import { invoke } from '@tauri-apps/api/core';

import { log } from './log';

export interface CrashReport {
  path: string;
  contents: string;
}

function detailOf(error: unknown): string {
  if (error instanceof Error) {
    return error.stack ?? `${error.name}: ${error.message}`;
  }
  try {
    return JSON.stringify(error, null, 2);
  } catch {
    return String(error);
  }
}

function messageOf(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

export function report(kind: string, error: unknown, location = ''): void {
  log.error({ kind, location }, messageOf(error));
  invoke('crash_report', {
    kind,
    message: messageOf(error),
    location,
    detail: detailOf(error),
  }).catch(() => {});
}

export function installCrashHandlers(): void {
  window.addEventListener('error', (event) => {
    const where = event.filename
      ? `${event.filename}:${event.lineno}:${event.colno}`
      : '';
    report('ui-error', event.error ?? event.message, where);
  });

  window.addEventListener('unhandledrejection', (event) => {
    report('ui-rejection', event.reason);
  });
}

export async function lastCrash(): Promise<CrashReport | null> {
  const paths = await invoke<string[]>('crash_list');
  const path = paths[0];
  if (!path) return null;
  return { path, contents: await invoke<string>('crash_read', { path }) };
}

export async function clearCrashes(): Promise<void> {
  await invoke('crash_clear');
}
