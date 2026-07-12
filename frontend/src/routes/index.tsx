import { createFileRoute } from "@tanstack/react-router";
import { LibraryScreen } from "@/features/library/LibraryScreen";

export const Route = createFileRoute("/")({
  component: LibraryScreen,
});
