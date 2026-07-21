import { createFileRoute, redirect } from '@tanstack/react-router';
import { kindBySlug } from '@/features/content/kinds';
import { BrowsePage } from '@/features/content/page';

export const Route = createFileRoute('/_app/browse/$kind/')({
  beforeLoad: ({ params }) => {
    if (!kindBySlug(params.kind)) throw redirect({ to: '/browse' });
  },
  component: RouteComponent,
});

function RouteComponent() {
  const { kind } = Route.useParams();
  return <BrowsePage kind={kindBySlug(kind)} />;
}
