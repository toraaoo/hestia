import type { ComponentType } from 'react';

import type {
  WindowDialogContentProps,
  WindowDialogOptions,
} from '@/dialogs/window-dialog';

export interface RegisteredDialog {
  Content: ComponentType<WindowDialogContentProps<unknown, unknown>>;
  options: WindowDialogOptions;
}

const dialogs = new Map<string, RegisteredDialog>();

/** Called by `windowDialog` at module load; never call directly. */
export function registerDialog(id: string, entry: RegisteredDialog) {
  dialogs.set(id, entry);
}

export function getDialog(id: string): RegisteredDialog | undefined {
  return dialogs.get(id);
}
