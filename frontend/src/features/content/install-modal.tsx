import {
  CaretLeftIcon,
  CaretRightIcon,
  CheckIcon,
  MagnifyingGlassIcon,
} from '@phosphor-icons/react';
import { useEffect, useMemo, useRef, useState } from 'react';

import { chipClass } from '@/components/chip';
import { contentIcon, contentKindLabel, entryIcon } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Combobox,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
} from '@/components/ui/combobox';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from '@/components/ui/progress';
import { kindInfo } from '@/features/content/kinds';
import {
  type ContentProject,
  type ContentVersion,
  contentProjects,
  projectVersions,
  resolveDependencies,
} from '@/features/content/mock';
import {
  type Instance,
  instances,
  type Server,
  servers,
} from '@/features/entries/mock';
import { agoLabel } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';
import { cn } from '@/lib/utils';

/** An entry the content can be installed into, drawn from both stores. */
export interface Target {
  id: string;
  name: string;
  type: 'server' | 'instance';
  flavor: string;
  game_version: string;
  running: boolean;
  worlds: string[];
}

export const serverTarget = (s: Server): Target => ({
  id: s.id,
  name: s.name,
  type: 'server',
  flavor: s.flavor,
  game_version: s.game_version,
  running: s.running,
  worlds: [],
});

export const instanceTarget = (i: Instance): Target => ({
  id: i.id,
  name: i.name,
  type: 'instance',
  flavor: i.flavor,
  game_version: i.game_version,
  running: i.running,
  worlds: i.worlds,
});

/** Which kinds each entry type accepts — mirrors the daemon's install surface. */
const ACCEPTS: Record<Target['type'], ContentKind[]> = {
  server: ['mod', 'datapack'],
  instance: ['mod', 'resourcepack', 'shader', 'datapack'],
};

/** A mod needs a loader; a vanilla entry cannot take one. */
const targetTakesKind = (t: Target, kind: ContentKind): boolean =>
  ACCEPTS[t.type].includes(kind) && (kind !== 'mod' || t.flavor === 'fabric');

/** Every entry that can take this kind, across both stores. */
function targetsFor(kind: ContentKind): Target[] {
  return [
    ...servers.map(serverTarget),
    ...instances.map(instanceTarget),
  ].filter((t) => targetTakesKind(t, kind));
}

