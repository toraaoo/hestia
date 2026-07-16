/**
 * Every window dialog is re-exported here, one line apiece — importing this
 * module registers them all, which is how the `/dialog/$id` sub-window route
 * can render any dialog by id from the shared bundle.
 */

export { EditSkinDialog } from '@/components/launcher/edit-skin-dialog';
export type {
  WindowDialogContentProps,
  WindowDialogOptions,
  WindowDialogProps,
} from '@/dialogs/window-dialog';
export { windowDialog } from '@/dialogs/window-dialog';
