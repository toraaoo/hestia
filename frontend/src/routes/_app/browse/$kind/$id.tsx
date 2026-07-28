import { createFileRoute, redirect } from '@tanstack/react-router';
import { ProjectDetailPage, type ProjectTab } from '@/features/content/detail';
import { kindBySlug } from '@/features/content/lib/kinds';

export const Route = createFileRoute('/_app/browse/$kind/$id')({
  validateSearch: (
    search: Record<string, unknown>,
  ): { tab?: ProjectTab; version?: string } => ({
    tab: search.tab === 'versions' ? 'versions' : undefined,
    version: typeof search.version === 'string' ? search.version : undefined,
  }),
  beforeLoad: ({ params }) => {
    if (!kindBySlug(params.kind)) throw redirect({ to: '/browse' });
  },
  component: RouteComponent,
});

function RouteComponent() {
  const { kind, id } = Route.useParams();
  const { tab = 'description', version } = Route.useSearch();
  const navigate = Route.useNavigate();

  const resolvedKind = kindBySlug(kind);
  if (!resolvedKind) return null;

  return (
    <ProjectDetailPage
      kind={resolvedKind}
      id={id}
      pinnedVersion={version}
      tab={tab}
      onTabChange={(next) =>
        navigate({
          search: next === 'description' ? { version } : { version, tab: next },
          replace: true,
        })
      }
    />
  );
}
