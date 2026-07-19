import { createFileRoute } from '@tanstack/react-router';
import type { ContentKind } from '@/api';
import {
  ServerDetailPage,
  type ServerTab,
  serverContentKinds,
} from '@/features/servers/server-detail-page';

const tabs: ServerTab[] = ['console', 'content', 'backups', 'settings'];

export const Route = createFileRoute('/_app/servers/$id')({
  validateSearch: (
    search: Record<string, unknown>,
  ): { tab?: ServerTab; kind?: ContentKind } => {
    const tab = tabs.includes(search.tab as ServerTab)
      ? (search.tab as ServerTab)
      : undefined;
    return {
      tab,
      kind:
        tab === 'content' &&
        serverContentKinds.includes(search.kind as ContentKind)
          ? (search.kind as ContentKind)
          : undefined,
    };
  },
  component: RouteComponent,
});

function RouteComponent() {
  const { id } = Route.useParams();
  const { tab = 'overview', kind } = Route.useSearch();
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
      contentKind={kind}
      onContentKindChange={(next) =>
        navigate({
          search: { tab: 'content', kind: next },
          replace: true,
        })
      }
    />
  );
}
