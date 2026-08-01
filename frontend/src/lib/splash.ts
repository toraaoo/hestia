import { invoke } from '@tauri-apps/api/core';

/**
 * Reports that the app has mounted. Not a frame callback — the window is
 * hidden until the shell reveals it, and a webview that is not visible has its
 * rendering update suspended, so `requestAnimationFrame` never runs.
 */
export function reportReady(): void {
  window.setTimeout(() => {
    invoke('ready').catch(() => {});
  }, 0);
}
