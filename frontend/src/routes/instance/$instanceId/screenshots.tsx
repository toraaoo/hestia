import { createFileRoute } from "@tanstack/react-router";
import { ScreenshotsTab } from "@/features/instance/ScreenshotsTab";

export const Route = createFileRoute("/instance/$instanceId/screenshots")({
  component: ScreenshotsTab,
});
