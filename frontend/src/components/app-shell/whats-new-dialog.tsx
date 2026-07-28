import { useQuery } from '@tanstack/react-query';

import { Markdown } from '@/components/markdown';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { m } from '@/paraglide/messages.js';
import { appQueries } from '@/queries/app';
import { usePrefs } from '@/queries/prefs';
import { changelogQuery } from '@/queries/update';

const SEEN_KEY = 'lastSeenVersion';

/**
 * This build's release notes, shown once after an upgrade.
 *
 * A local mechanism, not a feed: the trigger is "the version changed since this
 * machine last ran", and the notes are compiled into the binary — so it works
 * offline, always matches the build exactly, and cannot be spoofed. The
 * announcement feed covers the other case, things learned *after* a release.
 *
 * **Upgrade only.** With no recorded version this is a fresh install: record it
 * and show nothing. A first-time user has no context for "what changed" and
 * already meets the first-run overlay.
 */
export function WhatsNewDialog() {
  const prefs = usePrefs();
  const app = useQuery(appQueries.info());
  const version = app.data?.version;
  const seen = prefs.get<string | null>(SEEN_KEY, null);
  const upgraded = Boolean(version && seen && seen !== version);
  const notes = useQuery({ ...changelogQuery(), enabled: upgraded });

  // Record silently on a fresh install, so the *next* upgrade has a baseline.
  if (version && !seen && prefs.ready) {
    prefs.set(SEEN_KEY, version);
    return null;
  }

  const acknowledge = () => {
    if (version) prefs.set(SEEN_KEY, version);
  };

  // A build with no changelog section shows nothing, but still records the
  // version — otherwise it would re-check on every launch forever.
  if (upgraded && notes.isFetched && !notes.data?.trim()) {
    acknowledge();
    return null;
  }
  if (!upgraded || !notes.data?.trim()) return null;

  return (
    <AlertDialog open>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {m['update.whats_new']({ version: version ?? '' })}
          </AlertDialogTitle>
          <AlertDialogDescription>
            <Markdown>{notes.data}</Markdown>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogAction onClick={acknowledge}>
            {m['news.acknowledge']()}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
