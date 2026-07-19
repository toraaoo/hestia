import { createFileRoute } from '@tanstack/react-router';
import type { ContentKind } from '@/api';
import { ProfileDetailPage } from '@/features/profiles/profile-detail-page';
import { profileFilterKinds } from '@/features/profiles/profiles-page';

export const Route = createFileRoute('/_app/profiles/$name')({
  validateSearch: (
    search: Record<string, unknown>,
  ): { kind?: ContentKind } => ({
    kind: profileFilterKinds.includes(search.kind as ContentKind)
      ? (search.kind as ContentKind)
      : undefined,
  }),
  component: RouteComponent,
});

function RouteComponent() {
  const { name } = Route.useParams();
  const { kind } = Route.useSearch();
  const navigate = Route.useNavigate();

  return (
    <ProfileDetailPage
      name={name}
      kind={kind}
      onKindChange={(next) =>
        navigate({ search: next ? { kind: next } : {}, replace: true })
      }
    />
  );
}
