import { createFileRoute } from "@tanstack/react-router";
import { ServerDetailLayout } from "@/features/servers/ServerDetailLayout";

export const Route = createFileRoute("/servers/$serverId")({
  component: ServerDetailLayout,
});
