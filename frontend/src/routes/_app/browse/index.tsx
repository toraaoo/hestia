import { createFileRoute } from '@tanstack/react-router';
import { useState } from 'react';

import { ContentCard } from '@/components/launcher/content-card';
import { Page } from '@/components/launcher/page';
import { useSearch } from '@/components/launcher/search-context';
import { type ContentKind, contentProjects } from '@/lib/mock';
import { cn } from '@/lib/utils';

export const Route = createFileRoute('/_app/browse/')({
  component: BrowsePage,
});

type Filter = 'all' | ContentKind;

const filters: { value: Filter; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'mod', label: 'Mods' },
  { value: 'modpack', label: 'Modpacks' },
  { value: 'resourcepack', label: 'Resource packs' },
  { value: 'shader', label: 'Shaders' },
  { value: 'datapack', label: 'Datapacks' },
];

function BrowsePage() {
  const { query } = useSearch();
  const [kind, setKind] = useState<Filter>('all');

  const q = query.trim().toLowerCase();
  const results = contentProjects.filter((p) => {
    if (kind !== 'all' && p.kind !== kind) return false;
    if (!q) return true;
    return (
      p.title.toLowerCase().includes(q) ||
      p.author.toLowerCase().includes(q) ||
      p.description.toLowerCase().includes(q) ||
      p.categories.some((c) => c.toLowerCase().includes(q))
    );
  });

  return (
    <Page
      title="Browse content"
      subtitle="Discover mods, packs and shaders on Modrinth"
      search
      searchPlaceholder="Search Modrinth"
    >
      <div className="mb-4 flex flex-wrap gap-1.5">
        {filters.map((f) => (
          <button
            key={f.value}
            type="button"
            onClick={() => setKind(f.value)}
            className={cn(
              'border px-2.5 py-1 text-xs transition-colors outline-none focus-visible:ring-1 focus-visible:ring-ring',
              kind === f.value
                ? 'border-transparent bg-primary text-primary-foreground'
                : 'border-border text-muted-foreground hover:bg-muted hover:text-foreground',
            )}
          >
            {f.label}
          </button>
        ))}
      </div>

      {results.length === 0 ? (
        <p className="border border-dashed border-border px-4 py-10 text-center text-xs text-muted-foreground">
          Nothing matches your search.
        </p>
      ) : (
        <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
          {results.map((project) => (
            <ContentCard key={project.id} project={project} />
          ))}
        </div>
      )}
    </Page>
  );
}
