import { Link } from '@tanstack/react-router';
import { useEffect, useRef } from 'react';

import type { ContentKind, ContentProject } from '@/api';
import { useSearch } from '@/components/app-shell/search-context';
import { chipClass } from '@/components/chip';
import { Page } from '@/components/page';
import { Bone, CardGridSkeleton } from '@/components/skeleton';
import { ContentCard } from '@/features/content/content-card';
import { contentKinds, kindInfo } from '@/features/content/kinds';
import { m } from '@/paraglide/messages.js';
import { useContentSearchPaged } from '@/queries/content';

/** Merge/sort key so the same project from one source is never listed twice. */
const projectKey = (p: ContentProject) => `${p.source}:${p.id}`;

export function BrowsePage({ kind }: { kind?: ContentKind }) {
  const { query } = useSearch();
  const q = query.trim();

  // A specific kind is one search; "All" fans out over every kind and merges,
  // since a source's search is scoped to a single project type.
  const kinds = kind ? [kind] : contentKinds;
  const search = useContentSearchPaged(kinds, q);
  const { fetchNextPage, hasNextPage, isFetchingNextPage } = search;

  const hits = (search.data?.pages ?? [])
    .flat()
    .flatMap((r) => r.hits)
    .filter(
      (p, i, all) =>
        all.findIndex((x) => projectKey(x) === projectKey(p)) === i,
    )
    .sort((a, b) => b.downloads - a.downloads);

  // Grow the page when the sentinel scrolls into view (infinite scroll).
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const node = sentinelRef.current;
    if (!node || !hasNextPage) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting && !isFetchingNextPage) fetchNextPage();
      },
      { rootMargin: '600px' },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  return (
    <Page
      title={m['nav.browse']()}
      subtitle={m['browse.subtitle']()}
      search
      searchPlaceholder={m['search.modrinth']()}
      skeleton={
        <div>
          <div className="mb-5 flex flex-wrap gap-1.5">
            {contentKinds.map((k) => (
              <Bone key={k} className="h-6 w-20" />
            ))}
          </div>
          <CardGridSkeleton
            grid="grid grid-cols-1 gap-3 xl:grid-cols-2"
            count={8}
            card="h-28"
          />
        </div>
      }
    >
      <div className="mb-5 flex flex-wrap gap-1.5">
        <Link to="/browse" className={chipClass(!kind)}>
          {m['label.all']()}
        </Link>
        {contentKinds.map((k) => (
          <Link
            key={k}
            to="/browse/$kind"
            params={{ kind: kindInfo[k].slug }}
            className={chipClass(kind === k)}
          >
            {kindInfo[k].label()}
          </Link>
        ))}
      </div>

      {search.isPending ? (
        <CardGridSkeleton
          grid="grid grid-cols-1 gap-3 xl:grid-cols-2"
          count={8}
          card="h-28"
        />
      ) : hits.length === 0 ? (
        <p className="border border-dashed border-border px-4 py-10 text-center text-xs text-muted-foreground">
          {m['browse.nothing_matches']()}
        </p>
      ) : (
        <>
          <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
            {hits.map((project) => (
              <ContentCard key={projectKey(project)} project={project} />
            ))}
          </div>
          {hasNextPage && (
            <div
              ref={sentinelRef}
              className="mt-5 flex justify-center py-4 text-xs text-muted-foreground"
            >
              {isFetchingNextPage ? m['browse.loading_more']() : null}
            </div>
          )}
        </>
      )}
    </Page>
  );
}
