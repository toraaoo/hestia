import { createFileRoute } from '@tanstack/react-router';

import {
  ProfilesPage,
  profileFilterKinds,
} from '@/features/profiles/profiles-page';
import type { ContentKind } from '@/lib/mock';

export const Route = createFileRoute('/_app/profiles/')({
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
  const { kind } = Route.useSearch();
  const navigate = Route.useNavigate();

  return (
    <ProfilesPage
      kind={kind}
      onKindChange={(next) =>
        navigate({ search: next ? { kind: next } : {}, replace: true })
      }
    />
  );
}
