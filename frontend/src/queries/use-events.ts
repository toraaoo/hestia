/** React bindings for the daemon event bus and connection state. */
import { useEffect, useRef, useState } from 'react';
import {
  type ConnectionState,
  type DaemonEvent,
  daemon,
  onConnectionChange,
  onDaemonEvent,
} from '../api';

/**
 * Subscribe to daemon events for the component's lifetime. The handler is
 * kept in a ref, so an inline closure does not re-subscribe every render.
 */
export function useDaemonEvent(
  handler: (event: DaemonEvent) => void,
  topic?: string,
): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    let off: (() => void) | undefined;
    let disposed = false;
    void onDaemonEvent((event) => {
      if (topic && event.topic !== topic) return;
      handlerRef.current(event);
    }).then((unsubscribe) => {
      if (disposed) unsubscribe();
      else off = unsubscribe;
    });
    return () => {
      disposed = true;
      off?.();
    };
  }, [topic]);
}

/**
 * The daemon connection as the UI sees it: `unknown` until the first probe
 * answers, then live transitions from the shell's watcher. The mount-time
 * ping also auto-spawns a stopped daemon.
 */
export function useConnection(): ConnectionState | 'unknown' {
  const [state, setState] = useState<ConnectionState | 'unknown'>('unknown');

  useEffect(() => {
    let off: (() => void) | undefined;
    let disposed = false;
    void onConnectionChange((next) => setState(next)).then((unsubscribe) => {
      if (disposed) unsubscribe();
      else off = unsubscribe;
    });
    daemon
      .status()
      .then(() => setState('connected'))
      .catch(() =>
        setState((current) =>
          current === 'unknown' ? 'disconnected' : current,
        ),
      );
    return () => {
      disposed = true;
      off?.();
    };
  }, []);

  return state;
}
