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
 * Pick one content file and return its absolute path, or `null` if the dialog
 * was dismissed. The path is passed straight to `content.add`'s `path` field.
 */
export async function pickContentFile(): Promise<string | null> {
  const selection = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'Content', extensions: CONTENT_EXTENSIONS }],
  });
  return typeof selection === 'string' ? selection : null;
}
