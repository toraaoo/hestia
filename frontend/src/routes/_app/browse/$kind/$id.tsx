import { createFileRoute, redirect } from '@tanstack/react-router';

import { kindBySlug } from '@/features/content/kinds';
import {
  ProjectDetailPage,
  type ProjectTab,
} from '@/features/content/project-detail-page';

export const Route = createFileRoute('/_app/browse/$kind/$id')({
  validateSearch: (search: Record<string, unknown>): { tab?: ProjectTab } => ({
    tab: search.tab === 'versions' ? 'versions' : undefined,
  }),
  beforeLoad: ({ params }) => {
    if (!kindBySlug(params.kind)) throw redirect({ to: '/browse' });
  },
  component: RouteComponent,
});

function RouteComponent() {
  const { kind, id } = Route.useParams();
  const { tab = 'description' } = Route.useSearch();
  const navigate = Route.useNavigate();

  const resolvedKind = kindBySlug(kind);
  if (!resolvedKind) return null;

  return (
    <ProjectDetailPage
      kind={resolvedKind}
      id={id}
      tab={tab}
      onTabChange={(next) =>
        navigate({
          search: next === 'description' ? {} : { tab: next },
          replace: true,
        })
      }
    />
  );
}
