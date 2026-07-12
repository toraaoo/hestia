import type { ContentProject } from "@/lib/types";
import { MOCK_DISCOVER } from "./mock";

export function useContentSearch(query: string): ContentProject[] {
  if (!query) return MOCK_DISCOVER;
  return MOCK_DISCOVER.filter((p) => p.name.toLowerCase().includes(query.toLowerCase()));
}
