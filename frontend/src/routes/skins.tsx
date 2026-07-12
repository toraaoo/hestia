import { createFileRoute } from "@tanstack/react-router";
import { SkinsScreen } from "@/features/skins/SkinsScreen";

export const Route = createFileRoute("/skins")({
  component: SkinsScreen,
});
