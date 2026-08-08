import { createFileRoute } from '@tanstack/react-router';

import { OfflinePage } from '@/features/offline/page';

export const Route = createFileRoute('/_app/offline/')({
  component: OfflinePage,
});
