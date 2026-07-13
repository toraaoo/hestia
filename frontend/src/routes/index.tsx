import { createFileRoute } from "@tanstack/react-router";
import { LibraryScreen } from "@/features/library/library-screen";

export const Route = createFileRoute("/")({
  component: LibraryScreen,
});
