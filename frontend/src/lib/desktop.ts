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

// Form fields are the only focusable things in the app; everything else is
// chrome. The Base UI controls that render as buttons/divs count as fields.
const FORM_FIELD =
  "input, textarea, select, [contenteditable='true'], " +
  "[data-slot='checkbox'], [data-slot='switch'], [data-slot='select-trigger'], [data-slot='slider-thumb']";

const formFields = (): HTMLElement[] =>
  Array.from(document.querySelectorAll<HTMLElement>(FORM_FIELD)).filter(
    (el) => !el.matches(":disabled") && el.offsetParent !== null,
  );

export function installDesktopBehaviors(): void {
  // Chrome (buttons, links, tabs, the sidebar, window controls) never takes
  // focus: a click must not move focus off a form field, and Tab walks form
  // fields only — never the chrome. Popups are untouched: Base UI focuses
  // them programmatically and handles its own keys before these listeners.
  window.addEventListener("mousedown", (e) => {
    const chrome =
      e.target instanceof Element
        ? e.target.closest("a, button, [role='button'], [role='tab'], [tabindex]")
        : null;
    if (chrome && !chrome.closest(FORM_FIELD) && !chrome.closest("label")) e.preventDefault();
  });

  // Base UI focuses triggers programmatically (after click, and again when a
  // popup closes) — the mousedown guard can't stop that, so any focus landing
  // on chrome is blurred on arrival. Popup internals are exempt: menus paint
  // hover/keyboard highlight through focus and must keep it.
  window.addEventListener("focusin", (e) => {
    const el = e.target as HTMLElement;
    if (el.closest(FORM_FIELD)) return;
    if (
      el.closest("[role='menu'], [role='menuitem'], [role='listbox'], [role='dialog'], [popover]")
    )
      return;
    el.blur();
  });

  window.addEventListener("keydown", (e) => {
    if (e.key !== "Tab" || e.defaultPrevented) return;
    e.preventDefault();
    const fields = formFields();
    if (fields.length === 0) return;
    const current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const index = current ? fields.indexOf(current) : -1;
    const next = e.shiftKey
      ? fields[index <= 0 ? fields.length - 1 : index - 1]
      : fields[(index + 1) % fields.length];
    next?.focus();
  });

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
