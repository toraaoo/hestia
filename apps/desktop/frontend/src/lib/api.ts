// Typed surface for the daemon's IPC channels. The desktop shell forwards these
// to hestiad over the socket (the same channels the CLI drives), except the
// window controls, which the shell handles locally.

import { invoke, IpcError } from "@/lib/ipc"

export type Platform = "windows" | "macos" | "linux"

export interface AppInfo {
  name: string
  id: string
  vendor: string
  version: string
  channel: string
  scheme: string
  platform: Platform
}

const DISCONNECTED_APP_INFO: AppInfo = {
  name: "Hestia (disconnected)",
  id: "—",
  vendor: "—",
  version: "—",
  channel: "—",
  scheme: "—",
  platform: "linux",
}

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app.info", null, { fallback: DISCONNECTED_APP_INFO })
}

export const CONFIG_KEYS = {
  home: "home",
  autostart: "autostart",
} as const

export const config = {
  get: (key: string): Promise<string | null> =>
    invoke<{ value: string }>("settings.config.get", { key })
      .then((r) => r.value)
      .catch((error) => {
        if (error instanceof IpcError) return null
        throw error
      }),
  set: (key: string, value: string): Promise<void> =>
    invoke("settings.config.set", { key, value }).then(() => undefined),
}

export interface WindowState {
  maximized: boolean
  minimized: boolean
}

export const WINDOW_STATE_EVENT = "app.window.state"

const DETACHED_WINDOW_STATE: WindowState = {
  maximized: false,
  minimized: false,
}

export const windowControls = {
  minimize: () => invoke<null>("app.window.minimize", null, { fallback: null }),
  toggleMaximize: () =>
    invoke<null>("app.window.maximize", null, { fallback: null }),
  close: () => invoke<null>("app.window.close", null, { fallback: null }),
  getState: () =>
    invoke<WindowState>("app.window.state", null, {
      fallback: DETACHED_WINDOW_STATE,
    }),
}
