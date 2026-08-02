import { MagnifyingGlassIcon, WarningCircleIcon } from '@phosphor-icons/react';
import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useIntersectionObserver } from '@uidotdev/usehooks';
import { useEffect, useMemo } from 'react';

import { type ContentKind, errorMessage } from '@/api';
import { useSearch } from '@/components/app-shell/search-context';
import { Empty } from '@/components/empty';
import { FilterMenu } from '@/components/filter-menu';
import { Page } from '@/components/page';
import { CardGridSkeleton } from '@/components/skeleton';
import {
  ContentCard,
  sourceGroup,
  useSourceOptions,
} from '@/features/content/components';
import { mergeHits, projectKey } from '@/features/content/lib';
import { kindGroup } from '@/features/shared/content/components';
import { contentKinds, kindInfo } from '@/features/shared/content/lib';
import { m } from '@/paraglide/messages.js';
import { contentQueries, isContentUrl } from '@/queries/content';

export function BrowsePage({
  kind,
  source = '',
  onSourceChange,
}: {
  kind?: ContentKind;
  source?: string;
  onSourceChange?: (source: string) => void;
}) {
  const { query } = useSearch();
  const q = query.trim();
  const sources = useSourceOptions(kind, source);
  // The default source is the absence of the param, not an empty one.
  const sourceParam = source ? { source } : {};

  // The kind is the route, so narrowing to one is navigation — which keeps a
  // browsed kind shareable and survives a reload.
  const navigate = useNavigate();
  const goToKind = (next?: ContentKind) =>
    next
      ? navigate({
          to: '/browse/$kind',
          params: { kind: kindInfo[next].slug },
          search: sourceParam,
        })
      : navigate({ to: '/browse', search: sourceParam });

  const url = isContentUrl(q) ? q : '';
  const link = useQuery(contentQueries.url(url));

  // A specific kind is one search; "All" fans out over every kind and merges,
  // since a source's search is scoped to a single project type.
  const kinds = kind ? [kind] : contentKinds;
  const search = useInfiniteQuery({
    ...contentQueries.searchPaged(kinds, q, sources.active),
    enabled: !url,
  });
  const { fetchNextPage, hasNextPage, isFetchingNextPage } = search;

  const hits = useMemo(
    () => mergeHits(search.data?.pages),
    [search.data?.pages],
  );

  // Grow the page when the sentinel scrolls into view (infinite scroll).
  const [sentinelRef, sentinel] = useIntersectionObserver({
    threshold: 0,
    rootMargin: '600px',
  });
  useEffect(() => {
    if (sentinel?.isIntersecting && hasNextPage && !isFetchingNextPage) {
      fetchNextPage();
    }
  }, [sentinel, hasNextPage, isFetchingNextPage, fetchNextPage]);

  return (
    <Page
      title={m['app.nav.browse']()}
      subtitle={m['content.browse.subtitle']()}
      search
      searchPlaceholder={m['app.search.content_or_link']()}
      actions={
        <FilterMenu
          groups={[
            kindGroup({
              kinds: contentKinds,
              kind,
              onKindChange: goToKind,
            }),
            sourceGroup(sources.list, sources.active, (next) =>
              onSourceChange?.(next),
            ),
          ]}
        />
      }
      skeleton={
        <CardGridSkeleton
          grid="grid grid-cols-1 gap-3 xl:grid-cols-2"
          count={8}
          card="h-28"
        />
      }
    >
      {url ? (
        link.isPending ? (
          <CardGridSkeleton
            grid="grid grid-cols-1 gap-3 xl:grid-cols-2"
            count={1}
            card="h-28"
          />
        ) : link.data ? (
          <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
            <ContentCard
              project={link.data.project}
              pinnedVersion={link.data.versionId || undefined}
            />
          </div>
        ) : (
          <Empty icon={WarningCircleIcon} tone="destructive">
            {errorMessage(link.error)}
          </Empty>
        )
      ) : search.isPending ? (
        <CardGridSkeleton
          grid="grid grid-cols-1 gap-3 xl:grid-cols-2"
          count={8}
          card="h-28"
        />
      ) : hits.length === 0 ? (
        <Empty icon={MagnifyingGlassIcon}>
          {m['content.browse.nothing_matches']()}
        </Empty>
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
              {isFetchingNextPage ? m['content.browse.loading_more']() : null}
            </div>
          )}
        </>
      )}
    </Page>
  );
}
