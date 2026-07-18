/**
 * Host-shell affordances that do not cross the daemon socket — opening paths in
 * the OS file manager through the bundled `tauri-plugin-opener`.
 */

import { invokeCommand } from './core/ipc';

/** Open a folder (or file) in the OS file manager. */
export function openPath(path: string): Promise<void> {
  return invokeCommand('plugin:opener|open_path', { path });
}