/** The projects an entry can take — its accepted kinds, loader-aware for mods. */
function projectsFor(target: Target): ContentProject[] {
  return contentProjects.filter((p) => targetTakesKind(target, p.kind));
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * The content install wizard, mirroring the daemon's `content.add`. It opens
 * either way round: from Browse a `project` is fixed and the user picks a target
 * entry; from an entry's page the `entry` is fixed and the user picks a project.
 * The version auto-resolves to the newest compatible build (changeable in the
 * review), datapacks on an instance choose their worlds, and required
 * dependencies are pulled in — then the install job plays as progress. Nothing
 * is persisted; the library is static mock data.
 */
export function ContentInstallModal({
  project,
  entry,
  versionId,
  open,
  onOpenChange,
}: {
  project?: ContentProject;
  entry?: Target;
  versionId?: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const mode: 'browse' | 'entry' = project ? 'browse' : 'entry';
  const pickStep = mode === 'browse' ? 'target' : 'content';

  const [proj, setProj] = useState<ContentProject | null>(project ?? null);
  const [target, setTarget] = useState<Target | null>(entry ?? null);
  const [version, setVersion] = useState<ContentVersion | null>(null);
  const [worlds, setWorlds] = useState<string[]>([]);
  const [installing, setInstalling] = useState(false);
  const [phase, setPhase] = useState('');
  const [progress, setProgress] = useState(0);

  const versions = useMemo(() => (proj ? projectVersions(proj) : []), [proj]);
  const deps = useMemo(
    () => (proj ? resolveDependencies(proj.id) : []),
    [proj],
  );
  const resolved = version ?? versions[0];

  const needsWorlds = proj?.kind === 'datapack' && target?.type === 'instance';
  const steps: readonly string[] = needsWorlds
    ? [pickStep, 'worlds', 'review']
    : [pickStep, 'review'];
  const [step, setStep] = useState<string>(pickStep);

  useEffect(() => {
    if (!open) return;
    setProj(project ?? null);
    setTarget(entry ?? null);
    setVersion(null);
    setWorlds([]);
    setInstalling(false);
    setProgress(0);
    setStep(pickStep);
  }, [open, project, entry, pickStep]);

  // A preselected version (Browse's per-version Install) becomes the choice.
  useEffect(() => {
    if (open && versionId && versions.length) {
      setVersion(versions.find((v) => v.id === versionId) ?? null);
    }
  }, [open, versionId, versions]);

  const stepIndex = steps.indexOf(step);
  const back = () => setStep(steps[Math.max(0, stepIndex - 1)]);
  const next = () => setStep(steps[Math.min(steps.length - 1, stepIndex + 1)]);

  const canAdvance =
    step === 'target'
      ? !!target
      : step === 'content'
        ? !!proj
        : step === 'worlds'
          ? worlds.length > 0
          : true;

  const install = async () => {
    if (!proj) return;
    setInstalling(true);
    const files = [proj, ...deps];
    setPhase('Resolving dependencies');
    setProgress(4);
    await sleep(600);
    for (let i = 0; i < files.length; i++) {
      setPhase(`Downloading ${files[i].title}`);
      setProgress(Math.round(((i + 1) / (files.length + 1)) * 90) + 4);
      await sleep(650);
    }
    setPhase('Mirroring into data/');
    setProgress(97);
    await sleep(500);
    setPhase('Ready');
    setProgress(100);
    await sleep(400);
    onOpenChange(false);
    setInstalling(false);
  };

  const Icon =
    mode === 'browse' && project
      ? contentIcon(project.kind)
      : entryIcon(entry?.type ?? 'instance');
  const title =
    mode === 'browse'
      ? `Install ${project?.title}`
      : `Add content to ${entry?.name}`;

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!installing) onOpenChange(o);
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Icon className="size-4.5 text-muted-foreground" />
            {title}
          </DialogTitle>
          <DialogDescription>
            {installing
              ? 'Installing…'
              : step === 'target'
                ? 'Choose where this content is installed.'
                : step === 'content'
                  ? 'Choose the content to install.'
                  : step === 'worlds'
                    ? 'Choose the worlds to add this datapack to.'
                    : 'Review and install.'}
          </DialogDescription>
        </DialogHeader>

        <div className="max-h-[58vh] min-h-[18rem] overflow-x-hidden overflow-y-auto p-1">
          {installing ? (
            <div className="flex min-h-[18rem] flex-col justify-center px-1">
              <Progress value={progress}>
                <ProgressLabel>{phase}</ProgressLabel>
                <ProgressValue />
              </Progress>
            </div>
          ) : step === 'target' && project ? (
            <TargetStep
              kind={project.kind}
              targets={targetsFor(project.kind)}
              selected={target}
              onSelect={(t) => {
                setTarget(t);
                setWorlds([]);
              }}
            />
          ) : step === 'content' && entry ? (
            <ContentStep
              entry={entry}
              projects={projectsFor(entry)}
              selected={proj}
              onSelect={(p) => {
                setProj(p);
                setVersion(null);
                setWorlds([]);
              }}
            />
          ) : step === 'worlds' ? (
            <div className="flex flex-col gap-1.5">
              {target?.worlds.length ? (
                target.worlds.map((w) => (
                  <WorldRow
                    key={w}
                    world={w}
                    checked={worlds.includes(w)}
                    onToggle={(on) =>
                      setWorlds((prev) =>
                        on ? [...prev, w] : prev.filter((x) => x !== w),
                      )
                    }
                  />
                ))
              ) : (
                <p className="px-1 py-6 text-center text-xs text-muted-foreground">
                  This instance has no worlds yet.
                </p>
              )}
            </div>
          ) : (
            proj &&
            resolved && (
              <ReviewStep
                project={proj}
                versions={versions}
                version={resolved}
                explicit={version}
                onVersion={setVersion}
                target={target}
                worlds={needsWorlds ? worlds : undefined}
                deps={deps}
              />
            )
          )}
        </div>

        {!installing && (
          <DialogFooter className="items-center">
            <StepDots steps={steps} active={stepIndex} className="mr-auto" />
            {stepIndex === 0 ? (
              <Button variant="outline" onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
            ) : (
              <Button variant="outline" onClick={back} data-icon="inline-start">
                <CaretLeftIcon />
                Back
              </Button>
            )}

            {step === 'review' ? (
              <Button
                className="bg-ember text-ember-foreground hover:bg-ember/90"
                onClick={install}
              >
                Install
              </Button>
            ) : (
              <Button
                disabled={!canAdvance}
                onClick={next}
                data-icon="inline-end"
                className="bg-ember text-ember-foreground hover:bg-ember/90"
              >
                Next
                <CaretRightIcon />
              </Button>
            )}
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
  );
}

/** A search box over a chip-filtered list — shared by both pick steps. */
function FilterBar({
  search,
  onSearch,
  placeholder,
  chips,
}: {
  search: string;
  onSearch: (value: string) => void;
  placeholder: string;
  chips?: { label: string; active: boolean; onClick: () => void }[];
}) {
  return (
    <div className="mb-3 flex flex-col gap-2.5">
      <div className="relative">
        <MagnifyingGlassIcon className="-translate-y-1/2 absolute top-1/2 left-2.5 size-3.5 text-muted-foreground" />
        <Input
          className="pl-8"
          placeholder={placeholder}
          value={search}
          onChange={(e) => onSearch(e.target.value)}
        />
      </div>
      {chips && chips.length > 1 && (
        <div className="flex flex-wrap gap-1.5">
          {chips.map((c) => (
            <button
              key={c.label}
              type="button"
              className={chipClass(c.active)}
              onClick={c.onClick}
            >
              {c.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function TargetStep({
  kind,
  targets,
  selected,
  onSelect,
}: {
  kind: ContentKind;
  targets: Target[];
  selected: Target | null;
  onSelect: (target: Target) => void;
}) {
  const [search, setSearch] = useState('');
  const [type, setType] = useState<'all' | Target['type']>('all');

  const q = search.trim().toLowerCase();
  const shown = targets.filter((t) => {
    if (type !== 'all' && t.type !== type) return false;
    return !q || t.name.toLowerCase().includes(q);
  });

  return (
    <div>
      <FilterBar
        search={search}
        onSearch={setSearch}
        placeholder="Search servers and instances"
        chips={[
          {
            label: 'All',
            active: type === 'all',
            onClick: () => setType('all'),
          },
          {
            label: 'Servers',
            active: type === 'server',
            onClick: () => setType('server'),
          },
          {
            label: 'Instances',
            active: type === 'instance',
            onClick: () => setType('instance'),
          },
        ]}
      />
      {targets.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          No server or instance can take a{' '}
          {contentKindLabel[kind].toLowerCase()}.
        </p>
      ) : shown.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          Nothing matches your search.
        </p>
      ) : (
        <div className="grid gap-2">
          {shown.map((t) => (
            <PickRow
              key={t.id}
              icon={entryIcon(t.type)}
              title={t.name}
              subtitle={`${t.type} · ${t.flavor} · ${t.game_version}`}
              badge={t.running ? 'Stop to install' : undefined}
              disabled={t.running}
              selected={selected?.id === t.id}
              onSelect={() => onSelect(t)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function ContentStep({
  entry,
  projects,
  selected,
  onSelect,
}: {
  entry: Target;
  projects: ContentProject[];
  selected: ContentProject | null;
  onSelect: (project: ContentProject) => void;
}) {
  const [search, setSearch] = useState('');
  const [kind, setKind] = useState<'all' | ContentKind>('all');

  const kinds = ACCEPTS[entry.type].filter(
    (k) => k !== 'mod' || entry.flavor === 'fabric',
  );
  const q = search.trim().toLowerCase();
  const shown = projects.filter((p) => {
    if (kind !== 'all' && p.kind !== kind) return false;
    return (
      !q ||
      p.title.toLowerCase().includes(q) ||
      p.author.toLowerCase().includes(q)
    );
  });

  return (
    <div>
      <FilterBar
        search={search}
        onSearch={setSearch}
        placeholder="Search Modrinth"
        chips={[
          {
            label: 'All',
            active: kind === 'all',
            onClick: () => setKind('all'),
          },
          ...kinds.map((k) => ({
            label: kindInfo[k].label,
            active: kind === k,
            onClick: () => setKind(k),
          })),
        ]}
      />
      {shown.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          Nothing matches your search.
        </p>
      ) : (
        <div className="grid gap-2">
          {shown.map((p) => (
            <PickRow
              key={p.id}
              icon={contentIcon(p.kind)}
              title={p.title}
              subtitle={`${contentKindLabel[p.kind]} · by ${p.author}`}
              selected={selected?.id === p.id}
              onSelect={() => onSelect(p)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

/** A selectable row shared by the target and content pick lists. */
function PickRow({
  icon: Icon,
  title,
  subtitle,
  badge,
  disabled,
  selected,
  onSelect,
}: {
  icon: typeof CheckIcon;
  title: string;
  subtitle: string;
  badge?: string;
  disabled?: boolean;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      disabled={disabled}
      onClick={onSelect}
      className={cn(
        'flex items-center gap-3 p-3 text-left ring-1 transition-colors outline-none focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-55',
        selected
          ? 'bg-muted ring-ember'
          : 'ring-border hover:bg-muted/60 hover:ring-foreground/20',
      )}
    >
      <Icon className="size-4.5 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-sm font-medium">{title}</span>
          {badge && (
            <Badge variant="outline" className="shrink-0">
              {badge}
            </Badge>
          )}
        </div>
        <div className="truncate font-mono text-[11px] text-muted-foreground">
          {subtitle}
        </div>
      </div>
      {selected && <CheckIcon weight="bold" className="size-4 text-ember" />}
    </button>
  );
}

function WorldRow({
  world,
  checked,
  onToggle,
}: {
  world: string;
  checked: boolean;
  onToggle: (on: boolean) => void;
}) {
  const id = `world-${world}`;
  return (
    <label
      htmlFor={id}
      className={cn(
        'flex cursor-pointer items-center gap-2.5 px-3 py-2.5 text-sm ring-1 transition-colors',
        checked ? 'bg-muted ring-ember' : 'ring-border hover:bg-muted/60',
      )}
    >
      <Checkbox
        id={id}
        checked={checked}
        onCheckedChange={(c) => onToggle(c === true)}
      />
      {world}
    </label>
  );
}

function ReviewStep({
  project,
  versions,
  version,
  explicit,
  onVersion,
  target,
  worlds,
  deps,
}: {
  project: ContentProject;
  versions: ContentVersion[];
  version: ContentVersion;
  explicit: ContentVersion | null;
  onVersion: (version: ContentVersion | null) => void;
  target: Target | null;
  worlds?: string[];
  deps: ContentProject[];
}) {
  return (
    <div className="flex flex-col gap-4 p-1">
      <div className="divide-y divide-border border border-border">
        <ReviewRow label="Content" value={project.title} />
        <ReviewRow label="Target" value={target?.name ?? '—'} />
        {worlds && (
          <ReviewRow
            label="Worlds"
            value={worlds.length ? worlds.join(', ') : 'none selected'}
          />
        )}
        <div className="flex items-center justify-between gap-4 px-3 py-2 text-sm">
          <span className="text-xs text-muted-foreground">Version</span>
          <div className="flex items-center gap-2">
            {!explicit && (
              <Badge variant="secondary" className="shrink-0">
                latest
              </Badge>
            )}
            <VersionCombobox
              versions={versions}
              value={version}
              onChange={onVersion}
            />
          </div>
        </div>
      </div>

      {deps.length > 0 && (
        <div>
          <p className="mb-2 text-[10px] font-semibold tracking-wide text-muted-foreground uppercase">
            Dependencies ({deps.length})
          </p>
          <div className="divide-y divide-border border border-border">
            {deps.map((d) => {
              const Icon = contentIcon(d.kind);
              return (
                <div
                  key={d.id}
                  className="flex items-center gap-2.5 px-3 py-2 text-xs"
                >
                  <Icon className="size-4 shrink-0 text-muted-foreground" />
                  <span className="flex-1 truncate">{d.title}</span>
                  <span className="text-muted-foreground">required</span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

/** The searchable version picker — a combobox so the review never resizes. */
function VersionCombobox({
  versions,
  value,
  onChange,
}: {
  versions: ContentVersion[];
  value: ContentVersion;
  onChange: (version: ContentVersion | null) => void;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  const latestId = versions[0]?.id;
  return (
    <div ref={rootRef} className="contents">
      <Combobox
        items={versions}
        value={value}
        onValueChange={(v: ContentVersion | null) => {
          onChange(v && v.id === latestId ? null : v);
          // Drop focus once Base UI has returned it to the input on close.
          requestAnimationFrame(() =>
            rootRef.current?.querySelector('input')?.blur(),
          );
        }}
        itemToStringLabel={(v: ContentVersion) => v.version_number}
        itemToStringValue={(v: ContentVersion) => v.version_number}
      >
        <ComboboxInput placeholder="Select version" className="w-48" />
        <ComboboxContent>
          <ComboboxEmpty>No versions.</ComboboxEmpty>
          <ComboboxList>
            {(v: ContentVersion) => (
              <ComboboxItem key={v.id} value={v}>
                <div className="flex min-w-0 flex-col">
                  <span className="flex items-center gap-1.5">
                    {v.version_number}
                    {v.id === latestId && (
                      <Badge variant="secondary" className="text-[10px]">
                        latest
                      </Badge>
                    )}
                    {v.channel !== 'release' && (
                      <Badge
                        variant="outline"
                        className="text-[10px] capitalize"
                      >
                        {v.channel}
                      </Badge>
                    )}
                  </span>
                  <span className="truncate font-mono text-[11px] text-muted-foreground">
                    {v.game_versions.join(', ')} · {agoLabel(v.published_unix)}
                  </span>
                </div>
              </ComboboxItem>
            )}
          </ComboboxList>
        </ComboboxContent>
      </Combobox>
    </div>
  );
}

function ReviewRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 px-3 py-2 text-sm">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className="truncate text-right">{value}</span>
    </div>
  );
}

function StepDots({
  steps,
  active,
  className,
}: {
  steps: readonly string[];
  active: number;
  className?: string;
}) {
  return (
    <div className={cn('flex items-center gap-1.5', className)}>
      {steps.map((s, i) => (
        <span
          key={s}
          className={cn(
            'h-1.5 rounded-full transition-all',
            i === active ? 'w-4 bg-ember' : 'w-1.5 bg-border',
          )}
        />
      ))}
    </div>
  );
}
