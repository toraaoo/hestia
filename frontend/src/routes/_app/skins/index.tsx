import { createFileRoute, redirect } from '@tanstack/react-router';

import { SkinsPage } from '@/features/skins/skins-page';
import { ensureSignedIn } from '@/queries';

export const Route = createFileRoute('/_app/skins/')({
  beforeLoad: async ({ context }) => {
    if (!(await ensureSignedIn(context.queryClient))) {
      throw redirect({ to: '/' });
    }
  },
  component: SkinsPage,
});
