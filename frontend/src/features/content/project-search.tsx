import { MagnifyingGlassIcon, PlusIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import { contentIcon, contentKindLabel } from '@/components/icons';
import { Input } from '@/components/ui/input';
import type { ContentProject } from '@/features/content/mock';
import { contentProjects } from '@/features/content/mock';
import { compact } from '@/lib/format';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

/**
 * An inline project search (the mock stand-in for `content.search`): an input
 * with a results dropdown, each hit picked with one click. `exclude` hides
 * projects already taken (installed / already referenced).
 */
export function ProjectSearch({
  exclude,
  onPick,
  placeholder,
  className,
}: {
  exclude: Set<string>;
  onPick: (project: ContentProject) => void;
  placeholder?: string;
  className?: string;
}) {
  const [query, setQuery] = useState('');
  const q = query.trim().toLowerCase();
  const hits = q
    ? contentProjects
        .filter(
          (p) =>
            !exclude.has(p.id) &&
            (p.title.toLowerCase().includes(q) ||
              p.id.includes(q) ||
              p.author.toLowerCase().includes(q)),
        )
        .slice(0, 6)
    : [];

  return (
    <div className={cn('relative', className)}>
      <MagnifyingGlassIcon className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
      <Input
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={placeholder ?? m['search.placeholder']()}
        className="pl-8"
      />
      {q && (
        <div className="absolute top-full right-0 left-0 z-10 mt-1 border border-border bg-popover shadow-md">
          {hits.length === 0 ? (
            <p className="px-3 py-2.5 text-xs text-muted-foreground">
              {m['profiles.search_empty']({ query: query.trim() })}
            </p>
          ) : (
            <div className="divide-y divide-border">
              {hits.map((project) => {
                const Icon = contentIcon(project.kind);
                return (
                  <button
                    key={project.id}
                    type="button"
                    onClick={() => {
                      onPick(project);
                      setQuery('');
                    }}
                    className="flex w-full items-center gap-2.5 px-3 py-2 text-left transition-colors outline-none hover:bg-muted/60 focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset"
                  >
                    <Icon className="size-4 shrink-0 text-muted-foreground" />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm">
                        {project.title}
                      </span>
                      <span className="block truncate text-[11px] text-muted-foreground">
                        {contentKindLabel[project.kind]()} · {project.author} ·{' '}
                        {compact(project.downloads)}
                      </span>
                    </span>
                    <PlusIcon className="size-3.5 shrink-0 text-muted-foreground" />
                  </button>
                );
              })}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
