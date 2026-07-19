import {
  CopyIcon,
  FolderOpenIcon,
  GlobeHemisphereWestIcon,
  PlayIcon,
  PlusIcon,
  PowerIcon,
  UploadSimpleIcon,
} from '@phosphor-icons/react';
import { useQueries } from '@tanstack/react-query';
import { useState } from 'react';
import type { ContentKind } from '@/api';
import { type InstanceInfo, system } from '@/api';
import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { entryIcon } from '@/components/icons';
import { Stat, TabCount } from '@/components/page';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { Spinner } from '@/components/ui/spinner';
import { StatusDot } from '@/components/ui/status-dot';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  ContentInstallModal,
  instanceTarget,
} from '@/features/content/install-modal';
import { ContentSection, SideCard, StatCard } from '@/features/entries/detail';
import { instances as mockInstances } from '@/features/entries/mock';
import {
  type LiveResources,
  ResourceCards,
} from '@/features/entries/resource-panel';
import { InstanceSettingsTab } from '@/features/instances/settings-tab';
import { ProfilesPanel } from '@/features/profiles/profiles-panel';
import { agoLabel, bytes, memGb } from '@/lib/format';
import { m } from '@/paraglide/messages.js';
import {
  instanceQueries,
  useInstance,
  useInstanceConfig,
  useInstanceInfo,
  useInstanceLogs,
  useInstanceWorlds,
  useLaunchInstance,
  useStopInstance,
} from '@/queries/instance';
import { useProcessMetrics } from '@/queries/metrics';

export type InstanceTab =
  | 'overview'
  | 'content'
  | 'profiles'
  | 'worlds'
  | 'logs'
  | 'settings';

/** The content kinds an instance takes (see `instance.content.add`). */
export const instanceContentKinds: ContentKind[] = [
  'mod',
  'resource_pack',
  'shader',
  'data_pack',
];

function runningSessions(instance: InstanceInfo): number {
  return (instance.sessions ?? []).filter((s) => s.state === 'running').length;
}

