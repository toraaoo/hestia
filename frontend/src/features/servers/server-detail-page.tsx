import {
  FolderOpenIcon,
  PlayIcon,
  PlusIcon,
  PowerIcon,
} from '@phosphor-icons/react';
import { useMemo, useState } from 'react';

import { type ServerInfo, system } from '@/api';
import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { entryIcon } from '@/components/icons';
import { Stat, TabCount } from '@/components/page';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { StatusDot } from '@/components/ui/status-dot';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  ContentInstallModal,
  serverTarget,
} from '@/features/content/install-modal';
import { ContentSection, SideCard, StatCard } from '@/features/entries/detail';
import { servers as mockServers } from '@/features/entries/mock';
import {
  type LiveResources,
  ResourceCards,
} from '@/features/entries/resource-panel';
import { ServerBackupsTab } from '@/features/servers/backups-tab';
import { ServerConsoleTab } from '@/features/servers/console-tab';
import { ServerSettingsTab } from '@/features/servers/settings-tab';
import { agoLabel, bytes, memGb } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';
import { m } from '@/paraglide/messages.js';
import { useProcessMetrics } from '@/queries/metrics';
import {
  useServer,
  useServerConfig,
  useServerInfo,
  useServerPing,
  useStartServer,
  useStopServer,
} from '@/queries/server';

export type ServerTab =
  | 'overview'
  | 'console'
  | 'content'
  | 'backups'
  | 'settings';

/** The content kinds a server takes (see `server.content.add`). */
export const serverContentKinds: ContentKind[] = ['mod', 'datapack'];

function isRunning(server: ServerInfo): boolean {
  return server.process?.state === 'running';
}

