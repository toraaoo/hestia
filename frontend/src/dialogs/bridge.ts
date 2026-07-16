/**
 * The wire protocol between a dialog's opener (the main window) and its
 * Tauri sub-window. Events are broadcast but scoped by the sub-window's
 * unique label, so concurrent dialogs cannot cross talk:
 *
 *   sub  → `dialog:ready:<label>`   the page is loaded and listening
 *   main → `dialog:init:<label>`    which dialog to mount, its payload and
 *                                   measured size (all serializable)
 *   sub  → `dialog:shown:<label>`   sized, centered and visible
 *   sub  → `dialog:result:<label>`  `{ result }`, or `{}` when cancelled
 */

export type DialogPhase = 'ready' | 'init' | 'shown' | 'result';

export function dialogEvent(phase: DialogPhase, label: string): string {
  return `dialog:${phase}:${label}`;
}

export interface DialogInitEnvelope {
  dialog: string;
  payload: unknown;
  title: string;
  width: number;
  height: number;
}

export interface DialogResultEnvelope {
  result?: unknown;
}
