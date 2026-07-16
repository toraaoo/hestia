import { createFileRoute } from '@tanstack/react-router';
import { isTauri } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import { useCallback, useEffect, useRef, useState } from 'react';

import { Dialog, DialogContent } from '@/components/ui/dialog';
import { type DialogInitEnvelope, dialogEvent } from '@/dialogs/bridge';
import { getDialog } from '@/dialogs/registry';

import '@/dialogs';

export const Route = createFileRoute('/dialog/')({
  component: DialogWindow,
});

/**
 * The page a pre-warmed dialog window idles on (see `dialogs/pool.ts`). It
 * waits hidden until an init envelope names a registered dialog and carries
 * its payload and measured size, then mounts the content inside the same
 * `DialogContent` as the in-page overlay — Escape, backdrop and the close
 * button behave identically — resizes itself to the envelope's size,
 * centers, shows, and announces `shown`.
 */
function DialogWindow() {
  const [env, setEnv] = useState<DialogInitEnvelope | null>(null);
  const settled = useRef(false);
  const shown = useRef(false);

  const close = useCallback(async (result?: unknown) => {
    if (settled.current) return;
    settled.current = true;
    const view = getCurrentWebviewWindow();
    await emit(
      dialogEvent('result', view.label),
      result === undefined ? {} : { result },
    );
    await view.close();
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    const label = getCurrentWebviewWindow().label;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    listen<DialogInitEnvelope>(dialogEvent('init', label), (event) => {
      setEnv(event.payload);
    }).then((fn) => {
      if (disposed) return fn();
      unlisten = fn;
      emit(dialogEvent('ready', label));
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!env || shown.current || !isTauri()) return;
    shown.current = true;
    const window = getCurrentWindow();
    (async () => {
      await window.setTitle(env.title);
      await window.setSize(new LogicalSize(env.width, env.height));
      await window.center();
      await window.show();
      await window.setFocus();
      await window.setResizable(false);
      await emit(dialogEvent('shown', getCurrentWebviewWindow().label));
    })().catch((cause) => {
      // A window that cannot size or show would sit invisible while the
      // opener waits — cancel instead so the opener recovers.
      console.error('[dialogs] show pass failed, closing:', cause);
      close();
    });
  }, [env, close]);

  const entry = env ? getDialog(env.dialog) : undefined;
  if (!env || !entry) {
    return <div className="h-screen w-screen bg-popover" />;
  }

  const Content = entry.Content;
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) close();
      }}
    >
      <DialogContent className="top-0 left-0 h-screen w-screen max-w-none translate-x-0 translate-y-0 overflow-y-auto sm:max-w-none">
        <div data-tauri-drag-region className="absolute inset-x-0 top-0 h-6" />
        <Content payload={env.payload} close={close} />
      </DialogContent>
    </Dialog>
  );
}
