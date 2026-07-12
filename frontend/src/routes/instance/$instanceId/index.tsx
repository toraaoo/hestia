import { createFileRoute } from "@tanstack/react-router";
import { OverviewTab } from "@/features/instance/OverviewTab";

export const Route = createFileRoute("/instance/$instanceId/")({
  component: OverviewTab,
});
