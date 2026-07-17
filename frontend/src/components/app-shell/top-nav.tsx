import { CaretLeftIcon, CaretRightIcon } from '@phosphor-icons/react';
import { useCanGoBack, useRouter } from '@tanstack/react-router';

import { Logo } from '@/components/app-shell/logo';
import { WindowControls } from '@/components/app-shell/window-controls';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

/**
 * The window's single top bar: brand, browser-style history nav, and the
 * frameless window controls — one draggable strip, the way a desktop shell
 * reads. Search lives in each page header, not here.
 */
export function TopNav() {
  const router = useRouter();
  const canGoBack = useCanGoBack();

  return (
    <header
      data-tauri-drag-region
      className="flex h-11 shrink-0 items-stretch border-b border-border bg-sidebar select-none"
    >
      <div className="flex items-center gap-1 pl-3" data-tauri-drag-region>
        <Logo className="size-5" />
        <span className="mr-1 text-sm font-semibold">Hestia</span>
        <div className="mx-1 h-4 w-px bg-border" />
        <HistoryButton
          label={m['nav.back']()}
          disabled={!canGoBack}
          onClick={() => router.history.back()}
        >
          <CaretLeftIcon weight="bold" className="size-4" />
        </HistoryButton>
        <HistoryButton
          label={m['nav.forward']()}
          onClick={() => router.history.forward()}
        >
          <CaretRightIcon weight="bold" className="size-4" />
        </HistoryButton>
      </div>

      <div className="flex-1" data-tauri-drag-region />

      <WindowControls />
    </header>
  );
}

function HistoryButton({
  label,
  disabled,
  onClick,
  children,
}: {
  label: string;
  disabled?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className={cn(
        'flex size-7 items-center justify-center text-muted-foreground transition-colors outline-none hover:bg-muted hover:text-foreground focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-40',
      )}
    >
      {children}
    </button>
  );
}
