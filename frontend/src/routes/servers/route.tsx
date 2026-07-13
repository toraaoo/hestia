import { createFileRoute } from "@tanstack/react-router";
import { ServersLayout } from "@/features/servers/servers-layout";

export const Route = createFileRoute("/servers")({
  component: ServersLayout,
});
