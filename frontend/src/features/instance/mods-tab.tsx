import { useInstanceMods } from "@/data";
import { Button, IconButton } from "@/components/ui/button";
import { SectionHeading } from "@/components/ui/section-heading";
import { Tile } from "@/components/ui/tile";
import { Switch } from "@/components/ui/switch";
import { PlusIcon, TrashIcon } from "@/components/icons";
import { useCurrentInstance } from "./current";

export function ModsTab() {
  const instance = useCurrentInstance();
  const { mods, toggleMod } = useInstanceMods(instance.id);

  return (
    <div>
      <SectionHeading
        title={`${mods.filter((m) => m.enabled).length} of ${mods.length} enabled`}
        as="h3"
        action={
          <Button variant="primary" size="sm">
            <PlusIcon size={14} /> Add mods
          </Button>
        }
      />
      <div className="flex flex-col gap-2">
        {mods.map((mod, i) => (
          <div
            key={mod.name}
            className="flex items-center gap-3 rounded-lg bg-surface-2 px-3.5 py-3 shadow-card-flat"
          >
            <Tile tile={mod.tile} className="size-10" />
            <div className="min-w-0 flex-1">
              <div className="text-sm font-semibold text-fg-1">{mod.name}</div>
              <div className="mt-0.5 text-xs text-fg-3">{mod.summary}</div>
            </div>
            <div className="flex items-center gap-2">
              <IconButton quiet title="Remove">
                <TrashIcon size={16} />
              </IconButton>
              <Switch
                checked={mod.enabled}
                onCheckedChange={() => toggleMod(i)}
                aria-label={`Toggle ${mod.name}`}
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
