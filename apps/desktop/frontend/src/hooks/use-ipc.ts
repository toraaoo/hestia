// React bindings for the IPC bridge: TanStack Query hooks for request/response
// channels, plus a small effect hook for native -> JS events.

import { useEffect, useRef } from "react"
import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query"

import { config, CONFIG_KEYS, getAppInfo } from "@/lib/api"
import { on } from "@/lib/ipc"

export const ipcKeys = {
  appInfo: ["app", "info"] as const,
  configHome: ["config", CONFIG_KEYS.home] as const,
  autostart: ["config", CONFIG_KEYS.autostart] as const,
}

export function useAppInfo() {
  return useQuery({ queryKey: ipcKeys.appInfo, queryFn: getAppInfo })
}

export function useConfigHome() {
  return useQuery({
    queryKey: ipcKeys.configHome,
    queryFn: () => config.get<string>(CONFIG_KEYS.home),
  })
}

export function useSetConfigHome() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: async (dir: string) => {
      await config.set(CONFIG_KEYS.home, dir)
      return config.get<string>(CONFIG_KEYS.home)
    },
    onSuccess: (path) => queryClient.setQueryData(ipcKeys.configHome, path),
  })
}

export function useSetConfig() {
  return useMutation({
    mutationFn: ({ key, value }: { key: string; value: unknown }) =>
      config.set(key, value),
  })
}

export function useGetConfig() {
  return useMutation({ mutationFn: (key: string) => config.get(key) })
}

export function useAutostartStatus() {
  return useQuery({
    queryKey: ipcKeys.autostart,
    queryFn: () =>
      config.get<boolean>(CONFIG_KEYS.autostart).then((v) => v === true),
  })
}

export function useToggleAutostart() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: async (enable: boolean) => {
      await config.set(CONFIG_KEYS.autostart, enable)
      return enable
    },
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
