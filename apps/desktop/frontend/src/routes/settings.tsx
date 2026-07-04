import { useState } from "react"
import { createFileRoute } from "@tanstack/react-router"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Panel } from "@/components/ui/panel"
import { PageHeader } from "@/components/page-header"
import {
  useAutostartStatus,
  useConfigHome,
  useGetConfig,
  useSetConfig,
  useSetConfigHome,
  useToggleAutostart,
} from "@/hooks/use-ipc"

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
})

function SettingsPage() {
  return (
    <div className="flex flex-col gap-8">
      <PageHeader
        eyebrow="settings"
        title="Configuration & autostart"
        description="The same config store the CLI manages (config get/set), driven through the daemon. Data directory and login-autostart are the reserved keys home and autostart."
      />
      <ConfigPanel />
      <DataHomePanel />
      <AutostartPanel />
    </div>
  )
}

function ConfigPanel() {
  const [key, setKey] = useState("")
  const [value, setValue] = useState("")
  const get = useGetConfig()
  const set = useSetConfig()

  return (
    <Panel label="settings.config">
      <div className="flex flex-col gap-2">
        <div className="flex items-center gap-2">
          <Input
            value={key}
            onChange={(event) => setKey(event.target.value)}
            placeholder="key"
          />
          <Button
            onClick={() =>
              get.mutate(key, { onSuccess: (v) => setValue(v ?? "") })
            }
            disabled={!key || get.isPending}
          >
            get
          </Button>
        </div>
        <div className="flex items-center gap-2">
          <Input
            value={value}
            onChange={(event) => setValue(event.target.value)}
            placeholder="value"
          />
          <Button
            onClick={() => set.mutate({ key, value })}
            disabled={!key || set.isPending}
          >
            set
          </Button>
        </div>
      </div>
      {get.data === null && (
        <p className="mt-3 font-mono text-xs text-muted-foreground">
          key not set
        </p>
      )}
      {(get.error || set.error) && (
        <p className="mt-3 font-mono text-xs text-destructive">
          {(get.error || set.error)?.message}
        </p>
      )}
    </Panel>
  )
}

function DataHomePanel() {
  const home = useConfigHome()
  const setHome = useSetConfigHome()
  const [dir, setDir] = useState("")

  return (
    <Panel label='config "home"'>
      <p className="font-mono text-xs text-foreground">
        {home.isPending ? "loading…" : home.data}
      </p>
      <div className="mt-3 flex items-center gap-2">
        <Input
          value={dir}
          onChange={(event) => setDir(event.target.value)}
          placeholder="new data directory (empty resets)"
        />
        <Button
          onClick={() => setHome.mutate(dir)}
          disabled={setHome.isPending}
        >
          set
        </Button>
      </div>
      {setHome.error && (
        <p className="mt-3 font-mono text-xs text-destructive">
          {setHome.error.message}
        </p>
      )}
    </Panel>
  )
}

function AutostartPanel() {
  const status = useAutostartStatus()
  const toggle = useToggleAutostart()
  const enabled = status.data ?? false

  return (
    <Panel label='config "autostart"'>
      <div className="flex items-center justify-between">
        <span className="font-mono text-xs text-foreground">
          start the daemon at login —{" "}
          <span className={enabled ? "text-signal" : "text-muted-foreground"}>
            {status.isPending ? "…" : enabled ? "enabled" : "disabled"}
          </span>
        </span>
        <Button
          onClick={() => toggle.mutate(!enabled)}
          disabled={status.isPending || toggle.isPending}
        >
          {enabled ? "disable" : "enable"}
        </Button>
      </div>
      {toggle.error && (
        <p className="mt-3 font-mono text-xs text-destructive">
          {toggle.error.message}
        </p>
      )}
    </Panel>
  )
}
