import type { ContentKind, ContentSource } from '@/api';
import { chipClass } from '@/components/chip';
import { Badge } from '@/components/ui/badge';
import { useContentSources } from '@/queries/content';

/**
 * The sources a kind can be browsed on. The daemon answers only the sources
 * that can serve — a platform whose API key is unset is never among them — so
 * this never offers one that would come back empty.
 */
export function useSourceOptions(kind: ContentKind | undefined, value: string) {
  const sources = useContentSources();
  const list = (sources.data ?? []).filter(
    (s) => !kind || s.kinds.includes(kind),
  );
  // A source carried over from another kind may not catalogue this one; fall
  // back to the default rather than searching a source with nothing to answer.
  const active = list.some((s) => s.id === value) ? value : '';
  return { list, active };
}

/** One chip per source; absent when there is nothing to choose between. */
export function SourceChips({
  list,
  active,
  onChange,
}: {
  list: ContentSource[];
  active: string;
  onChange: (source: string) => void;
}) {
  if (list.length < 2) return null;
  const selected = active || list[0].id;
  return (
    <div className="flex flex-wrap gap-1.5">
      {list.map((source) => (
        <button
          key={source.id}
          type="button"
          className={chipClass(selected === source.id)}
          onClick={() => onChange(source.id)}
        >
          {source.name}
        </button>
      ))}
    </div>
  );
}

/** Which platform a project came from — only worth saying with more than one. */
export function SourceBadge({ source }: { source: string }) {
  const sources = useContentSources();
  const list = sources.data ?? [];
  if (!source || list.length < 2) return null;
  return (
    <Badge variant="outline">
      {list.find((s) => s.id === source)?.name ?? source}
    </Badge>
  );
}
