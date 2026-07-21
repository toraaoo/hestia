import { createFileRoute } from '@tanstack/react-router';

import type { View } from '@/features/entries/components/collection';
import { LibraryPage } from '@/features/library/page';

type LibrarySearch = { view?: View; servers?: string; instances?: string };

const flavor = (value: unknown) =>
  typeof value === 'string' && value !== 'all' ? value : undefined;

export const Route = createFileRoute('/_app/')({
  validateSearch: (search: Record<string, unknown>): LibrarySearch => ({
    view: search.view === 'list' ? 'list' : undefined,
    servers: flavor(search.servers),
    instances: flavor(search.instances),
  }),
  component: RouteComponent,
});

function RouteComponent() {
  const {
    view = 'grid',
    servers = 'all',
    instances = 'all',
  } = Route.useSearch();
  const navigate = Route.useNavigate();

  const patch = (next: LibrarySearch) =>
    navigate({ search: (prev) => ({ ...prev, ...next }), replace: true });

  return (
    <LibraryPage
      view={view}
      serverFlavor={servers}
      instanceFlavor={instances}
      onViewChange={(next) =>
        patch({ view: next === 'grid' ? undefined : next })
      }
      onServerFlavorChange={(next) =>
        patch({ servers: next === 'all' ? undefined : next })
      }
      onInstanceFlavorChange={(next) =>
        patch({ instances: next === 'all' ? undefined : next })
      }
    />
  );
}
