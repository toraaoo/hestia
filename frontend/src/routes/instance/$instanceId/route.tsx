import { createFileRoute } from "@tanstack/react-router";
import { InstanceLayout } from "@/features/instance/instance-layout";

export const Route = createFileRoute("/instance/$instanceId")({
  component: InstanceLayout,
});
