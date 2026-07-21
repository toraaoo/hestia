import { createFileRoute } from '@tanstack/react-router';

import type { View } from '@/features/entries/components/collection';
import { InstancesPage } from '@/features/instances/page';

export const Route = createFileRoute('/_app/instances/')({
  validateSearch: (
    search: Record<string, unknown>,
  ): { view?: View; flavor?: string } => ({
    view: search.view === 'list' ? 'list' : undefined,
    flavor:
      typeof search.flavor === 'string' && search.flavor !== 'all'
        ? search.flavor
        : undefined,
  }),
  component: RouteComponent,
});

function RouteComponent() {
  const { view = 'grid', flavor = 'all' } = Route.useSearch();
  const navigate = Route.useNavigate();

  return (
    <InstancesPage
      view={view}
      flavor={flavor}
      onViewChange={(next) =>
        navigate({
          search: (prev) => ({
            ...prev,
            view: next === 'grid' ? undefined : next,
          }),
          replace: true,
        })
      }
      onFlavorChange={(next) =>
        navigate({
          search: (prev) => ({
            ...prev,
            flavor: next === 'all' ? undefined : next,
          }),
          replace: true,
        })
      }
    />
  );
}
