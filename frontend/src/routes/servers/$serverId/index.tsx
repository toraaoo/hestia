import { createFileRoute } from "@tanstack/react-router";
import { ServerConsole } from "@/features/servers/ServerConsole";

export const Route = createFileRoute("/servers/$serverId/")({
  component: ServerConsole,
});
