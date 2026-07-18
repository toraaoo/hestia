import { DownloadSimpleIcon, HeartIcon, PlusIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Stat } from '@/components/page';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ContentInstallModal } from '@/features/content/install-modal';
import { kindInfo } from '@/features/content/kinds';
import { getProject, projectVersions } from '@/features/content/mock';
import { agoLabel, compact } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';
import { m } from '@/paraglide/messages.js';

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
  const [installVersion, setInstallVersion] = useState<string | null>(null);
  const [installOpen, setInstallOpen] = useState(false);

  if (!project || project.kind !== kind) {
    return (
      <div className="p-6">
        <Empty>{m['browse.project_missing']()}</Empty>
      </div>
    );
  }

  const parent = kindInfo[kind];
  const versions = projectVersions(project);
  const openInstall = (versionId?: string) => {
    setInstallVersion(versionId ?? null);
    setInstallOpen(true);
  };

  return (
    <div className="flex min-h-full flex-col">
      <DetailHero
        parentLabel={parent.label()}
        parentTo="/browse/$kind"
        parentParams={{ kind: parent.slug }}
        icon={contentIcon(project.kind)}
        name={project.title}
        badges={
          <>
            <Badge variant="secondary">
              {contentKindLabel[project.kind]()}
            </Badge>
            <span className="text-xs text-muted-foreground">
              {m['browse.by_author']({ name: project.author })}
            </span>
          </>
        }
        actions={
          <Button
            data-icon="inline-start"
            className="bg-ember text-ember-foreground hover:bg-ember/90"
            onClick={() => openInstall()}
          >
            <PlusIcon weight="bold" />
            {m['action.install']()}
          </Button>
        }
      />

      <Tabs
        value={tab}
        onValueChange={(value) => onTabChange(value as ProjectTab)}
        className="gap-0 p-0"
      >
        <TabsList variant="line" className="h-auto gap-6 px-5">
          <TabsTrigger value="description">
            {m['tab.description']()}
          </TabsTrigger>
          <TabsTrigger value="versions">{m['tab.versions']()}</TabsTrigger>
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
                  {m['browse.downloads']({ count: compact(project.downloads) })}
                </div>
                <div className="flex items-center gap-2 py-2 text-xs text-muted-foreground">
                  <HeartIcon className="size-4" />
                  {m['browse.followers']({ count: compact(project.follows) })}
                </div>
                <Stat
                  label={m['label.updated']()}
                  value={agoLabel(project.updatedUnix)}
                />
              </div>

              <div>
                <h3 className="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
                  {m['label.categories']()}
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
                  <div className="flex items-center gap-2">
                    <span className="text-sm">{v.versionNumber}</span>
                    {v.channel !== 'release' && (
                      <Badge
                        variant="outline"
                        className="text-[10px] capitalize"
                      >
                        {v.channel}
                      </Badge>
                    )}
                  </div>
                  <div className="font-mono text-[11px] text-muted-foreground">
                    {v.loaders.join(', ')} · {v.gameVersions.join(', ')} ·{' '}
                    {agoLabel(v.publishedUnix)}
                  </div>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  data-icon="inline-start"
                  onClick={() => openInstall(v.id)}
                >
                  <PlusIcon weight="bold" />
                  {m['action.install']()}
                </Button>
              </div>
            ))}
          </div>
        </TabsContent>
      </Tabs>

      <ContentInstallModal
        project={project}
        versionId={installVersion ?? undefined}
        open={installOpen}
        onOpenChange={setInstallOpen}
      />
    </div>
  );
}
