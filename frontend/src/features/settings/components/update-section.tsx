import { ArrowClockwiseIcon, DownloadSimpleIcon } from '@phosphor-icons/react';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';
import { Markdown } from '@/components/markdown';
import { Button } from '@/components/ui/button';
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from '@/components/ui/field';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { appQueries } from '@/queries/app';
import { useInstallUpdate, useUpdateCheck } from '@/queries/update';

/**
 * Self-update. The check runs only when asked — it reaches the network, and an
 * update is not something to nag about before the user looks.
 */
export function UpdateSection() {
  const app = useQuery(appQueries.info());
  const [asked, setAsked] = useState(false);
  const check = useUpdateCheck(asked);
  const install = useInstallUpdate();

  const update = check.data;
  const version = app.data?.version ?? '';

  return (
    <FieldSet>
      <FieldLegend>{m['update.title']()}</FieldLegend>
      <FieldGroup>
        <Field orientation="horizontal">
          <div className="min-w-0">
            <FieldLabel className="font-normal">
              {m['update.current']({ version })}
            </FieldLabel>
            {asked && !check.isFetching && (
              <FieldDescription>
                {check.isError
                  ? m['update.check_failed']()
                  : update
                    ? m['update.available']({ version: update.version })
                    : m['update.up_to_date']()}
              </FieldDescription>
            )}
          </div>
          <div className="ml-auto flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setAsked(true);
                void check.refetch();
              }}
              disabled={check.isFetching}
            >
              <ArrowClockwiseIcon
                className={cn('size-4', check.isFetching && 'animate-spin')}
              />
              {check.isFetching ? m['update.checking']() : m['update.check']()}
            </Button>
            {update && (
              <Button
                size="sm"
                onClick={() =>
                  install.mutate(undefined, {
                    // Success never resolves here — the app restarts into the
                    // new build — so only the failure path needs reporting.
                    onError: (e) =>
                      toast.error(m['update.failed'](), {
                        description: String(e),
                      }),
                  })
                }
                disabled={install.isPending}
              >
                <DownloadSimpleIcon className="size-4" />
                {install.isPending
                  ? m['update.installing']()
                  : m['update.install']()}
              </Button>
            )}
          </div>
        </Field>
        {update?.notes && (
          <Field>
            <FieldLabel className="font-normal">
              {m['update.notes']()}
            </FieldLabel>
            <Markdown>{update.notes}</Markdown>
          </Field>
        )}
      </FieldGroup>
    </FieldSet>
  );
}
