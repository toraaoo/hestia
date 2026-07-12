/**
 * Native-app behaviors: strip the affordances a browser hands out for free so
 * the Tauri shell reads as a desktop application, not a web page.
 *
 * The aggressive parts (context menu, browser-chrome shortcuts) are skipped in
 * dev builds, where they'd fight the `tauri dev` workflow — right-click Inspect,
 * F5 reload, and the devtools stay live while developing, and disappear in the
 * shipped build (which also compiles without the webview inspector at all).
 */

const isEditable = (target: EventTarget | null): boolean => {
  const node = target as HTMLElement | null;
  if (!node) return false;
  return node.tagName === "INPUT" || node.tagName === "TEXTAREA" || node.isContentEditable;
};

export function installDesktopBehaviors(): void {
  // A ghost drag-image and a webview that navigates when a file is dropped on
  // it are never wanted — kill them in every build.
  window.addEventListener("dragstart", (e) => {
    if (!isEditable(e.target)) e.preventDefault();
  });
  window.addEventListener("dragover", (e) => e.preventDefault());
  window.addEventListener("drop", (e) => e.preventDefault());

  if (import.meta.env.DEV) return;

  // Right-click yields nothing except in editable fields, where the native
  // copy/paste/select menu still earns its keep.
  window.addEventListener("contextmenu", (e) => {
    if (!isEditable(e.target)) e.preventDefault();
  });

  // Browser-chrome shortcuts a desktop app has no business honoring: reload
  // (the page *is* the app), zoom (breaks the fixed layout), print, and find.
  window.addEventListener(
    "keydown",
    (e) => {
      const mod = e.ctrlKey || e.metaKey;
      const key = e.key.toLowerCase();
      const reload = key === "f5" || (mod && key === "r");
      const zoom = mod && ["+", "-", "=", "0"].includes(key);
      const printOrFind = mod && (key === "p" || key === "f");
      if (reload || zoom || printOrFind) e.preventDefault();
    },
    { capture: true },
  );
}
