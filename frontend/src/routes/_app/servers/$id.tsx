import { createFileRoute } from '@tanstack/react-router';

import {
  ServerDetailPage,
  type ServerTab,
} from '@/features/servers/server-detail-page';

const tabs: ServerTab[] = ['console', 'content', 'backups', 'settings'];

export const Route = createFileRoute('/_app/servers/$id')({
  validateSearch: (search: Record<string, unknown>): { tab?: ServerTab } => ({
    tab: tabs.includes(search.tab as ServerTab)
      ? (search.tab as ServerTab)
      : undefined,
  }),
  component: RouteComponent,
});

function RouteComponent() {
  const { id } = Route.useParams();
  const { tab = 'overview' } = Route.useSearch();
  const navigate = Route.useNavigate();

  return (
    <ServerDetailPage
      id={id}
      tab={tab}
      onTabChange={(next) =>
        navigate({
          search: next === 'overview' ? {} : { tab: next },
          replace: true,
        })
      }
    />
  );
}
