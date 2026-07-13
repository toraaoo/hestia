import { Link, Outlet, createRootRoute, useRouterState } from "@tanstack/react-router";
import { MotionConfig, motion } from "framer-motion";
import { pageVariants } from "@/lib/motion";
import { TitleBar } from "@/components/layout/title-bar";
import { Sidebar } from "@/components/layout/sidebar";
import { PlayBar } from "@/components/layout/play-bar";
import { LaunchOverlay } from "@/components/layout/launch-overlay";
import { Button } from "@/components/ui/button";

export const Route = createRootRoute({
  component: RootLayout,
  notFoundComponent: NotFound,
  errorComponent: RouteError,
});

function RootLayout() {
  const section = useRouterState({ select: (s) => s.location.pathname.split("/")[1] });
  return (
    <MotionConfig reducedMotion="user">
      <div className="relative flex h-full flex-col bg-app text-fg-1">
        <TitleBar />
        <div className="flex min-h-0 flex-1">
          <Sidebar />
          <div className="flex min-w-0 flex-1 flex-col">
            <main className="flex min-h-0 flex-1 flex-col">
              <motion.div
                key={section}
                variants={pageVariants}
                initial="initial"
                animate="animate"
                className="flex min-h-0 flex-1 flex-col"
              >
                <Outlet />
              </motion.div>
            </main>
            <PlayBar />
          </div>
        </div>
        <LaunchOverlay />
      </div>
    </MotionConfig>
  );
}

function NotFound() {
  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-sm">
      <span className="font-hero text-xl text-fg-1 font-crisp">Nothing here</span>
      <p className="text-sm text-fg-3">That page doesn't exist.</p>
      <Link to="/">
        <Button variant="primary">Back to Library</Button>
      </Link>
    </div>
  );
}

function RouteError({ error }: { error: Error }) {
  return (
    <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-sm">
      <span className="font-hero text-xl text-fg-1 font-crisp">Something went wrong</span>
      <p className="max-w-100 text-center font-mono text-xs text-fg-3">{error.message}</p>
      <Link to="/">
        <Button variant="primary">Back to Library</Button>
      </Link>
    </div>
  );
}
