import type { Instance, InstalledMod, LogLine, WorldSave } from "@/lib/types";
import { useLauncherStore } from "@/stores/launcher";
import { MOCK_GAME_LOG, MOCK_MODS, MOCK_WORLDS } from "./mock";
import { useDomainStore } from "./store";

const LAUNCH_DURATION_MS = 2600;

export const useInstances = (): Instance[] => useDomainStore((s) => s.instances);

export const useInstance = (id: string): Instance | undefined =>
  useDomainStore((s) => s.instances.find((i) => i.id === id));

/** The play-bar target: the selected instance, falling back to the first one. */
export function useSelectedInstance(): Instance | null {
  const selectedId = useLauncherStore((s) => s.selectedId);
  return useDomainStore(
    (s) => s.instances.find((i) => i.id === selectedId) ?? s.instances[0] ?? null,
  );
}

/** Launch an instance: select it, run the overlay, then mark it running. */
export const usePlay = (): ((instance: Instance) => void) => play;

function play(instance: Instance): void {
  const ui = useLauncherStore.getState();
  ui.select(instance.id);
  if (instance.running || ui.launching) return;
  ui.setLaunching(instance);
  setTimeout(() => {
    if (useLauncherStore.getState().launching?.id !== instance.id) return;
    useLauncherStore.getState().setLaunching(null);
    useDomainStore.getState().markInstanceRunning(instance.id);
  }, LAUNCH_DURATION_MS);
}

export function useInstanceMods(instanceId: string): {
  mods: InstalledMod[];
  toggleMod: (index: number) => void;
} {
  const mods = useDomainStore((s) => s.modsByInstance[instanceId]) ?? MOCK_MODS;
  const toggle = useDomainStore((s) => s.toggleMod);
  return { mods, toggleMod: (index) => toggle(instanceId, index) };
}

export const useInstanceWorlds = (_instanceId: string): WorldSave[] => MOCK_WORLDS;

export const useInstanceLog = (_instanceId: string): readonly LogLine[] => MOCK_GAME_LOG;
