import { createFileRoute } from '@tanstack/react-router';

import { BrowsePage } from '@/features/content/browse-page';

export const Route = createFileRoute('/_app/browse/')({
  component: BrowsePage,
});
