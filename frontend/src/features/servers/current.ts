import { getRouteApi } from "@tanstack/react-router";
import { useServer } from "@/data";
import { orNotFound } from "@/lib/router";
import type { Server } from "@/lib/types";

const route = getRouteApi("/servers/$serverId");

/** The server the current /servers/$serverId subtree renders. */
export function useCurrentServer(): Server {
  const { serverId } = route.useParams();
  return orNotFound(useServer(serverId));
}
