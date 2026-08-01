import type { ContentProject, SearchResult } from '@/api';

export const projectKey = (project: ContentProject) =>
  `${project.source}:${project.id}`;

export function mergeHits(
  pages: SearchResult[][] | undefined,
): ContentProject[] {
  const byProject = new Map<string, ContentProject>();
  for (const page of pages ?? []) {
    for (const result of page) {
      for (const hit of result.hits) {
        const id = projectKey(hit);
        if (!byProject.has(id)) byProject.set(id, hit);
      }
    }
  }
  return [...byProject.values()].sort((a, b) => b.downloads - a.downloads);
}
