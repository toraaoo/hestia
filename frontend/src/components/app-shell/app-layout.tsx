import { Outlet, useLocation } from '@tanstack/react-router';

import {
  AnnouncementBanner,
  CriticalAnnouncementDialog,
} from '@/components/app-shell/announcement-surfaces';
import { FirstRunOverlay } from '@/components/app-shell/first-run-overlay';
import { OfflineOverlay } from '@/components/app-shell/offline-overlay';
import { PlayBar } from '@/components/app-shell/play-bar';
import { SearchProvider } from '@/components/app-shell/search-context';
import { Sidebar } from '@/components/app-shell/sidebar';
import { StatusBar } from '@/components/app-shell/status-bar';
import { TopNav } from '@/components/app-shell/top-nav';
import { WhatsNewDialog } from '@/components/app-shell/whats-new-dialog';
import { LaunchDialogProvider } from '@/features/instances/dialogs';

export function AppLayout() {
  const { pathname } = useLocation();

  return (
    <SearchProvider>
      <LaunchDialogProvider>
        <div className="flex h-screen w-screen flex-col overflow-hidden bg-background text-foreground">
          <TopNav />

          <div className="flex min-h-0 flex-1">
            <Sidebar />

            <div className="flex min-w-0 flex-1 flex-col">
              <AnnouncementBanner />
              <main className="flex-1 overflow-y-auto">
                <Outlet />
              </main>
              {pathname === '/' && <PlayBar />}
              <StatusBar />
            </div>
          </div>
        </div>
        <FirstRunOverlay />
        <OfflineOverlay />
        <CriticalAnnouncementDialog />
        <WhatsNewDialog />
      </LaunchDialogProvider>
    </SearchProvider>
  );
}
