import { useQuery } from '@tanstack/react-query';
import { Checkbox } from '@/components/ui/checkbox';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { instanceQueries } from '@/queries/instance';

export function WorldsStep({
  instanceId,
  selected,
  onToggle,
}: {
  instanceId: string;
  selected: string[];
  onToggle: (world: string, on: boolean) => void;
}) {
  const query = useQuery(instanceQueries.worlds(instanceId));
  const list = query.data ?? [];

  if (!query.isPending && list.length === 0) {
    return (
      <p className="px-1 py-6 text-center text-xs text-muted-foreground">
        {m['content.no_worlds_in_instance']()}
      </p>
    );
  }
  return (
    <div className="flex flex-col gap-1.5 p-0.5">
      {list.map((w) => {
        const checked = selected.includes(w);
        const id = `world-${w}`;
        return (
          <label
            key={w}
            htmlFor={id}
            className={cn(
              'flex cursor-pointer items-center gap-2.5 border px-3 py-2.5 text-sm transition-colors',
              checked
                ? 'border-ember bg-ember/5'
                : 'border-border hover:bg-muted/60',
            )}
          >
            <Checkbox
              id={id}
              checked={checked}
              onCheckedChange={(c) => onToggle(w, c === true)}
            />
            {w}
          </label>
        );
      })}
    </div>
  );
}
