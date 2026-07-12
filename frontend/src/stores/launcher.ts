import { create } from "zustand";
import type { Instance, LibraryView } from "@/lib/types";

/**
 * Global launcher UI state — selection, the launch overlay, view preferences.
 * Unlike the domain data behind data/, this store survives daemon wiring
 * unchanged, so components may use it directly.
 */
interface LauncherState {
  /** The instance the play bar targets; empty falls back to the first instance. */
  selectedId: string;
  /** Non-null while the launch overlay is up. */
  launching: Instance | null;
  libraryView: LibraryView;
  select: (id: string) => void;
  setLaunching: (instance: Instance | null) => void;
  setLibraryView: (view: LibraryView) => void;
}

export const useLauncherStore = create<LauncherState>((set) => ({
  selectedId: "",
  launching: null,
  libraryView: "grid",

  select: (id) => set({ selectedId: id }),
  setLaunching: (instance) => set({ launching: instance }),
  setLibraryView: (view) => set({ libraryView: view }),
}));
