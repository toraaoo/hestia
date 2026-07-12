import { Link, Outlet, createRootRoute } from "@tanstack/react-router";
import type { Instance } from "../lib/types";
import { useLauncherStore } from "../lib/store";
import { TitleBar } from "../components/TitleBar";
import { Sidebar } from "../components/Sidebar";
import { PlayBar } from "../components/PlayBar";
import { ProgressBar } from "../components/ui/ProgressBar";
import { Button } from "../components/ui/Button";
import logoEmber from "../assets/brand/logo-ember.svg";

export const Route = createRootRoute({
  component: RootLayout,
  notFoundComponent: NotFound,
  errorComponent: RouteError,
});

function RootLayout() {
  const launching = useLauncherStore((s) => s.launching);
  return (
    <div className="relative flex h-full flex-col bg-app text-text-1">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <div className="flex min-w-0 flex-1 flex-col">
          <main className="flex min-h-0 flex-1 flex-col">
            <Outlet />
          </main>
          <PlayBar />
        </div>
      </div>
      {launching && <LaunchOverlay instance={launching} />}
    </div>
  );
}

function LaunchOverlay({ instance }: { instance: Instance }) {
  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center bg-ink-950/78 backdrop-blur-xs">
      <div className="w-95 rounded-xl bg-surface-1 p-7 text-center shadow-xl">
        <img
          src={logoEmber}
          alt=""
          className="mx-auto mb-3.5 size-14 animate-flicker rounded-sm motion-reduce:animate-none"
        />
        <div className="mb-1.5 font-hero text-lg text-text-1 font-crisp">
          Launching {instance.name}
        </div>
        <div className="mb-4 font-mono text-xs text-text-3">
          {instance.loader} · {instance.version}
        </div>
        <ProgressBar indeterminate showPct={false} />
      </div>
    </div>
  );
}

function NotFound() {
  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-4">
      <span className="font-hero text-xl text-text-1 font-crisp">Nothing here</span>
      <p className="text-sm text-text-3">That page doesn't exist.</p>
      <Link to="/">
        <Button variant="primary">Back to Library</Button>
      </Link>
    </div>
  );
}

function RouteError({ error }: { error: Error }) {
  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-4">
      <span className="font-hero text-xl text-text-1 font-crisp">Something went wrong</span>
      <p className="max-w-100 text-center font-mono text-xs text-text-3">{error.message}</p>
      <Link to="/">
        <Button variant="primary">Back to Library</Button>
      </Link>
    </div>
  );
}
