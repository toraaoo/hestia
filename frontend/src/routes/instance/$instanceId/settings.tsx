import { createFileRoute } from "@tanstack/react-router";
import { SettingsTab } from "@/features/instance/SettingsTab";

export const Route = createFileRoute("/instance/$instanceId/settings")({
  component: SettingsTab,
});
