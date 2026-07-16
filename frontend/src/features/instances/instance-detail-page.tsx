import {
  CopyIcon,
  FolderOpenIcon,
  GlobeHemisphereWestIcon,
  PlayIcon,
  PlusIcon,
  PowerIcon,
  UploadSimpleIcon,
} from '@phosphor-icons/react';

import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { entryIcon } from '@/components/icons';
import { Stat, TabCount } from '@/components/page';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { StatusDot } from '@/components/ui/status-dot';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  BackupList,
  ContentSection,
  SideCard,
  StatCard,
} from '@/features/entries/detail';
import { getInstance } from '@/features/entries/mock';
import { InstanceSettingsForm } from '@/features/entries/settings-forms';
import { agoLabel } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';

export type InstanceTab =
  | 'overview'
  | 'content'
  | 'worlds'
  | 'backups'
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

  if (!inst) {
    return (
      <div className="p-6">
        <Empty>That instance no longer exists.</Empty>
      </div>
    );
  }

  return (
    <div className="flex min-h-full flex-col">
      <DetailHero
        parentLabel="Library"
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
                {inst.sessions} running
              </Badge>
            )}
          </>
        }
        actions={
          <>
            <Button variant="outline" size="icon" aria-label="Open folder">
              <FolderOpenIcon className="size-4" />
            </Button>
            {inst.running ? (
              <Button variant="outline" data-icon="inline-start">
                <PowerIcon weight="bold" />
                Stop
              </Button>
            ) : (
              <Button
                data-icon="inline-start"
                className="bg-ember text-ember-foreground hover:bg-ember/90"
              >
                <PlayIcon weight="fill" />
                Play
              </Button>
            )}
          </>
        }
      />

      <Tabs
        value={tab}
        onValueChange={(value) => onTabChange(value as InstanceTab)}
        className="gap-0 p-0"
      >
        <TabsList variant="line" className="h-auto gap-6 px-5">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="content">
            Content
            <TabCount n={inst.content.length} />
          </TabsTrigger>
          <TabsTrigger value="worlds">
            Worlds
            <TabCount n={inst.worlds.length} />
          </TabsTrigger>
          <TabsTrigger value="backups">
            Backups
            <TabCount n={inst.backups.length} />
          </TabsTrigger>
          <TabsTrigger value="logs">Logs</TabsTrigger>
          <TabsTrigger value="settings">Settings</TabsTrigger>
        </TabsList>

        <TabsContent value="overview" className="p-5">
          <div className="grid gap-6 lg:grid-cols-[1fr_260px]">
            <div className="space-y-5">
              <p className="max-w-2xl text-sm leading-relaxed text-foreground/90">
                A {inst.flavor} {inst.game_version} instance with{' '}
                {inst.content.length} mods across {inst.worlds.length} worlds.
              </p>
              <div className="grid grid-cols-3 gap-3">
                <StatCard value={inst.content.length} label="Content" />
                <StatCard value={inst.worlds.length} label="Worlds" />
                <StatCard value={inst.memory} label="Memory" />
              </div>
            </div>

            <div className="space-y-4">
              <SideCard title="Details">
                <div className="divide-y divide-border">
                  <Stat label="Loader" value={inst.flavor} />
                  <Stat label="Version" value={inst.game_version} />
                  <Stat label="Java" value={inst.java_major} />
                  <Stat
                    label="Last played"
                    value={agoLabel(inst.last_played_unix)}
                  />
                </div>
              </SideCard>
              <SideCard title="Quick actions">
                <div className="flex flex-col gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="justify-start"
                    data-icon="inline-start"
                  >
                    <FolderOpenIcon />
                    Open folder
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="justify-start"
                    data-icon="inline-start"
                  >
                    <CopyIcon />
                    Duplicate
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="justify-start"
                    data-icon="inline-start"
                  >
                    <UploadSimpleIcon />
                    Export
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
              <Button size="sm" variant="outline" data-icon="inline-start">
                <PlusIcon weight="bold" />
                Add content
              </Button>
            }
          />
        </TabsContent>

        <TabsContent value="worlds" className="p-5">
          {inst.worlds.length === 0 ? (
            <Empty>No saved worlds yet.</Empty>
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

        <TabsContent value="backups" className="p-5">
          <div className="mb-5 flex justify-end">
            <Button size="sm" variant="outline" data-icon="inline-start">
              <PlusIcon weight="bold" />
              Create backup
            </Button>
          </div>
          <BackupList backups={inst.backups} />
        </TabsContent>

        <TabsContent value="logs" className="p-5">
          <Empty>Launch this instance to stream its log here.</Empty>
        </TabsContent>

        <TabsContent value="settings" className="p-5">
          <InstanceSettingsForm inst={inst} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
