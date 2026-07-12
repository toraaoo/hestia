import { createFileRoute, redirect } from "@tanstack/react-router";
import { useLauncherStore } from "../../lib/store";

export const Route = createFileRoute("/servers/")({
  beforeLoad: () => {
    const first = useLauncherStore.getState().servers[0];
    if (first) {
      // eslint-disable-next-line @typescript-eslint/only-throw-error -- the router catches its own non-Error marker
      throw redirect({ to: "/servers/$serverId", params: { serverId: first.id } });
    }
  },
  component: NoServers,
});

function NoServers() {
  return (
    <section className="flex min-w-0 flex-1 items-center justify-center">
      <p className="text-sm text-text-3">No servers yet. Create one to get hosting.</p>
    </section>
  );
}
