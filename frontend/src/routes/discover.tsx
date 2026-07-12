import { createFileRoute } from "@tanstack/react-router";
import { DiscoverScreen } from "@/features/discover/DiscoverScreen";

export const Route = createFileRoute("/discover")({
  component: DiscoverScreen,
});
