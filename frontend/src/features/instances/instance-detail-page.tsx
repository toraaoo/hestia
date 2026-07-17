import {
  CopyIcon,
  FolderOpenIcon,
  GlobeHemisphereWestIcon,
  PlayIcon,
  PlusIcon,
  PowerIcon,
  UploadSimpleIcon,
} from '@phosphor-icons/react';

import { useState } from 'react';

import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { entryIcon } from '@/components/icons';
import { Stat, TabCount } from '@/components/page';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { StatusDot } from '@/components/ui/status-dot';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  ContentInstallModal,
  instanceTarget,
} from '@/features/content/install-modal';
import { ContentSection, SideCard, StatCard } from '@/features/entries/detail';
import { getInstance } from '@/features/entries/mock';
import { ResourceCards } from '@/features/entries/resource-panel';
import { InstanceSettingsForm } from '@/features/entries/settings-forms';
import { ProfilesPanel } from '@/features/profiles/profiles-panel';
import { agoLabel } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';
import { m } from '@/paraglide/messages.js';

const logLines = [
  '[12:31:08] [main/INFO]: Setting user: toraaoo',
  '[12:31:11] [Render thread/INFO]: OpenGL initialized, GL version 4.6',
  '[12:31:14] [Render thread/INFO]: Loaded 12 mods',
  '[12:31:19] [Render thread/INFO]: Reloading ResourceManager',
  '[12:31:22] [Render thread/INFO]: Created: 1024x512 textures-atlas',
];

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
  'resourcepack',
  'shader',
  'datapack',
];

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
  const inst = getInstance(id);
  const [addingContent, setAddingContent] = useState(false);

  if (!inst) {
    return (
      <div className="p-6">
        <Empty>{m['instances.missing']()}</Empty>
      </div>
    );
  }

  return (
    <div className="flex min-h-full flex-col">
      <DetailHero
        parentLabel={m['nav.library']()}
        parentTo="/instances"
        icon={entryIcon('instance')}
        name={inst.name}
        badges={
          <>
            <Badge variant="secondary" className="uppercase">
              {inst.flavor}
            </Badge>
            <Badge variant="outline" className="font-mono">
              {inst.game_version}
            </Badge>
            {inst.running && (
              <Badge variant="secondary" className="gap-1.5">
                <StatusDot tone="on" />
                {m['entry.sessions_running']({ count: inst.sessions })}
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
            >
              <FolderOpenIcon className="size-4" />
            </Button>
            {inst.running ? (
              <ConfirmDialog
                trigger={
                  <Button variant="outline" data-icon="inline-start">
                    <PowerIcon weight="bold" />
                    {m['action.stop']()}
                  </Button>
                }
                title={m['entry.stop_title']({ name: inst.name })}
                description={
                  inst.sessions > 1
                    ? m['entry.stop_sessions_description']({
                        count: inst.sessions,
                      })
                    : m['entry.stop_instance_description']()
                }
                confirmLabel={m['action.stop']()}
                onConfirm={() => {}}
              />
            ) : (
              <Button
                data-icon="inline-start"
                className="bg-ember text-ember-foreground hover:bg-ember/90"
              >
                <PlayIcon weight="fill" />
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
            <TabCount n={inst.content.length} />
          </TabsTrigger>
          <TabsTrigger value="profiles">
            {m['profiles.tab']()}
            <TabCount n={inst.profiles.length} />
          </TabsTrigger>
          <TabsTrigger value="worlds">
            {m['tab.worlds']()}
            <TabCount n={inst.worlds.length} />
          </TabsTrigger>
          <TabsTrigger value="logs">{m['tab.logs']()}</TabsTrigger>
          <TabsTrigger value="settings">{m['tab.settings']()}</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="flex flex-col p-5">
          <div className="grid flex-1 gap-6 lg:grid-cols-[1fr_260px]">
            <div className="flex flex-col gap-5">
              <p className="max-w-2xl text-sm leading-relaxed text-foreground/90">
                {m['entry.overview_summary']({
                  flavor: inst.flavor,
                  version: inst.game_version,
                  mods: inst.content.length,
                  worlds: inst.worlds.length,
                })}
              </p>
              <div className="grid grid-cols-3 gap-3">
                <StatCard
                  value={inst.content.length}
                  label={m['label.content']()}
                />
                <StatCard
                  value={inst.worlds.length}
                  label={m['label.worlds']()}
                />
                <StatCard value={inst.memory} label={m['label.memory']()} />
              </div>
              <ResourceCards id={inst.id} />
            </div>

            <div className="space-y-4">
              <SideCard title={m['label.details']()}>
                <div className="divide-y divide-border">
                  <Stat label={m['label.loader']()} value={inst.flavor} />
                  <Stat
                    label={m['label.version']()}
                    value={inst.game_version}
                  />
                  <Stat label={m['label.java']()} value={inst.java_major} />
                  <Stat
                    label={m['label.last_played']()}
                    value={agoLabel(inst.last_played_unix)}
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
                  >
                    <FolderOpenIcon />
                    {m['detail.open_folder']()}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="justify-start"
                    data-icon="inline-start"
                  >
                    <CopyIcon />
                    {m['detail.duplicate']()}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="justify-start"
                    data-icon="inline-start"
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
            items={inst.content}
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
          <ProfilesPanel inst={inst} />
        </TabsContent>

        <TabsContent value="worlds" className="p-5">
          {inst.worlds.length === 0 ? (
            <Empty>{m['detail.no_worlds']()}</Empty>
          ) : (
            <div className="divide-y divide-border border border-border">
              {inst.worlds.map((w) => (
                <div key={w} className="flex items-center gap-3 px-3 py-2.5">
                  <GlobeHemisphereWestIcon className="size-4 text-muted-foreground" />
                  <span className="text-sm">{w}</span>
                </div>
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="logs" className="flex min-h-0 flex-col p-5">
          {inst.running ? (
            <div className="min-h-0 flex-1 space-y-0.5 overflow-y-auto border border-border bg-card p-3 font-mono text-[11px] text-muted-foreground">
              {logLines.map((line) => (
                <div key={line}>{line}</div>
              ))}
            </div>
          ) : (
            <Empty>{m['detail.logs_empty']()}</Empty>
          )}
        </TabsContent>

        <TabsContent value="settings" className="p-5">
          <InstanceSettingsForm inst={inst} />
        </TabsContent>
      </Tabs>

      <ContentInstallModal
        entry={instanceTarget(inst)}
        open={addingContent}
        onOpenChange={setAddingContent}
      />
    </div>
  );
}
