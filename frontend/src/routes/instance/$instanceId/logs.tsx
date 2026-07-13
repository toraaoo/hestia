import { createFileRoute } from "@tanstack/react-router";
import { LogsTab } from "@/features/instance/logs-tab";

export const Route = createFileRoute("/instance/$instanceId/logs")({
  component: LogsTab,
});
