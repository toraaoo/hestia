import { createFileRoute, redirect } from '@tanstack/react-router';
import { ProjectDetailPage, type ProjectTab } from '@/features/content/detail';
import { kindBySlug, sourceSearch } from '@/features/shared/content/lib';

export const Route = createFileRoute('/_app/browse/$kind/$id')({
  validateSearch: (
    search: Record<string, unknown>,
  ): { tab?: ProjectTab; source?: string; version?: string } => ({
    tab: search.tab === 'versions' ? 'versions' : undefined,
    ...sourceSearch(search),
    version: typeof search.version === 'string' ? search.version : undefined,
  }),
  beforeLoad: ({ params }) => {
    if (!kindBySlug(params.kind)) throw redirect({ to: '/browse' });
  },
  component: RouteComponent,
});

function RouteComponent() {
  const { kind, id } = Route.useParams();
  const { tab = 'description', source = '', version } = Route.useSearch();
  const navigate = Route.useNavigate();

  const resolvedKind = kindBySlug(kind);
  if (!resolvedKind) return null;

  return (
    <ProjectDetailPage
      kind={resolvedKind}
      id={id}
      source={source}
      pinnedVersion={version}
      tab={tab}
      onTabChange={(next) =>
        navigate({
          search:
            next === 'description'
              ? { source, version }
              : { source, version, tab: next },
          replace: true,
        })
      }
    />
  );
}
