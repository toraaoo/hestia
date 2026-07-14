import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { createQueryClient, installDaemonInvalidation } from './queries';
import { getRouter } from './router';
import './styles.css';

const router = getRouter();
const queryClient = createQueryClient();
void installDaemonInvalidation(queryClient);

const rootElement = document.getElementById('app');
if (rootElement && !rootElement.innerHTML) {
  createRoot(rootElement).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </StrictMode>,
  );
}
