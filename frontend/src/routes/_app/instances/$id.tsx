import { createFileRoute } from '@tanstack/react-router';

import {
  InstanceDetailPage,
  type InstanceTab,
  instanceContentKinds,
} from '@/features/instances/instance-detail-page';
import type { ContentKind } from '@/lib/mock';

const tabs: InstanceTab[] = ['content', 'worlds', 'logs', 'settings'];

export const Route = createFileRoute('/_app/instances/$id')({
  validateSearch: (
    search: Record<string, unknown>,
  ): { tab?: InstanceTab; kind?: ContentKind } => {
    const tab = tabs.includes(search.tab as InstanceTab)
      ? (search.tab as InstanceTab)
      : undefined;
    return {
      tab,
      kind:
        tab === 'content' &&
        instanceContentKinds.includes(search.kind as ContentKind)
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
    <InstanceDetailPage
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
