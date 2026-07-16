/**
 * The wire protocol between a dialog's opener (the main window) and its
 * Tauri sub-window. Events are broadcast but scoped by the sub-window's
 * unique label, so concurrent dialogs cannot cross talk:
 *
 *   sub  → `dialog:ready:<label>`   the page is listening, send the payload
 *   main → `dialog:init:<label>`    the dialog payload (must be serializable)
 *   sub  → `dialog:result:<label>`  `{ result }`, or `{}` when cancelled
 */

export type DialogPhase = 'ready' | 'init' | 'result';

export function dialogEvent(phase: DialogPhase, label: string): string {
  return `dialog:${phase}:${label}`;
}

export interface DialogResultEnvelope {
  result?: unknown;
}
