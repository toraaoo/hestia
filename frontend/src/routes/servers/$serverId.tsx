import { createFileRoute } from "@tanstack/react-router";
import { ServerDetail } from "@/features/servers/ServerDetail";

export const Route = createFileRoute("/servers/$serverId")({
  component: ServerDetail,
});
