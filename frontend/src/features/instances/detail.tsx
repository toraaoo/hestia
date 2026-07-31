import {
  FolderOpenIcon,
  PlayIcon,
  PlusIcon,
  UploadSimpleIcon,
} from '@phosphor-icons/react';
import { useMutation, useQueries, useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';
import type { ContentKind } from '@/api';
import { errorMessage, system } from '@/api';
import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { entryIcon } from '@/components/icons';
import { Stat, TabCount } from '@/components/page';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Spinner } from '@/components/ui/spinner';
import { StatusDot } from '@/components/ui/status-dot';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  ContentInstallModal,
  instanceTarget,
} from '@/features/content/install';
import { EntryIconMenu } from '@/features/entries/components/icon-menu';
import {
  type LiveResources,
  ResourceCards,
} from '@/features/entries/components/resource-panel';
import {
  ContentSection,
  ModpackCard,
  SideCard,
  StatCard,
} from '@/features/entries/detail';
import { SessionList } from '@/features/instances/components/session-list';
import { WorldRow } from '@/features/instances/components/world-row';
import { ExportInstanceModal } from '@/features/instances/export-modal';
import { useLaunchModal } from '@/features/instances/launch-modal';
import { InstanceRunControl } from '@/features/instances/run-control';
import { InstanceLogsTab } from '@/features/instances/tabs/logs';
import { InstanceServersTab } from '@/features/instances/tabs/servers';
import { InstanceSettingsTab } from '@/features/instances/tabs/settings';
import { ProfilesPanel } from '@/features/profiles/panel';
import { agoLabel, bytes, memGb, uptime } from '@/lib/format';
import { supportsQuickPlay } from '@/lib/quick-play';
import { runningSessions } from '@/lib/sessions';
import { toastWarnings } from '@/lib/warnings';
import { m } from '@/paraglide/messages.js';
import {
  instanceMutations,
  instanceQueries,
  useInstance,
} from '@/queries/instance';
import { useJobMutation } from '@/queries/jobs';
import { useProcessMetrics } from '@/queries/metrics';

