/**
 * Desktop-only window behavior over the daemon's process events: hide the
 * launcher while a game runs and restore it when the last session exits. Play
 * accounting (last-played, playtime) is the daemon's now, carried on the
 * instance record and detail views.
 */
import { getCurrentWindow } from '@tauri-apps/api/window';
import { onDaemonEvent } from '../api';
import { queryClient } from './client';
import { instanceQueries } from './instance';
import { prefsQueries } from './prefs';

/** The prefs key for "keep the launcher open while a game runs". */
export const KEEP_OPEN_KEY = 'keepOpen';

const SESSION_ID = /^instance-(.+)_\d+$/;

const running = new Set<string>();
let hidden = false;
let started = false;

async function keepOpen(): Promise<boolean> {
  const all = await queryClient.ensureQueryData(prefsQueries.list());
  return (all[KEEP_OPEN_KEY] as boolean | undefined) ?? true;
}

async function hideWindow(): Promise<void> {
  if (hidden) return;
  hidden = true;
  await getCurrentWindow().hide();
}

async function restoreWindow(): Promise<void> {
  if (!hidden) return;
  hidden = false;
  const window = getCurrentWindow();
  await window.show();
  await window.setFocus();
}

async function onStarted(payload: Record<string, unknown>): Promise<void> {
  const id = String(payload.id ?? '');
  if (!SESSION_ID.test(id)) return;
  running.add(id);
  if (!(await keepOpen())) await hideWindow();
}

async function onExit(payload: Record<string, unknown>): Promise<void> {
  const id = String(payload.id ?? '');
  if (!running.delete(id)) return;
  if (running.size === 0) await restoreWindow();
}

async function adoptRunning(): Promise<void> {
  const instances = await queryClient.ensureQueryData(instanceQueries.list());
  for (const instance of instances) {
    for (const session of instance.sessions ?? []) {
      if (session.state === 'running') running.add(session.id);
    }
  }
}

/** Install the window tracker once, at app bootstrap. */
export function startSessionTracking(): void {
  if (started) return;
  started = true;
  onDaemonEvent((event) => {
    if (event.topic === 'process.started') void onStarted(event.payload);
    if (event.topic === 'process.exit') void onExit(event.payload);
  }).catch(() => {
    // Outside the Tauri shell there are no daemon events to hear.
  });
  adoptRunning().catch(() => {
    // No account yet (the instance surface is gated) or no daemon — fine.
  });
}
