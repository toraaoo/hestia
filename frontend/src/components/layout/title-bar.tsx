import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CloseIcon, CornersOutIcon, MinusIcon, SquareIcon } from "@/components/icons";
import logoEmber from "@/assets/brand/logo-ember.svg";

const inTauri = "__TAURI_INTERNALS__" in window;

/**
 * Frameless-window titlebar. Replaces the mock's macOS-dot bar with the
 * launcher's own chrome: the brand lockup sits over the sidebar column, a
 * faint ember line runs along the top edge, and real window controls live
 * on the right. The whole bar is a Tauri drag region (double-click
 * maximizes natively).
 */
export function TitleBar() {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!inTauri) return;
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    void win.isMaximized().then(setMaximized);
    void win
      .onResized(() => void win.isMaximized().then(setMaximized))
      .then((fn) => (unlisten = fn));
    return () => unlisten?.();
  }, []);

  const control = (action: "minimize" | "maximize" | "close") => {
    if (!inTauri) return;
    const win = getCurrentWindow();
    if (action === "minimize") void win.minimize();
    if (action === "maximize") void win.toggleMaximize();
    if (action === "close") void win.close();
  };

  return (
    <header
      data-tauri-drag-region
      className="relative flex h-10 shrink-0 items-stretch border-b border-border-2 bg-chrome"
    >
      <span
        aria-hidden
        className="pointer-events-none absolute inset-x-0 top-0 h-px bg-gradient-to-r from-hearth-500/60 via-hearth-500/15 to-transparent"
      />

      <div data-tauri-drag-region className="flex w-58 items-center gap-2.5 px-4">
        <img src={logoEmber} alt="" className="pointer-events-none size-4.5 rounded-xs" />
        <span className="pointer-events-none font-hero text-sm leading-none tracking-wide text-fg-2 font-crisp">
          HESTIA
        </span>
      </div>

      <div data-tauri-drag-region className="flex-1" />

      <div className="flex items-stretch">
        <button
          aria-label="Minimize"
          onClick={() => control("minimize")}
          className="flex w-12 items-center justify-center text-fg-3 transition-colors duration-100 hover:bg-surface-hover hover:text-fg-1"
        >
          <MinusIcon size={14} />
        </button>
        <button
          aria-label={maximized ? "Restore" : "Maximize"}
          onClick={() => control("maximize")}
          className="flex w-12 items-center justify-center text-fg-3 transition-colors duration-100 hover:bg-surface-hover hover:text-fg-1"
        >
          {maximized ? <CornersOutIcon size={13} /> : <SquareIcon size={12} />}
        </button>
        <button
          aria-label="Close"
          onClick={() => control("close")}
          className="flex w-12 items-center justify-center text-fg-3 transition-colors duration-100 hover:bg-tnt-500 hover:text-white"
        >
          <CloseIcon size={14} />
        </button>
      </div>
    </header>
  );
}
