import { useQueries } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useEffect, useRef, useState } from 'react';

import type { ContentKind, ContentProject } from '@/api';
import { useSearch } from '@/components/app-shell/search-context';
import { chipClass } from '@/components/chip';
import { Page } from '@/components/page';
import { Bone, CardGridSkeleton } from '@/components/skeleton';
import { ContentCard } from '@/features/content/content-card';
import { contentKinds, kindInfo } from '@/features/content/kinds';
import { m } from '@/paraglide/messages.js';
import { contentQueries } from '@/queries/content';

const PAGE = 20;

/** Merge/sort key so the same project from one source is never listed twice. */
const projectKey = (p: ContentProject) => `${p.source}:${p.id}`;

export function BrowsePage({ kind }: { kind?: ContentKind }) {
  const { query } = useSearch();
  const [limit, setLimit] = useState(PAGE);
  const q = query.trim();

  // A specific kind is one search; "All" fans out over every kind and merges,
  // since a source's search is scoped to a single project type.
  const kinds = kind ? [kind] : contentKinds;
  const searches = useQueries({
    queries: kinds.map((k) =>
      contentQueries.search({ kind: k, query: q, limit }),
    ),
  });

  const loading = searches.some((s) => s.isPending);
  const fetchingMore = searches.some((s) => s.isFetching);
  const hits = searches
    .flatMap((s) => s.data?.hits ?? [])
    .filter(
      (p, i, all) =>
        all.findIndex((x) => projectKey(x) === projectKey(p)) === i,
    )
    .sort((a, b) => b.downloads - a.downloads);
  const hasMore = kind
    ? (searches[0]?.data?.total ?? 0) > hits.length
    : searches.some((s) => (s.data?.total ?? 0) > (s.data?.hits.length ?? 0));

  // A new query or kind starts paging over — otherwise a fresh search would
  // fetch the whole grown page at once.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset on inputs.
  useEffect(() => setLimit(PAGE), [q, kind]);

  // Grow the page when the sentinel scrolls into view (infinite scroll).
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const node = sentinelRef.current;
    if (!node || !hasMore) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting && !fetchingMore) {
          setLimit((l) => l + PAGE);
        }
      },
      { rootMargin: '600px' },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [hasMore, fetchingMore]);

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

      {loading ? (
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
          {hasMore && (
            <div
              ref={sentinelRef}
              className="mt-5 flex justify-center py-4 text-xs text-muted-foreground"
            >
              {fetchingMore ? m['browse.loading_more']() : null}
            </div>
          )}
        </>
      )}
    </Page>
  );
}
