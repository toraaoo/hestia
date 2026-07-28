/**
 * Self-update. Unlike every other domain this does **not** go over the daemon
 * bridge: the desktop updates itself through `tauri-plugin-updater`, which
 * verifies the installer signature and restarts the app — both things only the
 * shell can do. So these are the shell's own commands, like `prefs.*`.
 *
 * The daemon has its own `update.*` channels for the CLI; the two are separate
 * paths to the same release feed, and the shell's is the one that can replace a
 * running binary.
 */
import { invokeCommand } from './core/ipc';

export interface DesktopUpdate {
  version: string;
  notes: string | null;
}

/** The newer version on the release feed, or null when up to date. */
export function check(): Promise<DesktopUpdate | null> {
  return invokeCommand('update_check');
}

/**
 * Download, verify, install, and restart into the new version. Resolves only
 * if the install failed — on success the app is replaced and restarted.
 */
export function install(): Promise<void> {
  return invokeCommand('update_install');
}
