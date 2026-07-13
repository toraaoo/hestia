import { createFileRoute } from "@tanstack/react-router";
import { SkinsScreen } from "@/features/skins/skins-screen";

export const Route = createFileRoute("/skins")({
  component: SkinsScreen,
});
