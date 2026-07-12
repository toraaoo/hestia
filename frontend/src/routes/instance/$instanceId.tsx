import { createFileRoute } from "@tanstack/react-router";
import { InstanceScreen } from "@/features/instance/InstanceScreen";

export const Route = createFileRoute("/instance/$instanceId")({
  component: InstanceScreen,
});
