import { createFileRoute } from "@tanstack/react-router";
import { DiscoverScreen } from "@/features/discover/DiscoverScreen";
import { parseContentKind, type ContentKind } from "@/features/discover/tabs";

export const Route = createFileRoute("/discover")({
  validateSearch: (search): { tab?: ContentKind } => ({
    tab: search.tab === undefined ? undefined : parseContentKind(search.tab),
  }),
  component: DiscoverScreen,
});
