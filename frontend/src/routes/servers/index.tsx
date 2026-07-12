import { createFileRoute, redirect } from "@tanstack/react-router";
import { getFirstServerId } from "@/data";
import { NoServers } from "@/features/servers/NoServers";

export const Route = createFileRoute("/servers/")({
  beforeLoad: () => {
    const first = getFirstServerId();
    if (first) {
      // eslint-disable-next-line @typescript-eslint/only-throw-error -- the router catches its own non-Error marker
      throw redirect({ to: "/servers/$serverId", params: { serverId: first } });
    }
  },
  component: NoServers,
});
