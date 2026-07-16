import { DownloadSimpleIcon, HeartIcon, PlusIcon } from '@phosphor-icons/react';

import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Stat } from '@/components/page';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { kindInfo } from '@/features/browse/kinds';
import { getProject } from '@/features/browse/mock';
import { agoLabel, compact } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';

const versions = [
  { id: 'v1', name: '0.6.13', game: '1.21.4', loader: 'fabric', when: 5 },
  { id: 'v2', name: '0.6.12', game: '1.21.3', loader: 'fabric', when: 26 },
  { id: 'v3', name: '0.6.9', game: '1.21.1', loader: 'fabric', when: 58 },
];

export type ProjectTab = 'description' | 'versions';

export function ProjectDetailPage({
  kind,
  id,
  tab,
  onTabChange,
}: {
  kind: ContentKind;
  id: string;
  tab: ProjectTab;
  onTabChange: (tab: ProjectTab) => void;
}) {
  const project = getProject(id);

  if (!project || project.kind !== kind) {
    return (
      <div className="p-6">
        <Empty>That project could not be found.</Empty>
      </div>
    );
  }

  const parent = kindInfo[kind];

  return (
    <div className="flex min-h-full flex-col">
      <DetailHero
        parentLabel={parent.label}
        parentTo="/browse/$kind"
        parentParams={{ kind: parent.slug }}
        icon={contentIcon(project.kind)}
        name={project.title}
        badges={
          <>
            <Badge variant="secondary">{contentKindLabel[project.kind]}</Badge>
            <span className="text-xs text-muted-foreground">
              by {project.author}
            </span>
          </>
        }
        actions={
          <Button
            data-icon="inline-start"
            className="bg-ember text-ember-foreground hover:bg-ember/90"
          >
            <PlusIcon weight="bold" />
            Install
          </Button>
        }
      />

      <Tabs
        value={tab}
        onValueChange={(value) => onTabChange(value as ProjectTab)}
        className="gap-0 p-0"
      >
        <TabsList
          variant="line"
          className="h-auto gap-4 border-b border-border px-5"
        >
          <TabsTrigger value="description">Description</TabsTrigger>
          <TabsTrigger value="versions">Versions</TabsTrigger>
        </TabsList>

        <TabsContent value="description" className="p-5">
          <div className="grid gap-6 lg:grid-cols-[1fr_260px]">
            <p className="max-w-2xl text-sm leading-relaxed text-foreground/90">
              {project.description}
            </p>

            <aside className="space-y-4">
              <div className="divide-y divide-border border border-border p-3">
                <div className="flex items-center gap-2 pb-2 text-xs text-muted-foreground">
                  <DownloadSimpleIcon className="size-4" />
                  {compact(project.downloads)} downloads
                </div>
                <div className="flex items-center gap-2 py-2 text-xs text-muted-foreground">
                  <HeartIcon className="size-4" />
                  {compact(project.follows)} followers
                </div>
                <Stat label="Updated" value={agoLabel(project.updated_unix)} />
              </div>

              <div>
                <h3 className="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
                  Categories
                </h3>
                <div className="flex flex-wrap gap-1.5">
                  {project.categories.map((c) => (
                    <Badge key={c} variant="secondary">
                      {c}
                    </Badge>
                  ))}
                </div>
              </div>
            </aside>
          </div>
        </TabsContent>

        <TabsContent value="versions" className="p-5">
          <div className="divide-y divide-border border border-border">
            {versions.map((v) => (
              <div key={v.id} className="flex items-center gap-3 px-3 py-2.5">
                <div className="min-w-0 flex-1">
                  <div className="text-sm">{v.name}</div>
                  <div className="font-mono text-[11px] text-muted-foreground">
                    {v.loader} · {v.game} ·{' '}
                    {agoLabel(Math.floor(Date.now() / 1000 - v.when * 86_400))}
                  </div>
                </div>
                <Button variant="outline" size="sm" data-icon="inline-start">
                  <PlusIcon weight="bold" />
                  Install
                </Button>
              </div>
            ))}
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}
