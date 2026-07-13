import { useInstanceLog } from "@/data";
import { LogLines } from "@/components/ui/log-view";
import { Button } from "@/components/ui/button";
import { Overline } from "@/components/ui/overline";
import { Panel } from "@/components/ui/panel";
import { SectionHeading } from "@/components/ui/section-heading";
import { Stat } from "@/components/ui/stat";
import { DuplicateIcon, ExportIcon, FolderIcon } from "@/components/icons";
import { useCurrentInstance } from "./current";

export function OverviewTab() {
  const instance = useCurrentInstance();
  const log = useInstanceLog(instance.id);

  return (
    <div className="grid grid-cols-[1fr_16.25rem] items-start gap-sm">
      <div>
        <p className="mb-4 text-sm leading-relaxed text-fg-2">{instance.description}</p>
        <div className="mb-5 grid grid-cols-3 gap-sm">
          {(
            [
              [instance.playtime, "Total playtime"],
              [instance.modCount, "Mods installed"],
              [instance.worldCount, "Worlds"],
            ] as const
          ).map(([value, label]) => (
            <Stat key={label} value={value} label={label} size="lg" className="px-sm" />
          ))}
        </div>
        <SectionHeading title="Recent activity" as="h3" />
        <Panel variant="inset">
          <div className="max-h-37.5 overflow-y-auto p-sm font-mono text-xs leading-relaxed">
            <LogLines lines={log.slice(0, 5)} />
          </div>
        </Panel>
      </div>

      <div className="flex flex-col gap-sm">
        <Panel className="p-sm">
          <Overline className="mb-2.5 block">Details</Overline>
          {(
            [
              ["Loader", instance.loader],
              ["Version", instance.version],
              ["Size on disk", instance.sizeOnDisk],
              ["Last played", instance.lastPlayed],
            ] as const
          ).map(([key, value]) => (
            <div key={key} className="flex justify-between gap-2.5 py-1.5 text-sm">
              <span className="text-fg-3">{key}</span>
              <span className="font-medium text-fg-1">{value}</span>
            </div>
          ))}
        </Panel>
        <Panel className="p-sm">
          <Overline className="mb-2.5 block">Quick actions</Overline>
          <div className="flex flex-col gap-sm">
            <Button variant="ghost" className="justify-start">
              <FolderIcon size={16} /> Open folder
            </Button>
            <Button variant="ghost" className="justify-start">
              <DuplicateIcon size={16} /> Duplicate
            </Button>
            <Button variant="ghost" className="justify-start">
              <ExportIcon size={16} /> Export
            </Button>
          </div>
        </Panel>
      </div>
    </div>
  );
}
