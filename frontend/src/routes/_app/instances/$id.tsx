import { createFileRoute } from '@tanstack/react-router';

import { InstanceDetailPage } from '@/features/instances/instance-detail-page';

export const Route = createFileRoute('/_app/instances/$id')({
  component: RouteComponent,
});

function RouteComponent() {
  const { id } = Route.useParams();
  return <InstanceDetailPage id={id} />;
}
