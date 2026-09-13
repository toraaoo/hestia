import { ImagesIcon, TrashIcon } from '@phosphor-icons/react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';

import type { Screenshot } from '@/api';
import * as screenshots from '@/api/screenshot';
import { Empty } from '@/components/empty';
import { Page, Section } from '@/components/page';
import { Bone } from '@/components/skeleton';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { m } from '@/paraglide/messages.js';
import { screenshotMutations, screenshotQueries } from '@/queries/screenshot';
import { syncQueries } from '@/queries/sync';

export function ScreenshotsPage() {
  const config = useQuery(syncQueries.config());
  const list = useQuery(screenshotQueries.list());
  const remove = useMutation(screenshotMutations.remove());
  const [pending, setPending] = useState<Screenshot | null>(null);
  const readable = useReadableFolders(list.data);

  const shared = config.data?.units?.some(
    (unit) => unit.unit === 'screenshots' && unit.enabled,
  );
  const taken = list.data ?? [];

  return (
    <Page
      title={m['screenshot.title']()}
      subtitle={m['screenshot.subtitle']()}
      loading={list.isPending}
      skeleton={<Bone className="m-5 h-64" />}
    >
      {!shared && taken.length === 0 ? (
        <Empty icon={ImagesIcon} description={m['screenshot.off_hint']()}>
          {m['screenshot.off']()}
        </Empty>
      ) : taken.length === 0 ? (
        <Empty icon={ImagesIcon} description={m['screenshot.empty_hint']()}>
          {m['screenshot.empty']()}
        </Empty>
      ) : (
        <div className="flex flex-col gap-6 p-5">
          {groupByDay(taken).map(([day, shots]) => (
            <Section key={day} title={day} count={shots.length}>
              <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
                {shots.map((shot) => (
                  <figure
                    key={`${shot.instance}:${shot.file}`}
                    className="group relative overflow-hidden border border-border"
                  >
                    {readable ? (
                      <img
                        src={screenshots.url(shot)}
                        alt={shot.file}
                        loading="lazy"
                        className="aspect-video w-full object-cover"
                      />
                    ) : (
                      <Bone className="aspect-video w-full" />
                    )}
                    <figcaption className="flex items-center gap-2 px-2 py-1.5 text-[11px]">
                      <span className="min-w-0 flex-1 truncate text-muted-foreground">
                        {shot.instanceName}
                      </span>
                      <Button
                        size="icon-sm"
                        variant="ghost"
                        aria-label={m['app.action.delete']()}
                        onClick={() => setPending(shot)}
                      >
                        <TrashIcon />
                      </Button>
                    </figcaption>
                  </figure>
                ))}
              </div>
            </Section>
          ))}
        </div>
      )}

      <ConfirmDialog
        open={pending !== null}
        onOpenChange={(open) => !open && setPending(null)}
        title={m['screenshot.delete.title']()}
        description={m['screenshot.delete.body']({ file: pending?.file ?? '' })}
        confirmLabel={m['app.action.delete']()}
        destructive
        onConfirm={() => {
          if (pending) {
            remove.mutate({ instance: pending.instance, file: pending.file });
          }
          setPending(null);
        }}
      />
    </Page>
  );
}

/** The webview cannot read a shot until its folder is allowed. */
function useReadableFolders(taken: Screenshot[] | undefined) {
  const [readable, setReadable] = useState(false);
  const folders = taken ? screenshots.folders(taken).join('\n') : '';

  useEffect(() => {
    if (!folders) {
      setReadable(false);
      return;
    }
    let live = true;
    screenshots
      .allow(folders.split('\n'))
      .then(() => live && setReadable(true))
      .catch(() => live && setReadable(false));
    return () => {
      live = false;
    };
  }, [folders]);

  return readable;
}

function groupByDay(taken: Screenshot[]): [string, Screenshot[]][] {
  const days = new Map<string, Screenshot[]>();
  for (const shot of taken) {
    const day = new Date(shot.takenUnix * 1000).toLocaleDateString(undefined, {
      dateStyle: 'medium',
    });
    days.set(day, [...(days.get(day) ?? []), shot]);
  }
  return [...days.entries()];
}
