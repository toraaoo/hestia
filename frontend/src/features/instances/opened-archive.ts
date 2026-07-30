/**
 * Instance archives the OS hands the app.
 *
 * Installing hestia claims `.hestia`, so a double-clicked archive reaches the
 * shell either as a launch argument or — when a window is already up — as an
 * event. Both end here, and the library picks it up wherever the app happens to
 * be: the delivery is global, but the dialog that answers it lives on one page.
 */
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { system } from '@/api';
import { logger } from '@/lib/log';

const log = logger('archive');

const ARCHIVE_OPENED = 'archive-opened';

let pending: string | null = null;
const listeners = new Set<(path: string) => void>();

/** Hand an opened archive to whoever is listening, or hold it until someone is. */
function deliver(path: string): void {
  if (listeners.size === 0) {
    pending = path;
    return;
  }
  for (const listener of listeners) listener(path);
}

let started = false;

/** Start receiving opened archives. Called once, at app bootstrap. */
export function watchOpenedArchives(): void {
  if (started) return;
  started = true;
  // The launch argument was captured before the webview existed, so it is
  // waiting on the shell rather than arriving as an event.
  system
    .pendingArchive()
    .then((path) => {
      if (path) deliver(path);
    })
    .catch((error) => log.warn('cannot read the opened archive', error));
  listen<string>(ARCHIVE_OPENED, (event) => deliver(event.payload)).catch(
    (error) => log.warn('cannot listen for opened archives', error),
  );
}

/**
 * Run `onOpen` for each archive the OS hands the app while this component is
 * mounted — including one that arrived before it was.
 */
export function useOpenedArchive(onOpen: (path: string) => void): void {
  useEffect(() => {
    listeners.add(onOpen);
    if (pending) {
      const path = pending;
      pending = null;
      onOpen(path);
    }
    return () => {
      listeners.delete(onOpen);
    };
  }, [onOpen]);
}
