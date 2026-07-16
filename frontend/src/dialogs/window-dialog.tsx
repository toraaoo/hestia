/**
 * Declarative dialogs that become native sub-windows under Tauri and stay
 * in-page overlays in a plain browser. Usage is one factory call per dialog:
 *
 * ```tsx
 * const ConfirmDialog = windowDialog('confirm', ConfirmContent, {});
 * // …later, a normal controlled component:
 * <ConfirmDialog open={open} onOpenChange={setOpen}
 *   payload={{ text }} onResult={(ok) => …} />
 * ```
 *
 * The content component renders ordinary `DialogHeader`/`DialogFooter`
 * composition and settles through `close(result?)`. A sub-window is a
 * separate webview process, so the content is registered by id and rendered
 * there from the same bundle (the `/dialog/$id` route); only `payload` and
 * the result cross the boundary, which is why both must be serializable —
 * handlers like `onResult` stay in the opening window.
 */

import { isTauri } from '@tauri-apps/api/core';
import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { type ComponentType, useEffect, useRef, useState } from 'react';

import { Dialog, DialogContent } from '@/components/ui/dialog';
import { type DialogResultEnvelope, dialogEvent } from '@/dialogs/bridge';
import { registerDialog } from '@/dialogs/registry';

export interface WindowDialogContentProps<P, R> {
  payload: P;
  /** Settle the dialog: with a result, or with nothing when cancelled. */
  close: (result?: R) => void;
}

export interface WindowDialogOptions<P = unknown> {
  /** The native window title; defaults to the app name. */
  title?: string | ((payload: P) => string);
  /**
   * Extra classes for the `DialogContent` (e.g. a wider `sm:max-w-*`). The
   * dialog's own layout is the single source of size in both modes: the
   * sub-window is fitted to the rendered content and is not resizable.
   */
  contentClassName?: string;
}

/**
 * The hidden bootstrap viewport a sub-window lays its content out in before
 * being fitted to the measured dialog box (`/dialog/$id`'s fit pass). Only
 * needs to exceed any dialog's natural size.
 */
export const MEASURE_VIEWPORT = { width: 1024, height: 1024 };

export interface WindowDialogProps<P, R> {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Required while open; must be JSON-serializable. */
  payload?: P;
  /** Called with the content's result; not called on cancel. */
  onResult?: (result: R) => void;
}

/**
 * Once a sub-window fails to come up (denied capability, load failure),
 * every dialog degrades to the in-page overlay for the rest of the session
 * instead of re-trying a broken path per open.
 */
let subWindowsUnavailable = false;

export function windowDialog<P, R>(
  id: string,
  Content: ComponentType<WindowDialogContentProps<P, R>>,
  options: WindowDialogOptions<P>,
): ComponentType<WindowDialogProps<P, R>> {
  registerDialog(id, {
    Content: Content as ComponentType<
      WindowDialogContentProps<unknown, unknown>
    >,
    options: options as WindowDialogOptions,
  });

  function WindowDialog(props: WindowDialogProps<P, R>) {
    const [fallback, setFallback] = useState(subWindowsUnavailable);
    if (!isTauri() || fallback) {
      return <InPageHost Content={Content} options={options} {...props} />;
    }
    return (
      <SubWindowHost
        id={id}
        options={options}
        onFallback={() => {
          subWindowsUnavailable = true;
          setFallback(true);
        }}
        {...props}
      />
    );
  }
  WindowDialog.displayName = `WindowDialog(${id})`;
  return WindowDialog;
}

function InPageHost<P, R>({
  Content,
  options,
  open,
  onOpenChange,
  payload,
  onResult,
}: WindowDialogProps<P, R> & {
  Content: ComponentType<WindowDialogContentProps<P, R>>;
  options: WindowDialogOptions<P>;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className={options.contentClassName}>
        {open && payload !== undefined && (
          <Content
            payload={payload}
            close={(result) => {
              if (result !== undefined) onResult?.(result);
              onOpenChange(false);
            }}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

/** How long the sub-window gets to load and announce itself ready. */
const READY_TIMEOUT_MS = 3000;

/**
 * Renders nothing; while `open`, a sub-window at `/dialog/<id>` carries the
 * content. The opener is disabled for modality once the window exists and
 * re-enabled when the dialog settles — including an OS-level close (the
 * destroyed event). Any failure to come up (creation error, load timeout)
 * reports through `onFallback` so the dialog reopens in-page instead of
 * silently disappearing.
 */
function SubWindowHost<P, R>({
  id,
  options,
  open,
  onOpenChange,
  payload,
  onResult,
  onFallback,
}: WindowDialogProps<P, R> & {
  id: string;
  options: WindowDialogOptions<P>;
  onFallback: () => void;
}) {
  const latest = useRef({ payload, onResult, onOpenChange, onFallback });
  latest.current = { payload, onResult, onOpenChange, onFallback };

  useEffect(() => {
    if (!open) return;

    const label = `dialog-${id}-${Math.random().toString(36).slice(2, 10)}`;
    const opener = getCurrentWindow();
    const unlistens: UnlistenFn[] = [];
    let view: WebviewWindow | undefined;
    let watchdog: ReturnType<typeof setTimeout> | undefined;
    let disposed = false;
    let settled = false;

    const release = () => {
      clearTimeout(watchdog);
      for (const unlisten of unlistens) unlisten();
      unlistens.length = 0;
      opener.setEnabled(true).catch(() => {});
    };

    const settle = (result?: R) => {
      if (settled) return;
      settled = true;
      release();
      if (disposed) return;
      if (result !== undefined) latest.current.onResult?.(result);
      latest.current.onOpenChange(false);
    };

    const fail = (cause: unknown) => {
      if (settled) return;
      settled = true;
      release();
      view?.close().catch(() => {});
      console.error(
        `[dialogs] sub-window for "${id}" failed, falling back to the in-page dialog:`,
        cause,
      );
      if (!disposed) latest.current.onFallback();
    };

    (async () => {
      unlistens.push(
        await listen(dialogEvent('ready', label), () => {
          clearTimeout(watchdog);
          emit(dialogEvent('init', label), latest.current.payload);
        }),
      );
      unlistens.push(
        await listen<DialogResultEnvelope>(
          dialogEvent('result', label),
          (event) => settle(event.payload.result as R | undefined),
        ),
      );
      if (disposed) return release();

      const { title } = options;
      view = new WebviewWindow(label, {
        url: `/dialog/${id}`,
        title:
          (typeof title === 'function' && latest.current.payload !== undefined
            ? title(latest.current.payload)
            : typeof title === 'string'
              ? title
              : undefined) ?? 'Hestia',
        parent: 'main',
        ...MEASURE_VIEWPORT,
        visible: false,
        resizable: false,
        minimizable: false,
        maximizable: false,
        skipTaskbar: true,
        decorations: false,
      });
      view.once('tauri://error', (event) => fail(event.payload));
      view.once('tauri://destroyed', () => settle());
      view.once('tauri://created', () => {
        if (!settled) opener.setEnabled(false).catch(() => {});
      });
      watchdog = setTimeout(
        () => fail(`no ready signal within ${READY_TIMEOUT_MS}ms`),
        READY_TIMEOUT_MS,
      );
    })().catch(fail);

    return () => {
      disposed = true;
      if (settled) return;
      settled = true;
      release();
      view?.close().catch(() => {});
    };
  }, [open, id, options]);

  return null;
}
