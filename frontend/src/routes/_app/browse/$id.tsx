import { createFileRoute } from '@tanstack/react-router';

import { ProjectDetailPage } from '@/features/browse/project-detail-page';

export const Route = createFileRoute('/_app/browse/$id')({
  component: RouteComponent,
});

function RouteComponent() {
  const { id } = Route.useParams();
  return <ProjectDetailPage id={id} />;
}
