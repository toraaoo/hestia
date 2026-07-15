import { createFileRoute, Outlet, useLocation } from '@tanstack/react-router';

import { PlayBar } from '@/components/launcher/play-bar';
import { SearchProvider } from '@/components/launcher/search-context';
import { Sidebar } from '@/components/launcher/sidebar';
import { StatusBar } from '@/components/launcher/status-bar';
import { TopNav } from '@/components/launcher/top-nav';

export const Route = createFileRoute('/_app')({ component: AppLayout });

function AppLayout() {
  const { pathname } = useLocation();

  return (
    <SearchProvider>
      <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
        <TopNav />

        <div className="flex min-h-0 flex-1">
          <Sidebar />

          <div className="flex min-w-0 flex-1 flex-col">
            <main className="flex-1 overflow-y-auto">
              <Outlet />
            </main>
            {pathname === '/' && <PlayBar />}
            <StatusBar />
          </div>
        </div>
      </div>
    </SearchProvider>
  );
}
