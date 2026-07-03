import { createFileRoute } from "@tanstack/react-router"

import { getAppInfo, type AppInfo } from "@/lib/api"
import { ipcKeys, useAppInfo } from "@/hooks/use-ipc"
import { PageHeader } from "@/components/page-header"
import { Panel } from "@/components/ui/panel"

export const Route = createFileRoute("/")({
  loader: ({ context: { queryClient } }) =>
    queryClient.ensureQueryData({
      queryKey: ipcKeys.appInfo,
      queryFn: getAppInfo,
    }),
  component: HomePage,
})

function HomePage() {
  const { data, isPending, error } = useAppInfo()

  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        eyebrow="overview"
        title="Connected to the daemon"
        description="The desktop shell drives hestiad over the local socket — the same engine the CLI and TUI use. Identity is read from the native channel."
      />

      <Panel label="app.info">
        {error ? (
          <p className="text-sm text-destructive">{error.message}</p>
        ) : isPending ? (
          <p className="font-mono text-xs text-muted-foreground">loading…</p>
        ) : (
          <dl className="flex flex-col">
            {(Object.entries(data) as [keyof AppInfo, string][]).map(
              ([key, value]) => (
                <div
                  key={key}
                  className="grid grid-cols-[7rem_1fr] items-baseline gap-x-6 border-border py-2 not-first:border-t"
                >
                  <dt className="font-mono text-[11px] tracking-wide text-muted-foreground">
                    {key}
                  </dt>
                  <dd className="truncate font-mono text-xs text-foreground">
                    {value}
                  </dd>
                </div>
              )
            )}
          </dl>
        )}
      </Panel>
    </div>
  )
}
