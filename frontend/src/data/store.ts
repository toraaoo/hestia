import { create } from "zustand";
import type { InstalledMod, Instance, Server } from "@/lib/types";
import { MOCK_INSTANCES, MOCK_MODS, MOCK_RUNNING_SERVERS, MOCK_SERVERS } from "./mock";

/**
 * Domain state, private to data/. Screens never touch this store (or the mock
 * fixtures) directly — they consume the domain hooks, so wiring the daemon
 * later swaps this file's internals, not the components.
 */
interface DomainState {
  instances: Instance[];
  servers: Server[];
  /** Live-process state per server id, keyed like the daemon's supervisor. */
  serverRunning: Record<string, boolean>;
  /** Installed content per instance id, seeded from the fixtures on first touch. */
  modsByInstance: Record<string, InstalledMod[]>;
  markInstanceRunning: (id: string) => void;
  setServerRunning: (id: string, up: boolean) => void;
  toggleMod: (instanceId: string, index: number) => void;
}

export const useDomainStore = create<DomainState>((set) => ({
  instances: MOCK_INSTANCES,
  servers: MOCK_SERVERS,
  serverRunning: MOCK_RUNNING_SERVERS,
  modsByInstance: {},

  markInstanceRunning: (id) =>
    set((state) => ({
      instances: state.instances.map((i) =>
        i.id === id ? { ...i, running: true, lastPlayed: "just now" } : i,
      ),
    })),

  setServerRunning: (id, up) =>
    set((state) => ({ serverRunning: { ...state.serverRunning, [id]: up } })),

  toggleMod: (instanceId, index) =>
    set((state) => {
      const mods = state.modsByInstance[instanceId] ?? MOCK_MODS;
      return {
        modsByInstance: {
          ...state.modsByInstance,
          [instanceId]: mods.map((mod, i) =>
            i === index ? { ...mod, enabled: !mod.enabled } : mod,
          ),
        },
      };
    }),
}));
