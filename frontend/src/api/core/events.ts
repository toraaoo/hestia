/**
 * Daemon events, as the shell forwards them: one Tauri event carries every
 * daemon push (`hestia:event`), another the connection-state transitions
 * (`hestia:connection`). The bridge holds the client session's single
 * event-callback slot and subscribes to everything, so multiplexing by topic
 * and job id happens here, where many concurrent listeners are natural.
 */
import { listen } from '@tauri-apps/api/event';

export const EVENT_CHANNEL = 'hestia:event';
export const CONNECTION_CHANNEL = 'hestia:connection';

export interface DaemonEvent {
  topic: string;
  payload: Record<string, unknown>;
}

export type ConnectionState = 'connected' | 'disconnected';

type DaemonEventHandler = (event: DaemonEvent) => void;
type ConnectionHandler = (state: ConnectionState) => void;

const eventHandlers = new Set<DaemonEventHandler>();
const connectionHandlers = new Set<ConnectionHandler>();
let eventListener: Promise<unknown> | null = null;
let connectionListener: Promise<unknown> | null = null;

async function ensureEventListener(): Promise<void> {
  eventListener ??= listen<DaemonEvent>(EVENT_CHANNEL, (event) => {
    // The wire is camelCase (proto's `rename_all`), so the payload already
    // matches the type mirrors — forwarded through with no conversion.
    for (const handler of [...eventHandlers]) handler(event.payload);
  });
  await eventListener;
}

async function ensureConnectionListener(): Promise<void> {
  connectionListener ??= listen<ConnectionState>(
    CONNECTION_CHANNEL,
    (event) => {
      for (const handler of [...connectionHandlers]) handler(event.payload);
    },
  );
  await connectionListener;
}

/**
 * Receive every daemon event. Resolves once the underlying listener is
 * installed, so events arriving after the returned promise settles are never
 * missed. The returned function unsubscribes.
 */
export async function onDaemonEvent(
  handler: DaemonEventHandler,
): Promise<() => void> {
  await ensureEventListener();
  eventHandlers.add(handler);
  return () => eventHandlers.delete(handler);
}

/** Receive only events on one topic. */
export async function onTopic<T = Record<string, unknown>>(
  topic: string,
  handler: (payload: T) => void,
): Promise<() => void> {
  return onDaemonEvent((event) => {
    if (event.topic === topic) handler(event.payload as T);
  });
}

/** Receive connection-state transitions from the shell's watcher. */
export async function onConnectionChange(
  handler: ConnectionHandler,
): Promise<() => void> {
  await ensureConnectionListener();
  connectionHandlers.add(handler);
  return () => connectionHandlers.delete(handler);
}
