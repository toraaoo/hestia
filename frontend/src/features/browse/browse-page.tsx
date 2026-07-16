import { Link } from '@tanstack/react-router';

import { useSearch } from '@/components/app-shell/search-context';
import { Page } from '@/components/page';
import { ContentCard } from '@/features/browse/content-card';
import { contentKinds, kindInfo } from '@/features/browse/kinds';
import { contentProjects } from '@/features/browse/mock';
import type { ContentKind } from '@/lib/mock';
import { cn } from '@/lib/utils';

const chipClass = (active: boolean) =>
  cn(
    'border px-2.5 py-1 text-xs transition-colors outline-none focus-visible:ring-1 focus-visible:ring-ring',
    active
      ? 'border-transparent bg-primary text-primary-foreground'
      : 'border-border text-muted-foreground hover:bg-muted hover:text-foreground',
  );

export function BrowsePage({ kind }: { kind?: ContentKind }) {
  const { query } = useSearch();

  const q = query.trim().toLowerCase();
  const results = contentProjects.filter((p) => {
    if (kind && p.kind !== kind) return false;
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
      <div className="mb-5 flex flex-wrap gap-1.5">
        <Link to="/browse" className={chipClass(!kind)}>
          All
        </Link>
        {contentKinds.map((k) => (
          <Link
            key={k}
            to="/browse/$kind"
            params={{ kind: kindInfo[k].slug }}
            className={chipClass(kind === k)}
          >
            {kindInfo[k].label}
          </Link>
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
