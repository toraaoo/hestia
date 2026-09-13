import { createFileRoute } from '@tanstack/react-router';

import { ScreenshotsPage } from '@/features/screenshots/page';

export const Route = createFileRoute('/_app/screenshots/')({
  component: ScreenshotsPage,
});
