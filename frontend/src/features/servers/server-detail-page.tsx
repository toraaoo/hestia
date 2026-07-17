import {
  FolderOpenIcon,
  PlayIcon,
  PlusIcon,
  PowerIcon,
} from '@phosphor-icons/react';
import { useState } from 'react';
import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { entryIcon } from '@/components/icons';
import { Stat, TabCount } from '@/components/page';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { Input } from '@/components/ui/input';
import { StatusDot } from '@/components/ui/status-dot';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  ContentInstallModal,
  serverTarget,
} from '@/features/content/install-modal';
import {
  BackupList,
  ContentSection,
  SideCard,
  StatCard,
} from '@/features/entries/detail';
import { getServer } from '@/features/entries/mock';
import { ResourceCards } from '@/features/entries/resource-panel';
import { ServerSettingsForm } from '@/features/entries/settings-forms';
import { agoLabel } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';
import { m } from '@/paraglide/messages.js';

const consoleLines = [
  '[12:04:21] [Server thread/INFO]: Starting minecraft server version 1.21.4',
  '[12:04:23] [Server thread/INFO]: Preparing level "world"',
  '[12:04:25] [Server thread/INFO]: Done (3.812s)! For help, type "help"',
  '[12:07:02] [Server thread/INFO]: toraaoo joined the game',
  '[12:19:44] [Server thread/INFO]: <toraaoo> anyone near spawn?',
];

export type ServerTab =
  | 'overview'
  | 'console'
  | 'content'
  | 'backups'
  | 'settings';

/** The content kinds a server takes (see `server.content.add`). */
export const serverContentKinds: ContentKind[] = ['mod', 'datapack'];

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
  const server = getServer(id);
  const [addingContent, setAddingContent] = useState(false);

  if (!server) {
    return (
      <div className="p-6">
        <Empty>{m['servers.missing']()}</Empty>
      </div>
    );
  }

  const statusTone = !server.ready ? 'warn' : server.running ? 'on' : 'off';
  const statusLabel = !server.ready
    ? m['status.preparing']()
    : server.running
      ? m['status.online']()
      : m['status.stopped']();

  return (
    <div className="flex min-h-full flex-col">
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
            <Button
              variant="outline"
              size="icon"
              aria-label={m['detail.open_folder']()}
            >
              <FolderOpenIcon className="size-4" />
            </Button>
            {server.running ? (
              <ConfirmDialog
                trigger={
                  <Button variant="outline" data-icon="inline-start">
                    <PowerIcon weight="bold" />
                    {m['action.stop']()}
                  </Button>
                }
                title={m['entry.stop_title']({ name: server.name })}
                description={m['entry.stop_server_description']()}
                confirmLabel={m['action.stop']()}
                onConfirm={() => {}}
              />
            ) : (
              <Button
                disabled={!server.ready}
                data-icon="inline-start"
                className="bg-ember text-ember-foreground hover:bg-ember/90"
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
            <TabCount n={server.content.length} />
          </TabsTrigger>
          <TabsTrigger value="backups">
            {m['tab.backups']()}
            <TabCount n={server.backups.length} />
          </TabsTrigger>
          <TabsTrigger value="settings">{m['tab.settings']()}</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="flex flex-col p-5">
          <div className="grid flex-1 gap-6 lg:grid-cols-[1fr_260px]">
            <div className="flex flex-col gap-5">
              <p className="max-w-2xl text-sm leading-relaxed text-foreground/90">
                {server.motd}
              </p>
              <div className="grid grid-cols-3 gap-3">
                <StatCard
                  value={`${server.players}/${server.max_players}`}
                  label={m['label.players']()}
                />
                <StatCard value={server.memory} label={m['label.memory']()} />
                <StatCard
                  value={server.content.length}
                  label={m['label.content']()}
                />
              </div>
              <ResourceCards id={server.id} />
            </div>

            <div className="space-y-4">
              <SideCard title={m['label.details']()}>
                <div className="divide-y divide-border">
                  <Stat
                    label={m['label.address']()}
                    value={`localhost:${server.port ?? '—'}`}
                  />
                  <Stat label={m['label.loader']()} value={server.flavor} />
                  <Stat
                    label={m['label.version']()}
                    value={server.game_version}
                  />
                  <Stat label={m['label.java']()} value={server.java_major} />
                  <Stat
                    label={m['label.created']()}
                    value={agoLabel(server.created_unix)}
                  />
                </div>
              </SideCard>
              <SideCard title={m['tab.backups']()}>
                <p className="text-xs text-muted-foreground">
                  {server.backup_interval
                    ? m['backup.schedule_summary']({
                        interval: server.backup_interval,
                        retention: server.backup_retention,
                      })
                    : m['backup.off']()}
                </p>
              </SideCard>
            </div>
          </div>
        </TabsContent>

        <TabsContent value="console" className="flex min-h-0 flex-col p-5">
          {server.running ? (
            <div className="flex min-h-0 flex-1 flex-col gap-2">
              <div className="min-h-0 flex-1 space-y-0.5 overflow-y-auto border border-border bg-card p-3 font-mono text-[11px] text-muted-foreground">
                {consoleLines.map((line) => (
                  <div key={line}>{line}</div>
                ))}
              </div>
              <form className="flex gap-2" onSubmit={(e) => e.preventDefault()}>
                <Input
                  placeholder={m['detail.console_placeholder']()}
                  className="font-mono"
                />
              </form>
            </div>
          ) : (
            <Empty>{m['detail.console_empty']()}</Empty>
          )}
        </TabsContent>

        <TabsContent value="content" className="p-5">
          <ContentSection
            items={server.content}
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
          <div className="mb-5 flex items-center justify-between">
            <span className="text-xs text-muted-foreground">
              {server.backup_interval
                ? m['backup.schedule_status']({
                    interval: server.backup_interval,
                    retention: server.backup_retention,
                  })
                : m['backup.off_short']()}
            </span>
            <Button size="sm" variant="outline" data-icon="inline-start">
              <PlusIcon weight="bold" />
              {m['backup.create']()}
            </Button>
          </div>
          <BackupList backups={server.backups} />
        </TabsContent>

        <TabsContent value="settings" className="p-5">
          <ServerSettingsForm server={server} />
        </TabsContent>
      </Tabs>

      <ContentInstallModal
        entry={serverTarget(server)}
        open={addingContent}
        onOpenChange={setAddingContent}
      />
    </div>
  );
}
