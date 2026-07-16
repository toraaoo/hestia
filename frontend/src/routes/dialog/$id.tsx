import { createFileRoute } from '@tanstack/react-router';
import { isTauri } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from 'react';

import { Dialog, DialogContent } from '@/components/ui/dialog';
import { dialogEvent } from '@/dialogs/bridge';
import { getDialog } from '@/dialogs/registry';
import { cn } from '@/lib/utils';

import '@/dialogs';

export const Route = createFileRoute('/dialog/$id')({
  component: DialogWindow,
});

/**
 * The surface a dialog sub-window renders: the registered content inside the
 * same `DialogContent` as the in-page overlay, so Escape, backdrop and the
 * close button behave identically. The window starts hidden at a measuring
 * size; once the payload arrives and the box is laid out, the window is
 * fitted to it, centered and shown — the dialog's own layout is the single
 * source of size (see `MEASURE_VIEWPORT` in `window-dialog.tsx`).
 */
function DialogWindow() {
  const { id } = Route.useParams();
  const entry = getDialog(id);
  const [box, setBox] = useState<{ payload: unknown } | null>(null);
  const contentRef = useRef<HTMLDivElement>(null);
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

  const ready = box !== null;
  useLayoutEffect(() => {
    if (!ready || !isTauri()) return;
    const el = contentRef.current;
    if (!el) return;

    const window = getCurrentWindow();
    let disposed = false;
    let observer: ResizeObserver | undefined;

    // offsetWidth/Height ignore the open animation's transform scale, so the
    // measurement is stable even mid-animation.
    const fitWindow = () =>
      window.setSize(new LogicalSize(el.offsetWidth, el.offsetHeight));

    (async () => {
      await Promise.race([
        document.fonts.ready,
        new Promise((resolve) => setTimeout(resolve, 300)),
      ]);
      if (disposed) return;
      await fitWindow();
      await window.center();
      await window.show();
      await window.setFocus();
      observer = new ResizeObserver(() => {
        fitWindow().catch(() => {});
      });
      observer.observe(el);
    })().catch((cause) => {
      // A window that cannot fit or show would sit invisible while the
      // opener is disabled — cancel instead so the opener recovers.
      console.error('[dialogs] fit pass failed, closing:', cause);
      close();
    });

    return () => {
      disposed = true;
      observer?.disconnect();
    };
  }, [ready, close]);

  if (!entry || !box) {
    return <div className="h-screen w-screen bg-background" />;
  }

  const Content = entry.Content;
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) close();
      }}
    >
      <DialogContent
        ref={contentRef}
        className={cn(
          'top-0 left-0 max-w-none translate-x-0 translate-y-0',
          entry.options.contentClassName,
        )}
      >
        <div data-tauri-drag-region className="absolute inset-x-0 top-0 h-6" />
        <Content payload={box.payload} close={close} />
      </DialogContent>
    </Dialog>
  );
}
