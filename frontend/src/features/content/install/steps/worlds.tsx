import { useQuery } from '@tanstack/react-query';
import { Checkbox } from '@/components/ui/checkbox';
import { WorldIcon } from '@/features/instances/components/world-icon';
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
      {list.map((world) => {
        // Selected and installed by folder — the identity the game reads —
        // while the player picks by the name they gave the world.
        const checked = selected.includes(world.folder);
        const id = `world-${world.folder}`;
        return (
          <label
            key={world.folder}
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
              onCheckedChange={(c) => onToggle(world.folder, c)}
            />
            <WorldIcon world={world} />
            <span className="min-w-0 flex-1 truncate">{world.name}</span>
            <span className="shrink-0 font-mono text-[11px] text-muted-foreground">
              {world.version}
            </span>
          </label>
        );
      })}
    </div>
  );
}
