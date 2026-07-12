import type { LogLine, Server } from "@/lib/types";
import { MOCK_SERVER_LOG } from "./mock";
import { useDomainStore } from "./store";

export const useServers = (): Server[] => useDomainStore((s) => s.servers);

export const useServer = (id: string): Server | undefined =>
  useDomainStore((s) => s.servers.find((x) => x.id === id));

/** Live-process state per server id. */
export const useServerRunning = (): Record<string, boolean> =>
  useDomainStore((s) => s.serverRunning);

export const useIsServerRunning = (id: string): boolean =>
  useDomainStore((s) => s.serverRunning[id] ?? false);

export const useSetServerRunning = (): ((id: string, up: boolean) => void) =>
  useDomainStore((s) => s.setServerRunning);

export const useServerLog = (_serverId: string): readonly LogLine[] => MOCK_SERVER_LOG;

/** Non-hook lookup for route guards (the servers index redirect). */
export const getFirstServerId = (): string | undefined => useDomainStore.getState().servers[0]?.id;
