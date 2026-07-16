import { createFileRoute } from '@tanstack/react-router';
import { isTauri } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useCallback, useEffect, useRef, useState } from 'react';

import { Dialog, DialogContent } from '@/components/ui/dialog';
import { dialogEvent } from '@/dialogs/bridge';
import { getDialog } from '@/dialogs/registry';

import '@/dialogs';

export const Route = createFileRoute('/dialog/$id')({
  component: DialogWindow,
});

/**
 * The surface a dialog sub-window renders: the registered content inside the
 * same `DialogContent` as the in-page overlay, so Escape, backdrop and the
 * close button behave identically. The window was created at the content's
 * measured size (see `Measurer` in `window-dialog.tsx`), so the box simply
 * fills the viewport; the payload arrives over the bridge handshake after
 * the page announces itself ready.
 */
function DialogWindow() {
  const { id } = Route.useParams();
  const entry = getDialog(id);
  const [box, setBox] = useState<{ payload: unknown } | null>(null);
  const settled = useRef(false);

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
    listen(dialogEvent('init', label), (event) => {
      setBox({ payload: event.payload });
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

  if (!entry || !box) {
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
        <Content payload={box.payload} close={close} />
      </DialogContent>
    </Dialog>
  );
}
