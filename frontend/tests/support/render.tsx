import { QueryClientProvider } from '@tanstack/react-query';
import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { render, type RenderResult } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';
import { Toaster } from '@/components/ui/sonner';
import { LocaleProvider } from '@/hooks/locale';
import { queryClient } from '@/queries';
import { installFakeDaemon } from './daemon';

export interface RenderOptions {
  /** Wrap in a memory router, for anything reaching Link or useNavigate. */
  route?: boolean;
  path?: string;
}

function Providers({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      <LocaleProvider>
        {children}
        <Toaster />
      </LocaleProvider>
    </QueryClientProvider>
  );
}

function routed(ui: ReactElement, path: string) {
  const rootRoute = createRootRoute();
  const indexRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: '/',
    component: () => ui,
  });
  const anywhere = createRoute({
    getParentRoute: () => rootRoute,
    path: '$',
    component: () => null,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([indexRoute, anywhere]),
    history: createMemoryHistory({ initialEntries: [path] }),
    context: { queryClient },
  });
  return <RouterProvider router={router as never} />;
}

export function renderWithProviders(
  ui: ReactElement,
  options: RenderOptions = {},
): RenderResult {
  installFakeDaemon();
  const { route = false, path = '/' } = options;
  return render(<Providers>{route ? routed(ui, path) : ui}</Providers>);
}

export function resetQueryCache(): void {
  queryClient.clear();
}
