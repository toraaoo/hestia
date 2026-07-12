import type { Instance } from "@/lib/types";
import { useInstanceLog } from "@/data";
import { LogLines } from "@/components/ui/LogView";
import { Button } from "@/components/ui/Button";
import { Overline } from "@/components/ui/Overline";
import { Panel } from "@/components/ui/Panel";
import { SectionHeading } from "@/components/ui/SectionHeading";
import { Stat } from "@/components/ui/Stat";
import { DuplicateIcon, ExportIcon, FolderIcon } from "@/components/icons";

export function OverviewTab({ instance }: { instance: Instance }) {
  const log = useInstanceLog(instance.id);

  return (
    <div className="grid grid-cols-[1fr_16.25rem] items-start gap-5.5">
      <div>
        <p className="mb-4.5 text-sm leading-relaxed text-text-2">{instance.description}</p>
        <div className="mb-5 grid grid-cols-3 gap-3">
          {(
            [
              [instance.playtime, "Total playtime"],
              [instance.modCount, "Mods installed"],
              [instance.worldCount, "Worlds"],
            ] as const
          ).map(([value, label]) => (
            <Stat key={label} value={value} label={label} size="lg" className="px-3.5" />
          ))}
        </div>
        <SectionHeading title="Recent activity" as="h3" />
        <Panel variant="inset">
          <div className="max-h-37.5 overflow-y-auto p-3.5 font-mono text-xs leading-relaxed">
            <LogLines lines={log.slice(0, 5)} />
          </div>
        </Panel>
      </div>

      <div className="flex flex-col gap-2.5">
        <Panel className="p-3.5">
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
              <span className="text-text-3">{key}</span>
              <span className="font-medium text-text-1">{value}</span>
            </div>
          ))}
        </Panel>
        <Panel className="p-3.5">
          <Overline className="mb-2.5 block">Quick actions</Overline>
          <div className="flex flex-col gap-2">
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
