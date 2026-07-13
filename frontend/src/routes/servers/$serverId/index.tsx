import { createFileRoute } from "@tanstack/react-router";
import { ServerConsole } from "@/features/servers/server-console";

export const Route = createFileRoute("/servers/$serverId/")({
  component: ServerConsole,
});
