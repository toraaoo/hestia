import { createFileRoute } from "@tanstack/react-router";
import { ModsTab } from "@/features/instance/mods-tab";

export const Route = createFileRoute("/instance/$instanceId/mods")({
  component: ModsTab,
});
