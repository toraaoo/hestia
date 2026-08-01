import { invoke } from '@tauri-apps/api/core';

const MIN_VISIBLE_MS = 1900;

/**
 * Reveals the main window once the app has painted. The shell holds the
 * splash until then, so the delay here is the animation's own length — the
 * mark finishes drawing rather than being cut off on a fast boot.
 */
export function reportReady(): void {
  const remaining = Math.max(0, MIN_VISIBLE_MS - performance.now());
  window.setTimeout(() => {
    invoke('ready').catch(() => {});
  }, remaining);
}
