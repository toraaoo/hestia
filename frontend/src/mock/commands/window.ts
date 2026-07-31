/**
 * `tauri-plugin-window` — the frameless window's own controls, plus the
 * hide/show the session tracker drives while a game runs. A browser tab has no
 * window to move, so these keep the state the UI reads back (`is_maximized`)
 * and otherwise no-op rather than falling through to the empty fallback.
 *
 * `@tauri-apps/api`'s `isTauri()` stays false under the mock, so the window
 * controls render inert on purpose — a tab cannot honestly answer a minimize.
 */
import type { Handlers } from '../support';

let maximized = false;

const noop = () => null;

export const commands: Handlers = {
  'plugin:window|is_maximized': () => maximized,
  'plugin:window|maximize': () => {
    maximized = true;
    return null;
  },
  'plugin:window|unmaximize': () => {
    maximized = false;
    return null;
  },
  'plugin:window|toggle_maximize': () => {
    maximized = !maximized;
    return null;
  },
  'plugin:window|minimize': noop,
  'plugin:window|hide': noop,
  'plugin:window|show': noop,
  'plugin:window|set_focus': noop,
  'plugin:window|close': noop,
  'plugin:window|destroy': noop,
  'plugin:window|start_dragging': noop,
};
