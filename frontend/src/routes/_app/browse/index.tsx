import { createFileRoute } from '@tanstack/react-router';
import { BrowsePage } from '@/features/content/page';
import { sourceSearch } from '@/features/shared/content/lib';

export const Route = createFileRoute('/_app/browse/')({
  validateSearch: sourceSearch,
  component: RouteComponent,
});

function RouteComponent() {
  const { source } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <BrowsePage
      source={source}
      onSourceChange={(next) => navigate({ search: { source: next } })}
    />
  );
}
