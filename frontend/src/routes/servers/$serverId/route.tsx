import { createFileRoute } from "@tanstack/react-router";
import { ServerDetailLayout } from "@/features/servers/server-detail-layout";

export const Route = createFileRoute("/servers/$serverId")({
  component: ServerDetailLayout,
});