export function ServerDetailPage({
  id,
  tab,
  onTabChange,
  contentKind,
  onContentKindChange,
}: {
  id: string;
  tab: ServerTab;
  onTabChange: (tab: ServerTab) => void;
  contentKind?: ContentKind;
  onContentKindChange: (kind?: ContentKind) => void;
}) {
  const query = useServer(id);
  const info = useServerInfo(id);
  const config = useServerConfig(id);
  const [addingContent, setAddingContent] = useState(false);
  const start = useStartServer(id);
  const stop = useStopServer(id);

  const server = query.data;
  const running = server ? isRunning(server) : false;
  const ping = useServerPing(id, running);
  const metrics = useProcessMetrics(server?.process?.id ?? null);

  const memoryLimitGb = useMemo(() => {
    const value = config.data?.find((e) => e.key === 'memory')?.value;
    return value ? memGb(value) : 4;
  }, [config.data]);

  if (query.isPending) {
    return (
      <div className="space-y-4 p-6">
        <Bone className="h-8 w-64" />
        <Bone className="h-40" />
      </div>
    );
  }

  if (!server) {
    return (
      <div className="p-6">
        <Empty>{m['servers.missing']()}</Empty>
      </div>
    );
  }

  const statusTone = !server.ready ? 'warn' : running ? 'on' : 'off';
  const statusLabel = !server.ready
    ? m['status.preparing']()
    : running
      ? m['status.online']()
      : m['status.stopped']();

  const live: LiveResources = {
    running,
    memoryLimitGb,
    diskBytes: info.data?.diskBytes ?? 0,
    series: metrics.series.map((s) => ({
      cpu: s.cpuPct,
      mem: s.memBytes / (1024 * 1024),
    })),
  };

  const contentItems = mockServers[0].content;

  return (
    <div className="flex h-full flex-col">
      <DetailHero
        parentLabel={m['nav.servers']()}
        parentTo="/servers"
        icon={entryIcon('server')}
        name={server.name}
        badges={
          <>
            <Badge variant="secondary" className="uppercase">
              {server.flavor}
            </Badge>
            <Badge variant="outline" className="font-mono">
              {server.gameVersion}
            </Badge>
            <Badge variant="secondary" className="gap-1.5">
              <StatusDot tone={statusTone} />
              {statusLabel}
            </Badge>
          </>
        }
        actions={
          <>
            <Button
              variant="outline"
              size="icon"
              aria-label={m['detail.open_folder']()}
              disabled={!info.data}
              onClick={() => {
                if (info.data) void system.openPath(info.data.entryDir);
              }}
            >
              <FolderOpenIcon className="size-4" />
            </Button>
            {running ? (
              <ConfirmDialog
                trigger={
                  <Button
                    variant="outline"
                    data-icon="inline-start"
                    disabled={stop.isPending}
                  >
                    <PowerIcon weight="bold" />
                    {m['action.stop']()}
                  </Button>
                }
                title={m['entry.stop_title']({ name: server.name })}
                description={m['entry.stop_server_description']()}
                confirmLabel={m['action.stop']()}
                onConfirm={() => stop.mutate()}
              />
            ) : (
              <Button
                disabled={!server.ready || start.isPending}
                data-icon="inline-start"
                className="bg-ember text-ember-foreground hover:bg-ember/90"
                onClick={() => start.mutate()}
              >
                <PlayIcon weight="fill" />
                {m['action.start']()}
              </Button>
            )}
          </>
        }
      />

      <Tabs
        value={tab}
        onValueChange={(value) => onTabChange(value as ServerTab)}
        className="min-h-0 flex-1 gap-0 p-0"
      >
        <TabsList variant="line" className="h-auto gap-6 px-5">
          <TabsTrigger value="overview">{m['tab.overview']()}</TabsTrigger>
          <TabsTrigger value="console">{m['tab.console']()}</TabsTrigger>
          <TabsTrigger value="content">
            {m['tab.content']()}
            <TabCount n={contentItems.length} />
          </TabsTrigger>
          <TabsTrigger value="backups">{m['tab.backups']()}</TabsTrigger>
          <TabsTrigger value="settings">{m['tab.settings']()}</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="flex flex-col p-5">
          <div className="grid flex-1 gap-6 lg:grid-cols-[1fr_260px]">
            <div className="flex flex-col gap-5">
              {ping.data?.motd && (
                <p className="max-w-2xl text-sm leading-relaxed text-foreground/90">
                  {ping.data.motd}
                </p>
              )}
              <div className="grid grid-cols-3 gap-3">
                <StatCard
                  value={
                    ping.data
                      ? `${ping.data.playersOnline}/${ping.data.playersMax}`
                      : '—'
                  }
                  label={m['label.players']()}
                />
                <StatCard
                  value={memoryLimitGb ? `${memoryLimitGb}G` : '—'}
                  label={m['label.memory']()}
                />
                <StatCard
                  value={
                    info.data?.diskBytes != null
                      ? bytes(info.data.diskBytes)
                      : '—'
                  }
                  label={m['label.disk']()}
                />
              </div>
              <ResourceCards id={server.id} live={live} />
            </div>

            <div className="space-y-4">
              <SideCard title={m['label.details']()}>
                <div className="divide-y divide-border">
                  <Stat
                    label={m['label.address']()}
                    value={`localhost:${server.gamePort ?? '—'}`}
                  />
                  <Stat label={m['label.loader']()} value={server.flavor} />
                  <Stat
                    label={m['label.version']()}
                    value={server.gameVersion}
                  />
                  <Stat label={m['label.java']()} value={server.javaMajor} />
                  <Stat
                    label={m['label.created']()}
                    value={agoLabel(server.createdUnix)}
                  />
                </div>
              </SideCard>
            </div>
          </div>
        </TabsContent>

        <TabsContent value="console" className="flex min-h-0 flex-col p-5">
          <ServerConsoleTab id={id} running={running} name={server.name} />
        </TabsContent>

        <TabsContent value="content" className="p-5">
          <ContentSection
            items={contentItems}
            kinds={serverContentKinds}
            kind={contentKind}
            onKindChange={onContentKindChange}
            action={
              <Button
                size="sm"
                variant="outline"
                data-icon="inline-start"
                onClick={() => setAddingContent(true)}
              >
                <PlusIcon weight="bold" />
                {m['content.add']()}
              </Button>
            }
          />
        </TabsContent>

        <TabsContent value="backups" className="p-5">
          <ServerBackupsTab id={id} running={running} config={config.data} />
        </TabsContent>

        <TabsContent value="settings" className="p-5">
          <ServerSettingsTab
            server={server}
            config={config.data}
            running={running}
          />
        </TabsContent>
      </Tabs>

      <ContentInstallModal
        entry={serverTarget(mockServers[0])}
        open={addingContent}
        onOpenChange={setAddingContent}
      />
    </div>
  );
}
