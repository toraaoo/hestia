import { createFileRoute } from "@tanstack/react-router";
import { ServersLayout } from "@/features/servers/ServersLayout";

export const Route = createFileRoute("/servers")({
  component: ServersLayout,
});
