import { createFileRoute } from '@tanstack/react-router';

import { AppLayout } from '@/components/app-shell/app-layout';

export const Route = createFileRoute('/_app')({ component: AppLayout });
