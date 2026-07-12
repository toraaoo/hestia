import { useEffect, useState } from "react";
import { Link, createFileRoute } from "@tanstack/react-router";
import type { Instance } from "../../lib/types";
import { TILES } from "../../lib/tiles";
import { useLauncherStore } from "../../lib/store";
import { MOCK_GAME_LOG, MOCK_MODS, MOCK_WORLDS } from "../../lib/mock";
import { loaderTone } from "../../lib/format";
import { orNotFound } from "../../lib/router";
import { LogLines } from "../../components/LogView";
import { Badge } from "../../components/ui/Badge";
import { Button, IconButton } from "../../components/ui/Button";
import { Overline } from "../../components/ui/Overline";
import { Panel } from "../../components/ui/Panel";
import { PlayButton } from "../../components/ui/PlayButton";
import { SectionHeading } from "../../components/ui/SectionHeading";
import { Stat } from "../../components/ui/Stat";
import { Tabs, type TabItem } from "../../components/ui/Tabs";
import { Tile } from "../../components/ui/Tile";
import { Toggle } from "../../components/ui/Toggle";
import { Field, RangeInput, Select, TextInput } from "../../components/ui/form";
import {
  ArrowLeftIcon,
  CopyIcon,
  DuplicateIcon,
  ExportIcon,
  FolderIcon,
  MenuIcon,
  PlayIcon,
  PlusIcon,
  TrashIcon,
} from "../../components/icons";

export const Route = createFileRoute("/instance/$instanceId")({
  component: InstancePage,
});

type Tab = "overview" | "mods" | "worlds" | "screenshots" | "logs" | "settings";

const SHOT_TILES = [
  "tile-sky",
  "tile-grass",
  "tile-ocean",
  "tile-nether",
  "tile-end",
  "tile-diamond",
] as const;

function instanceTabs(instance: Instance): TabItem<Tab>[] {
  return [
    { id: "overview", label: "Overview" },
    { id: "mods", label: "Mods", count: instance.modCount },
    { id: "worlds", label: "Worlds", count: instance.worldCount },
    { id: "screenshots", label: "Screenshots" },
    { id: "logs", label: "Logs" },
    { id: "settings", label: "Settings" },
  ];
}

function InstancePage() {
  const { instanceId } = Route.useParams();
  const instance = orNotFound(
    useLauncherStore((s) => s.instances.find((i) => i.id === instanceId)),
  );
  const select = useLauncherStore((s) => s.select);
  const play = useLauncherStore((s) => s.play);
  const [tab, setTab] = useState<Tab>("overview");

  useEffect(() => {
    select(instance.id);
  }, [instance, select]);

  return (
    <>
      <div className="flex h-13 shrink-0 items-center gap-3 border-b border-border-2 bg-app px-6">
        <Link to="/">
          <IconButton quiet title="Back">
            <ArrowLeftIcon size={18} />
          </IconButton>
        </Link>
        <span className="font-hero text-base text-text-1 font-crisp">
          Library / {instance.name}
        </span>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto">
        <Hero instance={instance} onPlay={play} />

        <Tabs
          items={instanceTabs(instance)}
          value={tab}
          onChange={setTab}
          className="mt-3.5 px-6 pt-4"
        />

        <div className="px-6 pt-5 pb-10">
          {tab === "overview" && <OverviewTab instance={instance} />}
          {tab === "mods" && <ModsTab />}
          {tab === "worlds" && <WorldsTab />}
          {tab === "screenshots" && <ScreenshotsTab />}
          {tab === "logs" && <LogsTab />}
          {tab === "settings" && <SettingsTab instance={instance} />}
        </div>
      </div>
    </>
  );
}

