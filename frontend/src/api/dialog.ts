/**
 * Native file dialogs, over `@tauri-apps/plugin-dialog`. A local-file content
 * import needs a real daemon-readable path (the daemon reads the file itself),
 * which only the shell's native picker can produce — a browser file input hands
 * back a sandboxed `File`, never a path.
 */
import { open } from '@tauri-apps/plugin-dialog';

/** File extensions a content import accepts, by nothing more than convention. */
const CONTENT_EXTENSIONS = ['jar', 'zip', 'mrpack'];

/**
 * Pick content files and return their absolute paths (empty if the dialog was
 * dismissed). Each path is passed straight to `content.add`'s `path` field.
 */
export async function pickContentFiles(): Promise<string[]> {
  const selection = await open({
    multiple: true,
    directory: false,
    filters: [{ name: 'Content', extensions: CONTENT_EXTENSIONS }],
  });
  if (Array.isArray(selection)) return selection;
  return typeof selection === 'string' ? [selection] : [];
}

/** Pick one image and return its absolute path, or null when dismissed. */
export async function pickImage(): Promise<string | null> {
  const selection = await open({
    multiple: false,
    directory: false,
    filters: [
      { name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif'] },
    ],
  });
  return typeof selection === 'string' ? selection : null;
}
