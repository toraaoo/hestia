import { createFileRoute } from "@tanstack/react-router";
import { OverviewTab } from "@/features/instance/overview-tab";

export const Route = createFileRoute("/instance/$instanceId/")({
  component: OverviewTab,
});
