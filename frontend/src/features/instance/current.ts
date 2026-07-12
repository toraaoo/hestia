import { getRouteApi } from "@tanstack/react-router";
import { useInstance } from "@/data";
import { orNotFound } from "@/lib/router";
import type { Instance } from "@/lib/types";

const route = getRouteApi("/instance/$instanceId");

/** The instance the current /instance/$instanceId subtree renders. */
export function useCurrentInstance(): Instance {
  const { instanceId } = route.useParams();
  return orNotFound(useInstance(instanceId));
}
