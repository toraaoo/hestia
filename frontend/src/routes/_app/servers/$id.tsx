import {
  FolderOpenIcon,
  PlayIcon,
  PlusIcon,
  PowerIcon,
} from '@phosphor-icons/react';
import { createFileRoute } from '@tanstack/react-router';

import {
  BackupList,
  ContentList,
  DetailHero,
  Empty,
  SideCard,
  StatCard,
} from '@/components/launcher/detail';
import { ServerSettingsForm } from '@/components/launcher/entry-settings';
import { entryIcon } from '@/components/launcher/icons';
import { Stat, TabCount } from '@/components/launcher/page';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { StatusDot } from '@/components/ui/status-dot';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { agoLabel } from '@/lib/format';
import { getServer } from '@/lib/mock';

export const Route = createFileRoute('/_app/servers/$id')({
  component: ServerDetailPage,
});

const consoleLines = [
  '[12:04:21] [Server thread/INFO]: Starting minecraft server version 1.21.4',
  '[12:04:23] [Server thread/INFO]: Preparing level "world"',
  '[12:04:25] [Server thread/INFO]: Done (3.812s)! For help, type "help"',
  '[12:07:02] [Server thread/INFO]: toraaoo joined the game',
  '[12:19:44] [Server thread/INFO]: <toraaoo> anyone near spawn?',
];

function ServerDetailPage() {
  const { id } = Route.useParams();
  const server = getServer(id);

  if (!server) {
    return (
      <div className="p-6">
        <Empty>That server no longer exists.</Empty>
      </div>
    );
  }

  const statusTone = !server.ready ? 'warn' : server.running ? 'on' : 'off';
  const statusLabel = !server.ready
    ? 'Preparing'
    : server.running
      ? 'Online'
      : 'Stopped';

  return (
    <div className="flex min-h-full flex-col">
      <DetailHero
        parentLabel="Servers"
        parentTo="/servers"
        icon={entryIcon('server')}
        name={server.name}
        badges={
          <>
            <Badge variant="secondary" className="uppercase">
              {server.flavor}
            </Badge>
            <Badge variant="outline" className="font-mono">
              {server.game_version}
            </Badge>
            <Badge variant="secondary" className="gap-1.5">
              <StatusDot tone={statusTone} />
              {statusLabel}
            </Badge>
          </>
        }
        actions={
          <>
            <Button variant="outline" size="icon" aria-label="Open folder">
              <FolderOpenIcon className="size-4" />
            </Button>
            {server.running ? (
              <Button variant="outline" data-icon="inline-start">
                <PowerIcon weight="bold" />
                Stop
              </Button>
            ) : (
              <Button
                disabled={!server.ready}
                data-icon="inline-start"
                className="bg-ember text-ember-foreground hover:bg-ember/90"
              >
                <PlayIcon weight="fill" />
                Start
              </Button>
            )}
          </>
        }
      />

      <Tabs defaultValue="overview" className="gap-0 p-0">
        <TabsList
          variant="line"
          className="h-auto gap-4 border-b border-border px-5"
        >
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="console">Console</TabsTrigger>
          <TabsTrigger value="content">
            Content
            <TabCount n={server.content.length} />
          </TabsTrigger>
          <TabsTrigger value="backups">
            Backups
            <TabCount n={server.backups.length} />
          </TabsTrigger>
          <TabsTrigger value="settings">Settings</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="p-5">
          <div className="grid gap-6 lg:grid-cols-[1fr_260px]">
            <div className="space-y-5">
              <p className="max-w-2xl text-sm leading-relaxed text-foreground/90">
                {server.motd}
              </p>
              <div className="grid grid-cols-3 gap-3">
                <StatCard
                  value={`${server.players}/${server.max_players}`}
                  label="Players"
                />
                <StatCard value={server.memory} label="Memory" />
                <StatCard value={server.content.length} label="Content" />
              </div>
            </div>

            <div className="space-y-4">
              <SideCard title="Details">
                <div className="divide-y divide-border">
                  <Stat
                    label="Address"
                    value={`localhost:${server.port ?? '—'}`}
                  />
                  <Stat label="Loader" value={server.flavor} />
                  <Stat label="Version" value={server.game_version} />
                  <Stat label="Java" value={server.java_major} />
                  <Stat label="Created" value={agoLabel(server.created_unix)} />
                </div>
              </SideCard>
              <SideCard title="Backups">
                <p className="text-xs text-muted-foreground">
                  {server.backup_interval
                    ? `Every ${server.backup_interval}, keeping ${server.backup_retention}.`
                    : 'Scheduled backups are off.'}
                </p>
              </SideCard>
            </div>
          </div>
        </TabsContent>

        <TabsContent value="console" className="p-5">
          {server.running ? (
            <div className="flex flex-col gap-2">
              <div className="h-72 space-y-0.5 overflow-y-auto border border-border bg-card p-3 font-mono text-[11px] text-muted-foreground">
                {consoleLines.map((line) => (
                  <div key={line}>{line}</div>
                ))}
              </div>
              <form className="flex gap-2" onSubmit={(e) => e.preventDefault()}>
                <Input
                  placeholder="Enter a server command, e.g. say hello"
                  className="font-mono"
                />
                <Button type="submit" size="sm">
                  Send
                </Button>
              </form>
            </div>
          ) : (
            <Empty>Start the server to open its console.</Empty>
          )}
        </TabsContent>

        <TabsContent value="content" className="p-5">
          <div className="mb-3 flex justify-end">
            <Button size="sm" variant="outline" data-icon="inline-start">
              <PlusIcon weight="bold" />
              Add content
            </Button>
          </div>
          <ContentList items={server.content} />
        </TabsContent>

        <TabsContent value="backups" className="p-5">
          <div className="mb-3 flex items-center justify-between">
            <span className="text-xs text-muted-foreground">
              {server.backup_interval
                ? `Scheduled every ${server.backup_interval}, keeping ${server.backup_retention}`
                : 'Scheduled backups off'}
            </span>
            <Button size="sm" variant="outline" data-icon="inline-start">
              <PlusIcon weight="bold" />
              Create backup
            </Button>
          </div>
          <BackupList backups={server.backups} />
        </TabsContent>

        <TabsContent value="settings" className="p-5">
          <ServerSettingsForm server={server} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
