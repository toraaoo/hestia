import { Link, Outlet, createRootRoute, useRouterState } from "@tanstack/react-router";
import { AnimatePresence, MotionConfig, motion } from "framer-motion";
import type { Instance } from "../lib/types";
import { useLauncherStore } from "../lib/store";
import { backdropVariants, pageVariants, popVariants } from "../lib/motion";
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
  const section = useRouterState({ select: (s) => s.location.pathname.split("/")[1] });
  return (
    <MotionConfig reducedMotion="user">
      <div className="relative flex h-full flex-col bg-app text-text-1">
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
        <AnimatePresence>{launching && <LaunchOverlay instance={launching} />}</AnimatePresence>
      </div>
    </MotionConfig>
  );
}

function LaunchOverlay({ instance }: { instance: Instance }) {
  return (
    <motion.div
      variants={backdropVariants}
      initial="initial"
      animate="animate"
      exit="exit"
      className="absolute inset-0 z-50 flex items-center justify-center bg-ink-950/78 backdrop-blur-xs"
    >
      <motion.div
        variants={popVariants}
        className="w-95 rounded-xl bg-surface-1 p-7 text-center shadow-xl"
      >
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
      </motion.div>
    </motion.div>
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
