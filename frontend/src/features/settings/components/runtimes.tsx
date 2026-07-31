import {
  CoffeeIcon,
  DownloadSimpleIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { useMutation, useQuery } from '@tanstack/react-query';

import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { Field, FieldDescription } from '@/components/ui/field';
import { Progress } from '@/components/ui/progress';
import { m } from '@/paraglide/messages.js';
import { javaMutations, javaQueries } from '@/queries/java';
import { useJobMutation } from '@/queries/jobs';

/**
 * Every runtime Hestia offers, installed or not, as one list: what is on disk
 * carries its release and an uninstall, what is not carries an install. The
 * previous strip of version buttons could not say which of the two a number was.
 */
export function RuntimeList() {
  const runtimesQuery = useQuery(javaQueries.runtimes());
  const releasesQuery = useQuery(javaQueries.releases());
  const install = useJobMutation(javaMutations.install());
  const uninstall = useMutation(javaMutations.uninstall());

  const runtimes = runtimesQuery.data ?? [];
  const installed = new Map(runtimes.map((rt) => [rt.major, rt]));
  const majors = [
    ...new Set([
      ...runtimes.map((rt) => rt.major),
      ...(releasesQuery.data ?? []).map((release) => release.major),
    ]),
  ].sort((a, b) => b - a);
  const lts = new Set(
    (releasesQuery.data ?? []).filter((r) => r.lts).map((r) => r.major),
  );

  if (runtimesQuery.isPending) return <Bone className="h-32" />;

  return (
    <Field>
      <FieldDescription>{m['settings.java.runtimes_hint']()}</FieldDescription>
      <div className="divide-y divide-border border border-border">
        {majors.map((major) => {
          const runtime = installed.get(major);
          const installing =
            install.isPending && install.variables?.major === major;
          const progress = installing ? install.progress : null;

          return (
            <div key={major} className="flex items-center gap-3 px-3 py-2">
              <CoffeeIcon
                className={
                  runtime
                    ? 'size-4 shrink-0 text-foreground/70'
                    : 'size-4 shrink-0 text-muted-foreground/50'
                }
              />
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2 text-sm">
                  {m['settings.java.runtime_name']({ major })}
                  {lts.has(major) && <Badge variant="outline">LTS</Badge>}
                  {runtime?.inUse && (
                    <Badge variant="secondary">{m['settings.in_use']()}</Badge>
                  )}
                </div>
                {runtime ? (
                  <div className="font-mono text-[11px] text-muted-foreground">
                    {runtime.vendor} · {runtime.releaseName}
                  </div>
                ) : (
                  <div className="text-[11px] text-muted-foreground">
                    {m['settings.java.not_installed']()}
                  </div>
                )}
                {progress && (
                  <Progress
                    className="mt-1.5 h-1"
                    value={
                      progress.total > 0
                        ? (progress.current / progress.total) * 100
                        : 0
                    }
                  />
                )}
              </div>

              {runtime ? (
                <ConfirmDialog
                  trigger={
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      aria-label={m['settings.java.uninstall']()}
                      disabled={runtime.inUse || uninstall.isPending}
                    >
                      <TrashIcon className="size-4" />
                    </Button>
                  }
                  title={m['settings.java.uninstall_title']()}
                  description={m['settings.java.uninstall_description']({
                    name: `${runtime.vendor} ${runtime.major}`,
                  })}
                  destructive
                  confirmLabel={m['app.action.uninstall']()}
                  onConfirm={() => uninstall.mutate(runtime.major)}
                />
              ) : (
                <Button
                  variant="outline"
                  size="xs"
                  data-icon="inline-start"
                  disabled={install.isPending}
                  onClick={() => install.mutate({ major })}
                >
                  <DownloadSimpleIcon className="size-3.5" />
                  {installing
                    ? m['settings.java.installing']()
                    : m['app.action.install']()}
                </Button>
              )}
            </div>
          );
        })}
      </div>
    </Field>
  );
}
