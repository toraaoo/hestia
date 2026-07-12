import { useInstanceLog } from "@/data";
import { LogLines } from "@/components/ui/LogView";
import { Panel } from "@/components/ui/Panel";
import { CopyIcon, MenuIcon } from "@/components/icons";
import { useCurrentInstance } from "./current";

export function LogsTab() {
  const instance = useCurrentInstance();
  const log = useInstanceLog(instance.id);

  return (
    <Panel
      variant="inset"
      title={
        <>
          <MenuIcon size={14} />
          latest.log
        </>
      }
      actions={
        <button className="text-text-3 hover:text-hearth-400" title="Copy">
          <CopyIcon size={13} />
        </button>
      }
    >
      <div className="max-h-90 overflow-y-auto p-3.5 font-mono text-xs leading-relaxed">
        <LogLines lines={log} />
      </div>
    </Panel>
  );
}
