/**
 * The bespoke-command registry — everything the frontend invokes that is *not*
 * the generic `ipc_call` bridge: the shell's own commands, and the Tauri
 * plugins it bundles. The mock's answer to `crates/desktop/src/commands/`.
 */
import type { Handlers } from '../support';
import { commands as auth } from './auth';
import { commands as diagnostics } from './diagnostics';
import { commands as dialog } from './dialog';
import { commands as icons } from './icons';
import { commands as opener } from './opener';
import { commands as prefs } from './prefs';
import { commands as shell } from './shell';
import { commands as splash } from './splash';
import { commands as window } from './window';

export const commands: Handlers = {
  ...auth,
  ...diagnostics,
  ...dialog,
  ...icons,
  ...opener,
  ...prefs,
  ...shell,
  ...splash,
  ...window,
};
