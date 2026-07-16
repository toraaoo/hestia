function inEditableField(target: EventTarget | null): boolean {
  return (
    target instanceof Element &&
    target.closest('input, textarea, [contenteditable]') !== null
  );
}

/**
 * Suppresses the webview behaviors that make the shell feel like a browser:
 * the page context menu (kept on editable fields for native cut/copy/paste),
 * pinch/ctrl-wheel zoom, and — in release builds only, so dev reload and
 * devtools stay reachable — the browser accelerators (reload, find, print,
 * zoom, view-source).
 */
export function initDesktopShell(): void {
  document.addEventListener('contextmenu', (event) => {
    if (!inEditableField(event.target)) event.preventDefault();
  });

  document.addEventListener(
    'wheel',
    (event) => {
      if (event.ctrlKey) event.preventDefault();
    },
    { passive: false },
  );

  if (!import.meta.env.PROD) return;

  document.addEventListener('keydown', (event) => {
    const key = event.key.toLowerCase();
    const combo = event.ctrlKey || event.metaKey;
    if (
      key === 'f5' ||
      (combo && ['r', 'f', 'g', 'p', 'u', '+', '-', '=', '0'].includes(key))
    ) {
      event.preventDefault();
    }
  });
}
