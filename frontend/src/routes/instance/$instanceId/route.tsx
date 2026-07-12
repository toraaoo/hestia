import { createFileRoute } from "@tanstack/react-router";
import { InstanceLayout } from "@/features/instance/InstanceLayout";

export const Route = createFileRoute("/instance/$instanceId")({
  component: InstanceLayout,
});
