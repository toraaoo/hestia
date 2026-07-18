import { createFileRoute } from '@tanstack/react-router';

import { SkinsPage } from '@/features/skins/skins-page';

export const Route = createFileRoute('/_app/skins/')({
  component: SkinsPage,
});