export function InstanceDetailPage({
  id,
  tab,
  onTabChange,
  contentKind,
  onContentKindChange,
}: {
  id: string;
  tab: InstanceTab;
  onTabChange: (tab: InstanceTab) => void;
  contentKind?: ContentKind;
  onContentKindChange: (kind?: ContentKind) => void;
}) {
  const query = useInstance(id);
  const info = useInstanceInfo(id);
  const config = useInstanceConfig(id);
  const worlds = useInstanceWorlds(id);
  const [addingContent, setAddingContent] = useState(false);
  const launch = useLaunchInstance(id);
  const stop = useStopInstance(id);
  // Shared with the content tab's own per-kind queries (cached), just for the
  // headline count.
  const contentLists = useQueries({
    queries: instanceContentKinds.map((k) => instanceQueries.content(id, k)),
  });
  const contentCount = contentLists.reduce(
    (n, q) => n + (q.data?.items.length ?? 0),
    0,
  );

  const instance = query.data;
  const sessions = instance ? runningSessions(instance) : 0;
  const running = sessions > 0;
  const liveSession = (instance?.sessions ?? []).find(
    (s) => s.state === 'running',
  );
  const metrics = useProcessMetrics(liveSession?.id ?? null);

  const memoryLimitGb = (() => {
    const value = config.data?.find((e) => e.key === 'memory')?.value;
    return value ? memGb(value) : 4;
  })();

  if (query.isPending) {
    return (
      <div className="space-y-4 p-6">
        <Bone className="h-8 w-64" />
        <Bone className="h-40" />
      </div>
    );
  }

  if (!instance) {
    return (
      <div className="p-6">
        <Empty>{m['instances.missing']()}</Empty>
      </div>
    );
  }

  const live: LiveResources = {
    running,
    memoryLimitGb,
    diskBytes: info.data?.diskBytes ?? 0,
    series: metrics.series.map((s) => ({
      cpu: s.cpuPct,
      mem: s.memBytes / (1024 * 1024),
    })),
  };

  const openFolder = () => {
    if (info.data) void system.openPath(info.data.entryDir);
  };

  // Content and profiles are not wired yet — they render over a mock instance,
  // as the server detail's content tab does (see `serverTarget(mockServers[0])`).
  const mock = mockInstances[0];
  const worldNames = worlds.data ?? [];

  return (
    <div className="flex h-full flex-col">
      <DetailHero
        parentLabel={m['nav.library']()}
        parentTo="/instances"
        icon={entryIcon('instance')}
        name={instance.name}
        badges={
          <>
            <Badge variant="secondary" className="uppercase">
              {instance.flavor}
            </Badge>
            <Badge variant="outline" className="font-mono">
              {instance.gameVersion}
            </Badge>
            {running && (
              <Badge variant="secondary" className="gap-1.5">
                <StatusDot tone="on" />
                {m['entry.sessions_running']({ count: sessions })}
              </Badge>
            )}
          </>
        }
        actions={
          <>
            <Button
              variant="outline"
              size="icon"
              aria-label={m['detail.open_folder']()}
              disabled={!info.data}
              onClick={openFolder}
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
                title={m['entry.stop_title']({ name: instance.name })}
                description={
                  sessions > 1
                    ? m['entry.stop_sessions_description']({ count: sessions })
                    : m['entry.stop_instance_description']()
                }
                confirmLabel={m['action.stop']()}
                onConfirm={() => stop.mutate({})}
              />
            ) : (
              <Button
                data-icon="inline-start"
                disabled={launch.isPending}
                className="bg-ember text-ember-foreground hover:bg-ember/90"
                onClick={() => launch.mutate({})}
              >
                {launch.isPending ? <Spinner /> : <PlayIcon weight="fill" />}
                {m['action.play']()}
              </Button>
            )}
          </>
        }
      />

      <Tabs
        value={tab}
        onValueChange={(value) => onTabChange(value as InstanceTab)}
        className="min-h-0 flex-1 gap-0 p-0"
      >
        <TabsList variant="line" className="h-auto gap-6 px-5">
          <TabsTrigger value="overview">{m['tab.overview']()}</TabsTrigger>
          <TabsTrigger value="content">
            {m['tab.content']()}
            <TabCount n={contentCount} />
          </TabsTrigger>
          <TabsTrigger value="profiles">
            {m['profiles.tab']()}
            <TabCount n={mock.profiles.length} />
          </TabsTrigger>
          <TabsTrigger value="worlds">
            {m['tab.worlds']()}
            <TabCount n={worldNames.length} />
          </TabsTrigger>
          <TabsTrigger value="logs">{m['tab.logs']()}</TabsTrigger>
          <TabsTrigger value="settings">{m['tab.settings']()}</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" keepMounted className="flex flex-col p-5">
          <div className="grid flex-1 gap-6 lg:grid-cols-[1fr_260px]">
            <div className="flex flex-col gap-5">
              <p className="max-w-2xl text-sm leading-relaxed text-foreground/90">
                {m['entry.overview_summary']({
                  flavor: instance.flavor,
                  version: instance.gameVersion,
                  mods: contentCount,
                  worlds: worldNames.length,
                })}
              </p>
              <div className="grid grid-cols-3 gap-3">
                <StatCard value={contentCount} label={m['label.content']()} />
                <StatCard
                  value={worldNames.length}
                  label={m['label.worlds']()}
                />
                <StatCard
                  value={memoryLimitGb ? `${memoryLimitGb}G` : '—'}
                  label={m['label.memory']()}
                />
              </div>
              <ResourceCards live={live} />
            </div>

            <div className="space-y-4">
              <SideCard title={m['label.details']()}>
                <div className="divide-y divide-border">
                  <Stat label={m['label.loader']()} value={instance.flavor} />
                  <Stat
                    label={m['label.version']()}
                    value={instance.gameVersion}
                  />
                  <Stat label={m['label.java']()} value={instance.javaMajor} />
                  <Stat
                    label={m['label.created']()}
                    value={agoLabel(instance.createdUnix)}
                  />
                  <Stat
                    label={m['label.disk']()}
                    value={
                      info.data?.diskBytes != null
                        ? bytes(info.data.diskBytes)
                        : '—'
                    }
                  />
                </div>
              </SideCard>
              <SideCard title={m['detail.quick_actions']()}>
                <div className="flex flex-col gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="justify-start"
                    data-icon="inline-start"
                    disabled={!info.data}
                    onClick={openFolder}
                  >
                    <FolderOpenIcon />
                    {m['detail.open_folder']()}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="justify-start"
                    data-icon="inline-start"
                    disabled
                  >
                    <CopyIcon />
                    {m['detail.duplicate']()}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="justify-start"
                    data-icon="inline-start"
                    disabled
                  >
                    <UploadSimpleIcon />
                    {m['detail.export']()}
                  </Button>
                </div>
              </SideCard>
            </div>
          </div>
        </TabsContent>

        <TabsContent value="content" className="p-5">
          <ContentSection
            entry={{
              kind: 'instance',
              id,
              flavor: instance.flavor,
              gameVersion: instance.gameVersion,
            }}
            kinds={instanceContentKinds}
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

        <TabsContent value="profiles" className="p-5">
          <ProfilesPanel inst={mock} />
        </TabsContent>

        <TabsContent value="worlds" className="p-5">
          {worlds.isPending ? (
            <div className="space-y-2">
              <Bone className="h-10" />
              <Bone className="h-10" />
            </div>
          ) : worldNames.length === 0 ? (
            <Empty>{m['detail.no_worlds']()}</Empty>
          ) : (
            <div className="divide-y divide-border border border-border">
              {worldNames.map((w) => (
                <div key={w} className="flex items-center gap-3 px-3 py-2.5">
                  <GlobeHemisphereWestIcon className="size-4 text-muted-foreground" />
                  <span className="text-sm">{w}</span>
                </div>
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="logs" className="flex min-h-0 flex-col p-5">
          <InstanceLogsTab id={id} running={running} name={instance.name} />
        </TabsContent>

        <TabsContent value="settings" className="p-5">
          <InstanceSettingsTab
            instance={instance}
            config={config.data}
            running={running}
          />
        </TabsContent>
      </Tabs>

      <ContentInstallModal
        entry={instanceTarget(instance)}
        open={addingContent}
        onOpenChange={setAddingContent}
      />
    </div>
  );
}

/** The newest running session's captured output, followed while it runs. */
function InstanceLogsTab({
  id,
  running,
  name,
}: {
  id: string;
  running: boolean;
  name: string;
}) {
  const logs = useInstanceLogs(id, { follow: running, tail: 500 });

  if (logs.lines.length === 0) {
    return <Empty className="h-full">{m['detail.logs_empty']()}</Empty>;
  }

  return (
    <div className="min-h-0 flex-1 space-y-0.5 overflow-y-auto border border-border bg-card p-3 font-mono text-[11px] wrap-break-word whitespace-pre-wrap text-muted-foreground">
      <span className="sr-only">{name}</span>
      {logs.lines.map((entry, index) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: log lines have no stable id.
        <div key={index}>{entry.line}</div>
      ))}
    </div>
  );
}
