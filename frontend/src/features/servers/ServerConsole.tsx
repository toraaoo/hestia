import { useIsServerRunning, useServerLog } from "@/data";
import { LogLines } from "@/components/ui/LogView";
import { Panel } from "@/components/ui/Panel";
import { MenuIcon } from "@/components/icons";
import { useCurrentServer } from "./current";

export function ServerConsole() {
  const server = useCurrentServer();
  const isUp = useIsServerRunning(server.id);
  const log = useServerLog(server.id);

  return (
    <Panel
      variant="inset"
      className="flex min-h-0 flex-1 flex-col"
      title={
        <>
          <MenuIcon size={13} />
          Console
        </>
      }
      actions={
        <button className="text-xs font-semibold text-text-3 hover:text-hearth-400">Clear</button>
      }
    >
      <div className="min-h-25 flex-1 overflow-y-auto p-3.5 font-mono text-xs leading-relaxed">
        {isUp ? (
          <LogLines lines={log} />
        ) : (
          <div className="font-body text-sm text-text-3">
            Server is stopped. Press Start to boot it up.
          </div>
        )}
      </div>
      <div className="flex h-11 shrink-0 items-center gap-2 border-t border-border-2 bg-ink-950 px-3.5">
        <span className="font-mono text-sm text-hearth-400">&gt;</span>
        <input
          disabled={!isUp}
          placeholder={isUp ? "Type a command (e.g. /weather clear)…" : "Server offline"}
          className="flex-1 bg-transparent font-mono text-xs text-text-1 outline-none placeholder:text-text-3"
        />
      </div>
    </Panel>
  );
}
