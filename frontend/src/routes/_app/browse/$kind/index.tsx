import { createFileRoute, redirect } from '@tanstack/react-router';
import { BrowsePage } from '@/features/content/page';
import { kindBySlug, sourceSearch } from '@/features/shared/content/lib';

export const Route = createFileRoute('/_app/browse/$kind/')({
  validateSearch: sourceSearch,
  beforeLoad: ({ params }) => {
    if (!kindBySlug(params.kind)) throw redirect({ to: '/browse' });
  },
  component: RouteComponent,
});

function RouteComponent() {
  const { kind } = Route.useParams();
  const { source } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <BrowsePage
      kind={kindBySlug(kind)}
      source={source}
      onSourceChange={(next) => navigate({ search: { source: next } })}
    />
  );
}
