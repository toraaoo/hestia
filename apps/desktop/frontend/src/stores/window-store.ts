// Client state for the native window (maximized / minimized), pushed from C++ as
// app.window.state events. Server state stays in TanStack Query; this is local
// UI state several components read, so it lives in a Zustand store.

import { create } from "zustand"

import { windowControls, WINDOW_STATE_EVENT, type WindowState } from "@/lib/api"
import { on } from "@/lib/ipc"

interface WindowStore extends WindowState {
  set: (state: WindowState) => void
}

export const useWindowStore = create<WindowStore>((set) => ({
  maximized: false,
  minimized: false,
  set: (state) => set(state),
}))

// Seed from the native channel and stay in sync with pushed events. Returns an
// unsubscribe; call once from the app shell.
export function initWindowState(): () => void {
  void windowControls.getState().then(useWindowStore.getState().set)
  return on<WindowState>(WINDOW_STATE_EVENT, useWindowStore.getState().set)
}
