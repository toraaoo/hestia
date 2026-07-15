/**
 * Daemon events as a React hook: subscribe to one topic for the component's
 * lifetime. The handler rides a ref, so an inline closure never re-subscribes.
 */
import { useEffect, useRef } from 'react';
import { onTopic } from '../api';

export function useDaemonEvent<T = Record<string, unknown>>(
  topic: string,
  handler: (payload: T) => void,
): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    let disposed = false;
    let off: (() => void) | undefined;
    onTopic<T>(topic, (payload) => handlerRef.current(payload))
      .then((unsubscribe) => {
        if (disposed) unsubscribe();
        else off = unsubscribe;
      })
      .catch(() => {
        // Outside the Tauri shell there are no daemon events to hear.
      });
    return () => {
      disposed = true;
      off?.();
    };
  }, [topic]);
}
