import { createFileRoute } from "@tanstack/react-router";
import { ScreenshotsTab } from "@/features/instance/screenshots-tab";

export const Route = createFileRoute("/instance/$instanceId/screenshots")({
  component: ScreenshotsTab,
});
