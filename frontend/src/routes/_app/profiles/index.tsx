import { createFileRoute } from '@tanstack/react-router';

import type { View } from '@/features/entries/collection';
import { ProfilesPage } from '@/features/profiles/page';

export const Route = createFileRoute('/_app/profiles/')({
  validateSearch: (search: Record<string, unknown>): { view?: View } => ({
    view: search.view === 'list' ? 'list' : undefined,
  }),
  component: RouteComponent,
});

function RouteComponent() {
  const { view = 'grid' } = Route.useSearch();
  const navigate = Route.useNavigate();

  return (
    <ProfilesPage
      view={view}
      onViewChange={(next) =>
        navigate({
          search: { view: next === 'grid' ? undefined : next },
          replace: true,
        })
      }
    />
  );
}
