import { ArrowClockwiseIcon, DownloadSimpleIcon } from '@phosphor-icons/react';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';

import { Markdown } from '@/components/markdown';
import { Button } from '@/components/ui/button';
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldLabel,
} from '@/components/ui/field';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { appQueries } from '@/queries/app';
import { useInstallUpdate, useUpdateCheck } from '@/queries/update';

/**
 * Self-update. The check runs only when asked — it reaches the network, and an
 * update is not something to nag about before the user looks.
 */
export function UpdatePanel() {
  const app = useQuery(appQueries.info());
  const [asked, setAsked] = useState(false);
  const check = useUpdateCheck(asked);
  const install = useInstallUpdate();

  const update = check.data;
  const version = app.data?.version ?? '';

  return (
    <>
      <Field orientation="horizontal">
        <FieldContent>
          <FieldLabel className="font-normal">
            {m['settings.update.current']({ version })}
          </FieldLabel>
          {asked && !check.isFetching && (
            <FieldDescription>
              {check.isError
                ? m['settings.update.check_failed']()
                : update
                  ? m['settings.update.available']({ version: update.version })
                  : m['settings.update.up_to_date']()}
            </FieldDescription>
          )}
        </FieldContent>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            data-icon="inline-start"
            onClick={() => {
              setAsked(true);
              void check.refetch();
            }}
            disabled={check.isFetching}
          >
            <ArrowClockwiseIcon
              className={cn('size-4', check.isFetching && 'animate-spin')}
            />
            {check.isFetching
              ? m['settings.update.checking']()
              : m['settings.update.check']()}
          </Button>
          {update && (
            <Button
              size="sm"
              data-icon="inline-start"
              onClick={() =>
                install.mutate(undefined, {
                  // Success never resolves here — the app restarts into the
                  // new build — so only the failure path needs reporting.
                  onError: (e) =>
                    toast.error(m['settings.update.failed'](), {
                      description: String(e),
                    }),
                })
              }
              disabled={install.isPending}
            >
              <DownloadSimpleIcon className="size-4" />
              {install.isPending
                ? m['settings.update.installing']()
                : m['settings.update.install']()}
            </Button>
          )}
        </div>
      </Field>
      {update?.notes && (
        <Field>
          <FieldLabel className="font-normal">
            {m['settings.update.notes']()}
          </FieldLabel>
          <Markdown>{update.notes}</Markdown>
        </Field>
      )}
    </>
  );
}
