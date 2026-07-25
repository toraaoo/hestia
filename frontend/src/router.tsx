import { createRouter as createTanStackRouter } from '@tanstack/react-router';
import { logger } from './lib/log';
import { queryClient } from './queries';
import { routeTree } from './routeTree.gen';

const log = logger('router');

export function getRouter() {
  const router = createTanStackRouter({
    routeTree,
    context: { queryClient },
    scrollRestoration: true,
    defaultPreload: 'intent',
    defaultPreloadStaleTime: 0,
  });

  router.subscribe('onResolved', ({ toLocation, fromLocation }) => {
    log.debug(
      { to: toLocation.pathname, from: fromLocation?.pathname },
      'navigated',
    );
  });

  return router;
}

declare module '@tanstack/react-router' {
  interface Register {
    router: ReturnType<typeof getRouter>;
  }
}
