// React bindings for the IPC bridge: TanStack Query hooks for request/response
// channels, plus a small effect hook for native -> JS events.

import { useEffect, useRef } from "react"
import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query"

import { autostart, config, getAppInfo, greet } from "@/lib/api"
import { on } from "@/lib/ipc"

export const ipcKeys = {
  appInfo: ["app", "info"] as const,
  configHome: ["config", "home"] as const,
  autostart: ["autostart", "status"] as const,
}

export function useAppInfo() {
  return useQuery({ queryKey: ipcKeys.appInfo, queryFn: getAppInfo })
}

export function useGreet() {
  return useMutation({ mutationFn: (name: string) => greet(name) })
}

export function useConfigHome() {
  return useQuery({ queryKey: ipcKeys.configHome, queryFn: config.home })
}

export function useSetConfigHome() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (dir: string) => config.setHome(dir),
    onSuccess: (path) => queryClient.setQueryData(ipcKeys.configHome, path),
  })
}

export function useSetConfig() {
  return useMutation({
    mutationFn: ({ key, value }: { key: string; value: string }) =>
      config.set(key, value),
  })
}

export function useGetConfig() {
  return useMutation({ mutationFn: (key: string) => config.get(key) })
}

export function useAutostartStatus() {
  return useQuery({ queryKey: ipcKeys.autostart, queryFn: autostart.status })
}

export function useToggleAutostart() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (enable: boolean) =>
      enable ? autostart.enable() : autostart.disable(),
    onSuccess: (enabled) =>
      queryClient.setQueryData(ipcKeys.autostart, enabled),
  })
}

export function useIpcEvent<TDetail = unknown>(
  channel: string,
  handler: (detail: TDetail) => void
) {
  const handlerRef = useRef(handler)
  useEffect(() => {
    handlerRef.current = handler
  })

  useEffect(() => {
    return on<TDetail>(channel, (detail) => handlerRef.current(detail))
  }, [channel])
}
