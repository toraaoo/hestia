/**
 * Dropping an instance archive onto a page.
 *
 * The webview's drag-drop event is the only way to learn a real filesystem
 * path from a drop — an HTML `DataTransfer` hands back a sandboxed `File`, and
 * the daemon needs a path it can open itself.
 *
 * Only files that *look* like an archive arm the drop: the format is still
 * decided by the daemon reading the file, but lighting the whole window up for
 * a dragged screenshot would be a lie about what will happen.
 */
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { useEffect, useRef, useState } from 'react';
import { logger } from '@/lib/log';

const log = logger('drop');

const ARCHIVE = /\.(hestia|mrpack|zip)$/i;

export function useArchiveDrop(onDrop: (path: string) => void): boolean {
  const [over, setOver] = useState(false);
  // Held in a ref so a caller passing an inline closure does not tear the
  // listener down and set it up again on every render.
  const handler = useRef(onDrop);
  handler.current = onDrop;

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === 'leave') {
          setOver(false);
          return;
        }
        const paths = 'paths' in payload ? payload.paths : [];
        const archive = paths.find((path) => ARCHIVE.test(path));
        if (payload.type === 'drop') {
          setOver(false);
          if (archive) handler.current(archive);
          return;
        }
        setOver(!!archive);
      })
      .then((off) => {
        if (cancelled) off();
        else unlisten = off;
      })
      .catch((error) => log.warn('cannot listen for dropped files', error));

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return over;
}
