import { create } from "zustand";
import type { Instance, LibraryView, Server } from "./types";
import { MOCK_INSTANCES, MOCK_RUNNING_SERVERS, MOCK_SERVERS } from "./mock";

const LAUNCH_DURATION_MS = 2600;

interface LauncherState {
  instances: Instance[];
  servers: Server[];
  /** Live-process state per server id, keyed like the daemon's supervisor. */
  serverRunning: Record<string, boolean>;
  /** The instance the play bar targets. */
  selectedId: string;
  /** Non-null while the launch overlay is up. */
  launching: Instance | null;
  libraryView: LibraryView;
  select: (id: string) => void;
  setLibraryView: (view: LibraryView) => void;
  setServerRunning: (id: string, up: boolean) => void;
  play: (instance: Instance) => void;
}

export const useLauncherStore = create<LauncherState>((set, get) => ({
  instances: MOCK_INSTANCES,
  servers: MOCK_SERVERS,
  serverRunning: MOCK_RUNNING_SERVERS,
  selectedId: MOCK_INSTANCES[0]?.id ?? "",
  launching: null,
  libraryView: "grid",

  select: (id) => set({ selectedId: id }),

  setLibraryView: (view) => set({ libraryView: view }),

  setServerRunning: (id, up) =>
    set((state) => ({ serverRunning: { ...state.serverRunning, [id]: up } })),

  play: (instance) => {
    set({ selectedId: instance.id });
    if (instance.running || get().launching) return;
    set({ launching: instance });
    setTimeout(() => {
      if (get().launching?.id !== instance.id) return;
      set((state) => ({
        launching: null,
        instances: state.instances.map((i) =>
          i.id === instance.id ? { ...i, running: true, lastPlayed: "just now" } : i,
        ),
      }));
    }, LAUNCH_DURATION_MS);
  },
}));

export const useSelectedInstance = (): Instance | null =>
  useLauncherStore((s) => s.instances.find((i) => i.id === s.selectedId) ?? null);
