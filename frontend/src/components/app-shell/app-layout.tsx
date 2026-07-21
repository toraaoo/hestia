import { Outlet, useLocation } from '@tanstack/react-router';

import { FirstRunOverlay } from '@/components/app-shell/first-run-overlay';
import { PlayBar } from '@/components/app-shell/play-bar';
import { SearchProvider } from '@/components/app-shell/search-context';
import { Sidebar } from '@/components/app-shell/sidebar';
import { StatusBar } from '@/components/app-shell/status-bar';
import { TopNav } from '@/components/app-shell/top-nav';
import { LaunchModalProvider } from '@/features/instances/launch-modal';

export function AppLayout() {
  const { pathname } = useLocation();

  return (
    <SearchProvider>
      <LaunchModalProvider>
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
        <FirstRunOverlay />
      </LaunchModalProvider>
    </SearchProvider>
  );
}