function Hero({ instance, onPlay }: { instance: Instance; onPlay: (i: Instance) => void }) {
  return (
    <div className="relative flex items-end gap-4.5 px-6 pt-6">
      <div
        className="absolute inset-x-0 top-0 h-37.5 bg-size-[34px_34px] opacity-50 pixelated"
        style={{ backgroundImage: `url(${TILES[instance.tile]})` }}
      />
      <div className="absolute inset-x-0 top-0 h-37.5 bg-gradient-to-b from-ink-900/40 to-app" />
      <Tile
        tile={instance.tile}
        rounded="lg"
        className="relative size-24 shadow-md shadow-outline-dark"
      />
      <div className="relative flex min-w-0 flex-1 flex-col gap-2.5 pb-1">
        <h1 className="font-hero text-3xl leading-none tracking-wide text-text-1 font-crisp">
          {instance.name}
        </h1>
        <div className="flex items-center gap-2">
          <Badge tone={loaderTone(instance.loader)}>{instance.loader}</Badge>
          <Badge>{instance.version}</Badge>
          {instance.running && (
            <Badge tone="success" dot>
              Running
            </Badge>
          )}
        </div>
      </div>
      <div className="relative flex items-center gap-2.5 pb-1">
        <IconButton title="Open folder">
          <FolderIcon size={18} />
        </IconButton>
        <PlayButton onClick={() => onPlay(instance)} />
      </div>
    </div>
  );
}

function OverviewTab({ instance }: { instance: Instance }) {
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
            <LogLines lines={MOCK_GAME_LOG.slice(0, 5)} />
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

function ModsTab() {
  const [mods, setMods] = useState(MOCK_MODS);
  const toggleMod = (index: number) =>
    setMods((m) => m.map((mod, i) => (i === index ? { ...mod, enabled: !mod.enabled } : mod)));

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
              <div className="text-sm font-semibold text-text-1">{mod.name}</div>
              <div className="mt-0.5 text-xs text-text-3">{mod.summary}</div>
            </div>
            <div className="flex items-center gap-2">
              <IconButton quiet title="Remove">
                <TrashIcon size={16} />
              </IconButton>
              <Toggle on={mod.enabled} onChange={() => toggleMod(i)} label={`Toggle ${mod.name}`} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function WorldsTab() {
  return (
    <div className="flex flex-col gap-2">
      {MOCK_WORLDS.map((world) => (
        <div
          key={world.name}
          className="flex items-center gap-3 rounded-lg bg-surface-2 px-3.5 py-3 shadow-card-flat"
        >
          <Tile tile={world.tile} className="size-10" />
          <div className="min-w-0 flex-1">
            <div className="text-sm font-semibold text-text-1">{world.name}</div>
            <div className="mt-0.5 text-xs text-text-3">{world.summary}</div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm">
              Backup
            </Button>
            <Button variant="play" size="sm">
              <PlayIcon size={13} weight="fill" /> Play
            </Button>
          </div>
        </div>
      ))}
    </div>
  );
}

function ScreenshotsTab() {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-3">
      {SHOT_TILES.map((tile) => (
        <div
          key={tile}
          className="aspect-video overflow-hidden rounded-lg bg-size-[22px_22px] shadow-card-flat pixelated"
          style={{ backgroundImage: `url(${TILES[tile]})` }}
        />
      ))}
    </div>
  );
}

function LogsTab() {
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
        <LogLines lines={MOCK_GAME_LOG} />
      </div>
    </Panel>
  );
}

function SettingsTab({ instance }: { instance: Instance }) {
  const [memory, setMemory] = useState(instance.memoryGb);
  return (
    <div className="flex max-w-160 flex-col gap-5">
      <Field label="Instance name">
        <TextInput defaultValue={instance.name} />
      </Field>
      <div className="grid grid-cols-2 gap-4">
        <Field label="Minecraft version">
          <Select value={instance.version} />
        </Field>
        <Field label="Mod loader">
          <Select value={`${instance.loader} 0.16.14`} />
        </Field>
      </div>
      <Field
        label={`Allocated memory — ${memory} GB`}
        hint="Recommended: 6 GB for this modpack. Your system has 32 GB."
      >
        <RangeInput
          min={2}
          max={16}
          step={1}
          value={memory}
          onChange={(e) => setMemory(Number(e.target.value))}
        />
      </Field>
      <Field label="Java arguments">
        <TextInput defaultValue="-XX:+UseG1GC -XX:+ParallelRefProcEnabled" className="font-mono" />
      </Field>
      <div className="mt-0.5 border-t border-border-2 pt-4.5">
        <Button variant="danger">
          <TrashIcon size={15} /> Delete instance
        </Button>
      </div>
    </div>
  );
}
