import { AnimatePresence, motion } from "framer-motion";
import { useLauncherStore } from "@/stores/launcher";
import { backdropVariants, popVariants } from "@/lib/motion";
import { ProgressBar } from "@/components/ui/progress-bar";
import logoEmber from "@/assets/brand/logo-ember.svg";

/** Full-window launching overlay, driven by the store's launching state. */
export function LaunchOverlay() {
  const launching = useLauncherStore((s) => s.launching);

  return (
    <AnimatePresence>
      {launching && (
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
              Launching {launching.name}
            </div>
            <div className="mb-4 font-mono text-xs text-text-3">
              {launching.loader} · {launching.version}
            </div>
            <ProgressBar indeterminate showPct={false} />
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
