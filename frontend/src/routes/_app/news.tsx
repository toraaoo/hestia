import { createFileRoute } from '@tanstack/react-router';

import { NewsPage } from '@/features/news/page';

export const Route = createFileRoute('/_app/news')({
  component: NewsPage,
});
