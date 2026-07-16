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
import {
  type ComponentType,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from 'react';
import { createPortal } from 'react-dom';

import { Dialog, DialogContent } from '@/components/ui/dialog';
import { type DialogResultEnvelope, dialogEvent } from '@/dialogs/bridge';
import { registerDialog } from '@/dialogs/registry';
import { cn } from '@/lib/utils';

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
   * sub-window is created at the measured size of the rendered content and
   * is not resizable.
   */
  contentClassName?: string;
}

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
        Content={Content}
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
const READY_TIMEOUT_MS = 10_000;

/**
 * The size-affecting classes of `DialogContent`, replicated for the hidden
 * measuring pass. Kept in sync with `components/ui/dialog.tsx`.
 */
const MEASURE_BOX =
  'fixed top-0 left-0 grid w-full max-w-[calc(100%-2rem)] gap-4 p-4 text-xs/relaxed sm:max-w-sm invisible pointer-events-none';

interface BoxSize {
  width: number;
  height: number;
}

/**
 * Renders the dialog content invisibly in the opener to take its natural
 * size. A hidden Wayland window has no mapped surface (its layout viewport
 * is bogus), so measuring must happen here — a real, font-loaded viewport
 * with the same CSS — and the sub-window is then created at exactly this
 * size, visible from the start.
 */
function Measurer<P, R>({
  Content,
  options,
  payload,
  onMeasured,
}: {
  Content: ComponentType<WindowDialogContentProps<P, R>>;
  options: WindowDialogOptions<P>;
  payload: P;
  onMeasured: (size: BoxSize) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    onMeasured({ width: el.offsetWidth, height: el.offsetHeight });
  }, [onMeasured]);

  return createPortal(
    // A Dialog root only for context: DialogTitle/DialogDescription inside
    // the content need it; nothing of the overlay itself renders here.
    <Dialog open modal={false}>
      <div
        ref={ref}
        aria-hidden
        className={cn(MEASURE_BOX, options.contentClassName)}
      >
        <Content payload={payload} close={() => {}} />
      </div>
    </Dialog>,
    document.body,
  );
}

/**
 * Renders only the measuring pass; once sized, a sub-window at
 * `/dialog/<id>` carries the content. The opener is disabled for modality
 * once the window exists and re-enabled when the dialog settles — including
 * an OS-level close (the destroyed event). Any failure to come up (creation
 * error, load timeout) reports through `onFallback` so the dialog reopens
 * in-page instead of silently disappearing.
 */
function SubWindowHost<P, R>({
  id,
  Content,
  options,
  open,
  onOpenChange,
  payload,
  onResult,
  onFallback,
}: WindowDialogProps<P, R> & {
  id: string;
  Content: ComponentType<WindowDialogContentProps<P, R>>;
  options: WindowDialogOptions<P>;
  onFallback: () => void;
}) {
  const [size, setSize] = useState<BoxSize | null>(null);
  const latest = useRef({ payload, onResult, onOpenChange, onFallback });
  latest.current = { payload, onResult, onOpenChange, onFallback };

  useEffect(() => {
    if (!open) setSize(null);
  }, [open]);

  useEffect(() => {
    if (!open || !size) return;

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
        width: size.width,
        height: size.height,
        center: true,
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
  }, [open, size, id, options]);

  if (!open || size || payload === undefined) return null;
  return (
    <Measurer
      Content={Content}
      options={options}
      payload={payload}
      onMeasured={(measured) => setSize((prev) => prev ?? measured)}
    />
  );
}
