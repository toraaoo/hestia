import { DownloadSimpleIcon, HeartIcon, PlusIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import type { ContentKind } from '@/api';
import { DetailHero } from '@/components/detail-hero';
import { Empty } from '@/components/empty';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { Markdown } from '@/components/markdown';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ContentInstallModal } from '@/features/content/install-modal';
import { kindInfo } from '@/features/content/kinds';
import { agoLabel, compact } from '@/lib/format';
import { m } from '@/paraglide/messages.js';
import { useContentProject, useContentVersions } from '@/queries/content';

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
  const project = useContentProject(id);
  const versions = useContentVersions({ project: id });
  const [installOpen, setInstallOpen] = useState(false);

  if (project.isPending) {
    return <div className="p-6 text-xs text-muted-foreground">…</div>;
  }
  if (!project.data || project.data.kind !== kind) {
    return (
      <div className="p-6">
        <Empty>{m['browse.project_missing']()}</Empty>
      </div>
    );
  }

  const p = project.data;
  const parent = kindInfo[kind];

  return (
    <div className="flex min-h-full flex-col">
      <DetailHero
        parentLabel={parent.label()}
        parentTo="/browse/$kind"
        parentParams={{ kind: parent.slug }}
        icon={contentIcon(p.kind)}
        iconUrl={p.iconUrl || undefined}
        name={p.title}
        badges={
          <>
            <Badge variant="secondary">{contentKindLabel[p.kind]()}</Badge>
            <span className="text-xs text-muted-foreground">
              {m['browse.by_author']({ name: p.author })}
            </span>
          </>
        }
        actions={
          <Button
            data-icon="inline-start"
            className="bg-ember text-ember-foreground hover:bg-ember/90"
            onClick={() => setInstallOpen(true)}
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
            <Markdown className="max-w-2xl">{p.body || p.description}</Markdown>

            <aside className="space-y-4">
              <div className="divide-y divide-border border border-border p-3">
                <div className="flex items-center gap-2 pb-2 text-xs text-muted-foreground">
                  <DownloadSimpleIcon className="size-4" />
                  {m['browse.downloads']({ count: compact(p.downloads) })}
                </div>
                <div className="flex items-center gap-2 py-2 text-xs text-muted-foreground">
                  <HeartIcon className="size-4" />
                  {m['browse.followers']({ count: compact(p.follows) })}
                </div>
              </div>

              {p.categories.length > 0 && (
                <div>
                  <h3 className="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
                    {m['label.categories']()}
                  </h3>
                  <div className="flex flex-wrap gap-1.5">
                    {p.categories.map((c) => (
                      <Badge key={c} variant="secondary">
                        {c}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}
            </aside>
          </div>
        </TabsContent>

        <TabsContent value="versions" className="p-5">
          {versions.isPending ? (
            <p className="text-xs text-muted-foreground">…</p>
          ) : (versions.data ?? []).length === 0 ? (
            <Empty>{m['content.no_versions']()}</Empty>
          ) : (
            <div className="divide-y divide-border border border-border">
              {(versions.data ?? []).map((v) => (
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
                      {agoLabel(Date.parse(v.datePublished) / 1000)}
                    </div>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    data-icon="inline-start"
                    onClick={() => setInstallOpen(true)}
                  >
                    <PlusIcon weight="bold" />
                    {m['action.install']()}
                  </Button>
                </div>
              ))}
            </div>
          )}
        </TabsContent>
      </Tabs>

      <ContentInstallModal
        project={p}
        open={installOpen}
        onOpenChange={setInstallOpen}
      />
    </div>
  );
}
