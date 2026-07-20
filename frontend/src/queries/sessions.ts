/**
 * Desktop-side session tracking over the daemon's process events: per-instance
 * last-played and accumulated playtime persisted into prefs, plus the
 * keep-open behavior (hide the window while a game runs, restore when the
 * last session exits). Prefs-based by design — only sessions this window
 * observes are counted; a CLI-launched game with the desktop closed is not.
 */
import { getCurrentWindow } from '@tauri-apps/api/window';
import { onDaemonEvent } from '../api';
import * as prefsApi from '../api/prefs';
import { queryClient } from './client';
import { instanceQueries } from './instance';
import { keys } from './keys';
import { prefsQueries, usePrefs } from './prefs';

export interface Playtime {
  lastPlayedUnix: number;
  totalSeconds: number;
}

export const playtimeKey = (instanceId: string) => `playtime.${instanceId}`;

/** The prefs key for "keep the launcher open while a game runs". */
export const KEEP_OPEN_KEY = 'keepOpen';

const SESSION_ID = /^instance-(.+)_\d+$/;

const live = new Map<string, { instanceId: string; startedUnix: number }>();
let hidden = false;
let started = false;

async function prefs(): Promise<Record<string, unknown>> {
  return queryClient.ensureQueryData(prefsQueries.list());
}

async function writePref(key: string, value: unknown): Promise<void> {
  await prefsApi.set(key, value);
  queryClient.setQueryData<Record<string, unknown>>(
    keys.prefs.list(),
    (current) => ({ ...(current ?? {}), [key]: value }),
  );
}

async function recordStart(instanceId: string): Promise<void> {
  const all = await prefs();
  const current = (all[playtimeKey(instanceId)] ?? {
    lastPlayedUnix: 0,
    totalSeconds: 0,
  }) as Playtime;
  await writePref(playtimeKey(instanceId), {
    ...current,
    lastPlayedUnix: Math.floor(Date.now() / 1000),
  });
}

async function recordExit(
  instanceId: string,
  startedUnix: number,
): Promise<void> {
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - startedUnix);
  const all = await prefs();
  const current = (all[playtimeKey(instanceId)] ?? {
    lastPlayedUnix: 0,
    totalSeconds: 0,
  }) as Playtime;
  await writePref(playtimeKey(instanceId), {
    ...current,
    totalSeconds: current.totalSeconds + seconds,
  });
}

async function keepOpen(): Promise<boolean> {
  const all = await prefs();
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
  const match = SESSION_ID.exec(String(payload.id ?? ''));
  if (!match) return;
  const instanceId = match[1];
  live.set(String(payload.id), {
    instanceId,
    startedUnix: Math.floor(Date.now() / 1000),
  });
  await recordStart(instanceId);
  if (!(await keepOpen())) await hideWindow();
}

async function onExit(payload: Record<string, unknown>): Promise<void> {
  const session = live.get(String(payload.id ?? ''));
  if (!session) return;
  live.delete(String(payload.id));
  await recordExit(session.instanceId, session.startedUnix);
  if (live.size === 0) await restoreWindow();
}

/** Adopt sessions already running when the window opens: the supervisor
 * carries the true start time, so their exit records the full session. */
async function adoptRunning(): Promise<void> {
  const instances = await queryClient.ensureQueryData(instanceQueries.list());
  for (const instance of instances) {
    for (const session of instance.sessions ?? []) {
      if (session.state === 'running' && !live.has(session.id)) {
        live.set(session.id, {
          instanceId: instance.id,
          startedUnix: session.startedUnix,
        });
      }
    }
  }
}

/** Install the tracker once, at app bootstrap. */
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

/** One instance's observed playtime, from prefs; null before any launch. */
export function usePlaytime(instanceId: string): Playtime | null {
  const { get } = usePrefs();
  return get<Playtime | null>(playtimeKey(instanceId), null);
}