export type InstanceTab =
  | 'overview'
  | 'content'
  | 'profiles'
  | 'worlds'
  | 'servers'
  | 'logs'
  | 'settings';

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
  const info = useQuery(instanceQueries.info(id));
  const config = useQuery(instanceQueries.config(id));
  const worlds = useQuery(instanceQueries.worlds(id));
  const profiles = useQuery(instanceQueries.profiles(id));
  const [addingContent, setAddingContent] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [focus, setFocus] = useState<string | null>(null);
  const [logSession, setLogSession] = useState<string | null>(null);
  const { launch, isLaunching } = useLaunchModal();
  // Joining a world or a server is the same launch job, carrying what to join.
  const launchQuick = useJobMutation(instanceMutations.launchQuick());
  const stop = useMutation(instanceMutations.stop(id));
  const instance = query.data;
  const accepts = instance?.accepts ?? [];
  // Shared with the content tab's own per-kind queries (cached), just for the
  // headline count.
  const contentLists = useQueries({
    queries: accepts.map((k) => instanceQueries.content(id, k)),
  });
  const contentCount = contentLists.reduce(
    (n, q) => n + (q.data?.items.length ?? 0),
    0,
  );

  const sessions = instance ? runningSessions(instance) : [];
  const running = sessions.length > 0;
  const liveSession =
    sessions.find((s) => s.id === focus) ?? sessions[sessions.length - 1];
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
        <Empty>{m['instance.missing']()}</Empty>
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
    if (info.data)
      system.openPath(info.data.entryDir).catch((error: Error) => {
        toast.error(errorMessage(error));
      });
  };

  const worldList = worlds.data ?? [];
  const joinable = supportsQuickPlay(instance.gameVersion);

  return (
    <div className="flex h-full flex-col">
      <DetailHero
        parentLabel={m['app.nav.library']()}
        parentTo="/instances"
        icon={entryIcon('instance')}
        iconUrl={instance.iconUrl}
        iconAction={<EntryIconMenu id={id} />}
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
                {m['entry.sessions_running']({ count: sessions.length })}
              </Badge>
            )}
          </>
        }
        actions={
          <>
            <Button
              variant="outline"
              size="icon"
              aria-label={m['app.action.open_folder']()}
              disabled={!info.data}
              onClick={openFolder}
            >
              <FolderOpenIcon className="size-4" />
            </Button>
            {running ? (
              <InstanceRunControl
                name={instance.name}
                sessions={sessions}
                busy={stop.isPending}
                launching={isLaunching(id)}
                onNewSession={() => launch(instance, { newSession: true })}
                onStop={(session) => stop.mutate({ session })}
              />
            ) : (
              <Button
                data-icon="inline-start"
                disabled={isLaunching(id)}
                className="bg-ember text-ember-foreground hover:bg-ember/90"
                onClick={() => launch(instance)}
              >
                {isLaunching(id) ? <Spinner /> : <PlayIcon weight="fill" />}
                {m['app.action.play']()}
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
          <TabsTrigger value="overview">
            {m['app.label.overview']()}
          </TabsTrigger>
          <TabsTrigger value="content">
            {m['app.label.content']()}
            <TabCount n={contentCount} />
          </TabsTrigger>
          <TabsTrigger value="profiles">
            {m['app.nav.profiles']()}
            <TabCount n={profiles.data?.profiles.length ?? 0} />
          </TabsTrigger>
          <TabsTrigger value="worlds">
            {m['app.label.worlds']()}
            <TabCount n={worldList.length} />
          </TabsTrigger>
          <TabsTrigger value="servers">{m['app.label.servers']()}</TabsTrigger>
          <TabsTrigger value="logs">{m['app.label.logs']()}</TabsTrigger>
          <TabsTrigger value="settings">
            {m['app.label.settings']()}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="overview" keepMounted className="flex flex-col p-5">
          <div className="grid flex-1 gap-6 lg:grid-cols-[1fr_260px]">
            <div className="flex flex-col gap-5">
              <p className="max-w-2xl text-sm leading-relaxed text-foreground/90">
                {m['entry.overview_summary']({
                  flavor: instance.flavor,
                  version: instance.gameVersion,
                  mods: contentCount,
                  worlds: worldList.length,
                })}
              </p>
              <div className="grid grid-cols-3 gap-3">
                <StatCard
                  value={contentCount}
                  label={m['app.label.content']()}
                />
                <StatCard
                  value={worldList.length}
                  label={m['app.label.worlds']()}
                />
                <StatCard
                  value={memoryLimitGb ? `${memoryLimitGb}G` : '—'}
                  label={m['app.label.memory']()}
                />
              </div>
              <ResourceCards live={live} />
              {running && (
                <section className="space-y-2">
                  <h2 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
                    {m['app.label.sessions']()}
                  </h2>
                  <SessionList
                    sessions={sessions}
                    focused={liveSession?.id}
                    onFocus={setFocus}
                    onLogs={(session) => {
                      setLogSession(session);
                      onTabChange('logs');
                    }}
                    onStop={(session) => stop.mutate({ session })}
                    stopping={stop.isPending}
                  />
                </section>
              )}
            </div>

            <div className="space-y-4">
              <SideCard title={m['app.label.details']()}>
                <div className="divide-y divide-border">
                  <Stat
                    label={m['app.label.loader']()}
                    value={instance.flavor}
                  />
                  <Stat
                    label={m['app.label.version']()}
                    value={instance.gameVersion}
                  />
                  <Stat
                    label={m['app.label.java']()}
                    value={instance.javaMajor}
                  />
                  <Stat
                    label={m['app.label.created']()}
                    value={agoLabel(instance.createdUnix)}
                  />
                  <Stat
                    label={m['app.label.last_played']()}
                    value={
                      info.data?.lastPlayedUnix
                        ? agoLabel(info.data.lastPlayedUnix)
                        : '—'
                    }
                  />
                  <Stat
                    label={m['app.label.playtime']()}
                    value={
                      info.data?.playtimeSeconds
                        ? uptime(info.data.playtimeSeconds)
                        : '—'
                    }
                  />
                  <Stat
                    label={m['app.label.disk']()}
                    value={
                      info.data?.diskBytes != null
                        ? bytes(info.data.diskBytes)
                        : '—'
                    }
                  />
                </div>
              </SideCard>
              <ModpackCard
                kind="instance"
                id={instance.id}
                name={instance.name}
                running={!!live}
              />
              <SideCard title={m['entry.quick_actions']()}>
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
                    {m['app.action.open_folder']()}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="justify-start"
                    data-icon="inline-start"
                    onClick={() => setExporting(true)}
                  >
                    <UploadSimpleIcon />
                    {m['app.action.export']()}
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
            kinds={accepts}
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
          <ProfilesPanel instance={instance} running={running} />
        </TabsContent>

        <TabsContent value="worlds" className="p-5">
          {worlds.isPending ? (
            <div className="space-y-2">
              <Bone className="h-10" />
              <Bone className="h-10" />
            </div>
          ) : worldList.length === 0 ? (
            <Empty>{m['instance.no_worlds']()}</Empty>
          ) : (
            <div className="divide-y divide-border border border-border">
              {worldList.map((world) => (
                <WorldRow
                  key={world.folder}
                  world={world}
                  playing={
                    launchQuick.isPending &&
                    launchQuick.variables?.quickPlay.target === world.folder
                  }
                  disabledReason={
                    joinable
                      ? undefined
                      : m['instance.quick_play.unsupported']()
                  }
                  onPlay={() =>
                    launchQuick.mutate(
                      {
                        id,
                        quickPlay: { kind: 'world', target: world.folder },
                        newSession: running,
                      },
                      { onSuccess: (done) => toastWarnings(done.warnings) },
                    )
                  }
                />
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="servers" className="p-5">
          <InstanceServersTab instance={instance} />
        </TabsContent>

        <TabsContent value="logs" className="flex min-h-0 flex-col p-5">
          <InstanceLogsTab
            id={id}
            name={instance.name}
            sessions={sessions}
            session={logSession}
            onSessionChange={setLogSession}
          />
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
      <ExportInstanceModal
        id={id}
        name={instance.name}
        open={exporting}
        onOpenChange={setExporting}
      />
    </div>
  );
}
