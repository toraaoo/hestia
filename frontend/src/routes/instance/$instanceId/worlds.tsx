import { createFileRoute } from "@tanstack/react-router";
import { WorldsTab } from "@/features/instance/worlds-tab";

export const Route = createFileRoute("/instance/$instanceId/worlds")({
  component: WorldsTab,
});
