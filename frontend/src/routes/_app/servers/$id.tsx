import { createFileRoute } from '@tanstack/react-router';

import { ServerDetailPage } from '@/features/servers/server-detail-page';

export const Route = createFileRoute('/_app/servers/$id')({
  component: RouteComponent,
});

function RouteComponent() {
  const { id } = Route.useParams();
  return <ServerDetailPage id={id} />;
}
