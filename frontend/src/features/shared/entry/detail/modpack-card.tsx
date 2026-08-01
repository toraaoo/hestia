import { PackageIcon } from '@phosphor-icons/react';
import { useState } from 'react';
import { toast } from 'sonner';

import { errorMessage } from '@/api';
import { Stat } from '@/components/page';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { WarningNotice } from '@/components/warning-notice';
import { m } from '@/paraglide/messages.js';
import type { JobEntryKind } from '@/queries';
import { useModpack, useRemoveModpack, useUpdateModpack } from '@/queries';

import { ProvisionProgressView } from '../components/provision-progress';
import { SideCard } from './cards';

/**
 * The pack an entry runs, on its overview: what it is, and the two things that
 * can be done to it. Absent when the entry was not built from one — which is an
 * ordinary state, not an error, so it says so rather than hiding.
 */
export function ModpackCard({
  kind,
  id,
  name,
  running,
}: {
  kind: JobEntryKind;
  id: string;
  name: string;
  running: boolean;
}) {
  const pack = useModpack(kind, id);
  const update = useUpdateModpack(kind, id);
  const remove = useRemoveModpack(kind, id);
  const [confirmRemove, setConfirmRemove] = useState(false);

  if (pack.isLoading) return null;
  if (!pack.data) {
    return (
      <SideCard title={m['content.modpack.title']()}>
        <p className="text-xs text-muted-foreground">
          {m['content.modpack.none']()}
        </p>
      </SideCard>
    );
  }

  const installed = pack.data;
  const loader = installed.loader
    ? `${installed.loader}${installed.loaderVersion ? ` ${installed.loaderVersion}` : ''}`
    : 'vanilla';
  // A file-sourced pack has no catalogue behind it, so there is nothing to
  // update against — the daemon refuses it, and the button should not offer it.
  const updatable = installed.projectId !== '';
  const busy = update.isPending || remove.isPending;

  return (
    <SideCard title={m['content.modpack.title']()}>
      <div className="space-y-3">
        <div className="flex items-start gap-2.5">
          {installed.iconUrl ? (
            <img
              src={installed.iconUrl}
              alt=""
              className="size-9 shrink-0 border border-border object-cover"
            />
          ) : (
            <PackageIcon className="size-9 shrink-0 p-2 text-muted-foreground" />
          )}
          <div className="min-w-0">
            <p className="truncate text-sm font-medium">{installed.name}</p>
            <p className="truncate text-xs text-muted-foreground">
              {installed.versionNumber || m['content.modpack.from_file']()}
            </p>
          </div>
        </div>

        <div className="divide-y divide-border">
          <Stat label={m['app.label.game']()} value={installed.gameVersion} />
          <Stat label={m['app.label.loader']()} value={loader} />
          <Stat
            label={m['domain.kind.mods']()}
            value={installed.files.length}
          />
          <Stat
            label={m['content.modpack.pack_files']()}
            value={installed.overrides.length}
          />
        </div>

        {update.progress && (
          <ProvisionProgressView progress={update.progress} />
        )}
        <WarningNotice warnings={update.data?.warnings} />

        <div className="flex gap-2">
          {updatable && (
            <Button
              size="sm"
              variant="outline"
              disabled={busy || running}
              onClick={() =>
                update
                  .mutateAsync({})
                  .catch((error) => toast.error(errorMessage(error)))
              }
            >
              {m['content.modpack.update']()}
            </Button>
          )}
          <Button
            size="sm"
            variant="ghost"
            disabled={busy || running}
            onClick={() => setConfirmRemove(true)}
          >
            {m['content.modpack.remove.action']()}
          </Button>
        </div>
      </div>

      <ConfirmDialog
        open={confirmRemove}
        onOpenChange={setConfirmRemove}
        title={m['content.modpack.remove.title']({ name: installed.name })}
        description={m['content.modpack.remove.body']({ entry: name })}
        confirmLabel={m['content.modpack.remove.action']()}
        destructive
        onConfirm={() =>
          remove
            .mutateAsync()
            .then((result) => {
              toast.success(
                m['content.modpack.removed']({
                  files: result.removedFiles,
                  overrides: result.removedOverrides,
                }),
              );
              if (result.kept.length)
                toast.info(
                  m['content.modpack.kept']({ count: result.kept.length }),
                );
            })
            .catch((error) => toast.error(errorMessage(error)))
        }
      />
    </SideCard>
  );
}
