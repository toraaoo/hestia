/**
 * A pre-warmed dialog window: spawning a webview is the expensive part of
 * opening a dialog sub-window (process start + bundle load), so one hidden
 * window at `/dialog` is created ahead of time with the dialog registry
 * already loaded. Opening a dialog then only sends it an init envelope and
 * shows it. Dialogs are modal (one at a time), so a pool of one suffices —
 * it is replenished when a dialog settles.
 */

import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

import { type DialogInitEnvelope, dialogEvent } from '@/dialogs/bridge';

export interface DialogWindow {
  label: string;
  view: WebviewWindow;
  /** Resolves once the page has announced `ready`; then init may be sent. */
  whenReady: Promise<void>;
  /** Sends the init envelope (after `whenReady`). */
  init: (envelope: DialogInitEnvelope) => void;
  /** Releases the warm-up listener; call when the dialog settles. */
  release: () => void;
}

export function spawnDialogWindow(): DialogWindow {
  const label = `dialog-${Math.random().toString(36).slice(2, 10)}`;

  let announceReady: () => void;
  const whenReady = new Promise<void>((resolve) => {
    announceReady = resolve;
  });
  let unlisten: UnlistenFn | undefined;
  let released = false;
  listen(dialogEvent('ready', label), () => announceReady()).then((fn) => {
    if (released) fn();
    else unlisten = fn;
  });

  const view = new WebviewWindow(label, {
    url: '/dialog',
    title: 'Hestia',
    parent: 'main',
    // Placeholder geometry: the window is hidden and resizes itself to the
    // init envelope's measured size before showing.
    width: 320,
    height: 240,
    visible: false,
    // Resizable while hidden — GTK ignores programmatic resizes on
    // non-resizable windows; the page locks it after showing.
    minimizable: false,
    maximizable: false,
    skipTaskbar: true,
    decorations: false,
  });

  return {
    label,
    view,
    whenReady,
    init: (envelope) => {
      emit(dialogEvent('init', label), envelope);
    },
    release: () => {
      released = true;
      unlisten?.();
      unlisten = undefined;
    },
  };
}

let pooled: DialogWindow | null = null;

/** Ensures a warm window is waiting; safe to call repeatedly. */
export function warmDialogPool() {
  if (!pooled) pooled = spawnDialogWindow();
}

/** The warm window if one is waiting, else `null` (spawn cold instead). */
export function takeDialogWindow(): DialogWindow | null {
  const taken = pooled;
  pooled = null;
  return taken;
}
