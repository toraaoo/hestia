import {
  CornersInIcon,
  CornersOutIcon,
  MinusIcon,
  XIcon,
} from '@phosphor-icons/react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';

import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

/**
 * Custom min/maximize/close for the frameless window (`decorations: false`).
 * Wired to the Tauri window API; in a plain browser (`isTauri()` false, e.g.
 * `vite preview`) the buttons render but stay inert instead of throwing.
 */
export function WindowControls() {
  const [maximized, setMaximized] = useState(false);
  const inApp = isTauri();

  useEffect(() => {
    if (!inApp) return;
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;

    win
      .isMaximized()
      .then(setMaximized)
      .catch(() => {});
    win
      .onResized(() => {
        win
          .isMaximized()
          .then(setMaximized)
          .catch(() => {});
      })
      .then((fn) => {
        unlisten = fn;
      });

    return () => unlisten?.();
  }, [inApp]);

  const minimize = () => inApp && getCurrentWindow().minimize();
  const toggleMaximize = () => inApp && getCurrentWindow().toggleMaximize();
  const close = () => inApp && getCurrentWindow().close();

  return (
    <div className="flex h-full">
      <ControlButton label={m['window.minimize']()} onClick={minimize}>
        <MinusIcon weight="bold" className="size-3.5" />
      </ControlButton>
      <ControlButton
        label={maximized ? m['window.restore']() : m['window.maximize']()}
        onClick={toggleMaximize}
      >
        {maximized ? (
          <CornersInIcon weight="bold" className="size-3.5" />
        ) : (
          <CornersOutIcon weight="bold" className="size-3.5" />
        )}
      </ControlButton>
      <ControlButton label={m['window.close']()} onClick={close} danger>
        <XIcon weight="bold" className="size-3.5" />
      </ControlButton>
    </div>
  );
}

function ControlButton({
  label,
  onClick,
  danger,
  children,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={cn(
        'flex h-full w-12 items-center justify-center text-muted-foreground transition-colors outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-ring',
        danger
          ? 'hover:bg-destructive hover:text-white'
          : 'hover:bg-muted hover:text-foreground',
      )}
    >
      {children}
    </button>
  );
}
