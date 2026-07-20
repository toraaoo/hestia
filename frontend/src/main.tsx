import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { LocaleProvider } from './hooks/locale';
import { initDesktopShell } from './lib/desktop';
import { queryClient, startInvalidation } from './queries';
import { startSessionTracking } from './queries/sessions';
import { getRouter } from './router';
import './styles.css';

initDesktopShell();
startInvalidation();
startSessionTracking();
const router = getRouter();

const rootElement = document.getElementById('app');
if (rootElement && !rootElement.innerHTML) {
  createRoot(rootElement).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <LocaleProvider>
          <RouterProvider router={router} />
        </LocaleProvider>
      </QueryClientProvider>
    </StrictMode>,
  );
}
