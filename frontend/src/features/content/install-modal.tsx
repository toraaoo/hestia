import {
  CaretLeftIcon,
  CaretRightIcon,
  MagnifyingGlassIcon,
  UploadSimpleIcon,
} from '@phosphor-icons/react';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState } from 'react';

import {
  type ContentKind,
  type ContentProject,
  type ContentVersion,
  dialog,
  type GlobalProfile,
  type InstanceInfo,
  instance as instanceApi,
  type ProvisionProgress,
  profile as profileApi,
  type ServerInfo,
  server as serverApi,
} from '@/api';
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
import { projectRef } from '@/features/content/content-card';
import { kindInfo } from '@/features/content/kinds';
import { PickRow } from '@/features/content/pick-row';
import { agoLabel } from '@/lib/format';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { contentQueries } from '@/queries/content';
import { useInstances } from '@/queries/instance';
import { useGlobalProfiles } from '@/queries/profile';
import { useServers } from '@/queries/server';

/** An entry the content can be installed into, drawn from every store. */
export interface Target {
  id: string;
  name: string;
  type: 'server' | 'instance' | 'profile';
  flavor: string;
  gameVersion: string;
  running: boolean;
}

export const serverTarget = (s: ServerInfo): Target => ({
  id: s.id,
  name: s.name,
  type: 'server',
  flavor: s.flavor,
  gameVersion: s.gameVersion,
  running: s.process?.state === 'running',
});

export const instanceTarget = (i: InstanceInfo): Target => ({
  id: i.id,
  name: i.name,
  type: 'instance',
  flavor: i.flavor,
  gameVersion: i.gameVersion,
  running: (i.sessions ?? []).some((s) => s.state === 'running'),
});

/**
 * A global profile as an install target: references, never jars — a profile
 * has no version or loader of its own, so anything compatible can join it.
 */
export const profileTarget = (p: GlobalProfile): Target => ({
  id: p.name,
  name: p.name,
  type: 'profile',
  flavor: '',
  gameVersion: '',
  running: false,
});

/** Which kinds each entry type accepts — mirrors the daemon's install surface. */
const ACCEPTS: Record<Target['type'], ContentKind[]> = {
  profile: ['mod', 'resource_pack', 'shader'],
  server: ['mod', 'data_pack'],
  instance: ['mod', 'resource_pack', 'shader', 'data_pack'],
};

/** A mod needs a loader; a vanilla entry cannot take one. */
const targetTakesKind = (t: Target, kind: ContentKind): boolean =>
  ACCEPTS[t.type].includes(kind) &&
  (kind !== 'mod' || t.flavor === 'fabric' || t.type === 'profile');

const entryTypeLabel = (type: Target['type']): string =>
  type === 'server'
    ? m['entry.type_server']()
    : type === 'profile'
      ? m['entry.type_profile']()
      : m['entry.type_instance']();

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** Every entry, from all three stores, merged into a common target shape. */
function useTargets(): Target[] {
  const servers = useServers();
  const instances = useInstances();
  const profiles = useGlobalProfiles();
  return useMemo(
    () => [
      ...(servers.data ?? []).map(serverTarget),
      ...(instances.data ?? []).map(instanceTarget),
      ...(profiles.data ?? []).map(profileTarget),
    ],
    [servers.data, instances.data, profiles.data],
  );
}

/**
 * The content install modal over the daemon's `content.add` (and, for a global
 * profile, `profile.edit`). It opens either way round: from Browse a `project`
 * is fixed and the user picks a target; from an entry's page the `entry` is
 * fixed and the user picks a project. The newest compatible version resolves
 * automatically (changeable in the review), a datapack on an instance chooses
 * its worlds, and the install runs as a job with live progress.
 */
export function ContentInstallModal({
  project,
  entry,
  open,
  onOpenChange,
}: {
  project?: ContentProject;
  entry?: Target;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const mode: 'browse' | 'entry' = project ? 'browse' : 'entry';
  const targets = useTargets();

  const [step, setStep] = useState(0);
  const [targetId, setTargetId] = useState(entry?.id ?? '');
  const [picked, setPicked] = useState<ContentProject | null>(project ?? null);
  const [file, setFile] = useState<string | null>(null);
  const [kindFilter, setKindFilter] = useState<ContentKind | null>(null);
  const [versionId, setVersionId] = useState('');
  const [worlds, setWorlds] = useState<string[]>([]);

  const [installing, setInstalling] = useState(false);
  const [phase, setPhase] = useState('');
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState('');

  // Reset when (re)opened, keeping whichever side the caller fixed.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset only on open.
  useEffect(() => {
    if (!open) return;
    setStep(0);
    setTargetId(entry?.id ?? '');
    setPicked(project ?? null);
    setFile(null);
    setKindFilter(null);
    setVersionId('');
    setWorlds([]);
    setInstalling(false);
    setProgress(0);
    setPhase('');
    setError('');
  }, [open]);

  const target = targets.find((t) => t.id === targetId) ?? entry ?? null;
  // In entry mode the kind comes from the active filter (which scopes search);
  // in browse mode it is the project's own kind.
  const kind: ContentKind | null =
    mode === 'browse' ? (picked?.kind ?? null) : kindFilter;
  const needsWorlds = kind === 'data_pack' && target?.type === 'instance';
  const isProfile = target?.type === 'profile';

  const pickStep = mode === 'browse' ? 'target' : 'content';
  const steps: string[] =
    needsWorlds && !isProfile
      ? [pickStep, 'worlds', 'review']
      : [pickStep, 'review'];
  const stepId = steps[Math.min(step, steps.length - 1)];

  const Icon =
    mode === 'browse' && picked
      ? contentIcon(picked.kind)
      : entryIcon(entry?.type ?? 'instance');
  const title =
    mode === 'browse'
      ? m['content.install_title']({ name: picked?.title ?? '' })
      : m['content.add_to_title']({ name: entry?.name ?? '' });

  const hint = installing
    ? m['content.installing']()
    : stepId === 'target'
      ? m['content.hint_target']()
      : stepId === 'content'
        ? m['content.hint_content']()
        : stepId === 'worlds'
          ? m['content.hint_worlds']()
          : m['content.hint_review']();

  const canAdvance =
    stepId === 'target'
      ? !!targetId
      : stepId === 'content'
        ? !!picked || !!file
        : stepId === 'worlds'
          ? worlds.length > 0
          : true;

  async function install() {
    if (!target) return;
    setInstalling(true);
    setError('');
    setProgress(0);
    const onProgress = (p: ProvisionProgress) => {
      setPhase(p.detail || p.phase);
      setProgress(p.total > 0 ? Math.round((p.current / p.total) * 100) : 0);
    };
    try {
      if (isProfile && picked) {
        await profileApi.edit(target.name, { add: [projectRef(picked)] });
      } else if (target.type !== 'profile') {
        const items = file
          ? [{ path: file }]
          : [
              {
                project: projectRef(picked as ContentProject),
                version: versionId,
              },
            ];
        const spec = {
          kind: kind as ContentKind,
          items,
          worlds: needsWorlds ? worlds : [],
        };
        const add =
          target.type === 'server'
            ? serverApi.content.add
            : instanceApi.content.add;
        await add(target.id, spec, onProgress);
      }
      onOpenChange(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setInstalling(false);
    }
  }

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
          <DialogDescription>{hint}</DialogDescription>
        </DialogHeader>

        {installing ? (
          <div className="flex min-h-[18rem] flex-col justify-center px-1">
            <Progress value={progress}>
              <ProgressLabel>
                {phase || m['content.installing']()}
              </ProgressLabel>
              <ProgressValue />
            </Progress>
          </div>
        ) : (
          <>
            <div className="max-h-[58vh] min-h-[16rem] overflow-x-hidden overflow-y-auto p-1">
              {stepId === 'target' ? (
                <TargetStep
                  kind={picked?.kind ?? 'mod'}
                  targets={targets.filter(
                    (t) => picked && targetTakesKind(t, picked.kind),
                  )}
                  selectedId={targetId}
                  onSelect={(t) => {
                    setTargetId(t.id);
                    setWorlds([]);
                  }}
                />
              ) : stepId === 'content' ? (
                target && (
                  <ContentStep
                    target={target}
                    kind={kindFilter}
                    onKindChange={(k) => {
                      setKindFilter(k);
                      setPicked(null);
                      setFile(null);
                      setVersionId('');
                    }}
                    selectedId={picked ? projectRef(picked) : ''}
                    file={file}
                    onSelect={(p) => {
                      setPicked(p);
                      setFile(null);
                      setVersionId('');
                      setWorlds([]);
                    }}
                    onPickFile={(path) => {
                      setFile(path);
                      setPicked(null);
                      setVersionId('');
                    }}
                  />
                )
              ) : stepId === 'worlds' ? (
                <WorldsStep
                  instanceId={target?.id ?? ''}
                  selected={worlds}
                  onToggle={(w, on) =>
                    setWorlds((prev) =>
                      on ? [...prev, w] : prev.filter((x) => x !== w),
                    )
                  }
                />
              ) : (
                <ReviewStep
                  target={target}
                  project={picked}
                  file={file}
                  kind={kind}
                  versionId={versionId}
                  onVersion={setVersionId}
                  worlds={needsWorlds ? worlds : undefined}
                />
              )}
              {error && (
                <p className="mt-3 px-1 text-xs text-destructive">{error}</p>
              )}
            </div>

            <DialogFooter className="items-center">
              <StepDots
                steps={steps}
                active={Math.min(step, steps.length - 1)}
                className="mr-auto"
              />
              {step === 0 ? (
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                >
                  {m['action.cancel']()}
                </Button>
              ) : (
                <Button
                  type="button"
                  variant="outline"
                  data-icon="inline-start"
                  onClick={() => setStep((s) => Math.max(0, s - 1))}
                >
                  <CaretLeftIcon />
                  {m['action.back']()}
                </Button>
              )}
              {stepId === 'review' ? (
                <Button
                  type="button"
                  disabled={!canAdvance}
                  className="bg-ember text-ember-foreground hover:bg-ember/90"
                  onClick={install}
                >
                  {m['action.install']()}
                </Button>
              ) : (
                <Button
                  type="button"
                  data-icon="inline-end"
                  disabled={!canAdvance}
                  className="bg-ember text-ember-foreground hover:bg-ember/90"
                  onClick={() => setStep((s) => s + 1)}
                >
                  {m['action.next']()}
                  <CaretRightIcon />
                </Button>
              )}
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}

function FilterBar({
  search,
  onSearch,
  placeholder,
  chips,
}: {
  search: string;
  onSearch: (v: string) => void;
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
  selectedId,
  onSelect,
}: {
  kind: ContentKind;
  targets: Target[];
  selectedId: string;
  onSelect: (t: Target) => void;
}) {
  const [search, setSearch] = useState('');
  const q = search.trim().toLowerCase();
  const shown = targets.filter((t) => !q || t.name.toLowerCase().includes(q));

  if (targets.length === 0) {
    return (
      <p className="px-1 py-8 text-center text-xs text-muted-foreground">
        {m['content.no_target_for_kind']({
          kind: contentKindLabel[kind]().toLowerCase(),
        })}
      </p>
    );
  }
  return (
    <div>
      <FilterBar
        search={search}
        onSearch={setSearch}
        placeholder={m['search.targets']()}
      />
      {shown.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['browse.nothing_matches']()}
        </p>
      ) : (
        <div className="grid gap-2">
          {shown.map((t) => (
            <PickRow
              key={t.id}
              icon={entryIcon(t.type)}
              title={t.name}
              subtitle={
                t.type === 'profile'
                  ? entryTypeLabel(t.type)
                  : `${entryTypeLabel(t.type)} · ${t.flavor} · ${t.gameVersion}`
              }
              badge={t.running ? m['content.stop_to_install']() : undefined}
              disabled={t.running}
              selected={selectedId === t.id}
              onSelect={() => onSelect(t)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function ContentStep({
  target,
  kind,
  onKindChange,
  selectedId,
  file,
  onSelect,
  onPickFile,
}: {
  target: Target;
  kind: ContentKind | null;
  onKindChange: (kind: ContentKind | null) => void;
  selectedId: string;
  file: string | null;
  onSelect: (p: ContentProject) => void;
  onPickFile: (path: string) => void;
}) {
  const [search, setSearch] = useState('');
  const kinds = ACCEPTS[target.type].filter((k) => targetTakesKind(target, k));
  const activeKind = kind ?? kinds[0];

  const results = useQuery(
    contentQueries.search({
      kind: activeKind,
      query: search.trim(),
      loader: activeKind === 'mod' ? target.flavor : undefined,
      gameVersion: target.gameVersion || undefined,
      limit: 30,
    }),
  );
  const hits = results.data?.hits ?? [];

  return (
    <div>
      <FilterBar
        search={search}
        onSearch={setSearch}
        placeholder={m['search.modrinth']()}
        chips={kinds.map((k) => ({
          label: kindInfo[k].label(),
          active: activeKind === k,
          onClick: () => onKindChange(k),
        }))}
      />

      {/* A global profile stores project references, never files. */}
      {target.type !== 'profile' && (
        <FileImportButton file={file} onPickFile={onPickFile} />
      )}

      {results.isPending ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['content.installing']()}
        </p>
      ) : hits.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['browse.nothing_matches']()}
        </p>
      ) : (
        <div className="grid gap-2">
          {hits.map((p) => (
            <PickRow
              key={`${p.source}:${p.id}`}
              icon={contentIcon(p.kind)}
              title={p.title}
              subtitle={`${contentKindLabel[p.kind]()} · ${m['browse.by_author']({ name: p.author })}`}
              selected={selectedId === projectRef(p)}
              onSelect={() => onSelect(p)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function FileImportButton({
  file,
  onPickFile,
}: {
  file: string | null;
  onPickFile: (path: string) => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={file !== null}
      onClick={async () => {
        const path = await dialog.pickContentFile();
        if (path) onPickFile(path);
      }}
      className={cn(
        'mb-2 flex w-full items-center gap-3 border border-dashed p-3 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-ring',
        file ? 'border-ember bg-muted' : 'border-border hover:bg-muted/60',
      )}
    >
      <UploadSimpleIcon className="size-4 shrink-0 text-muted-foreground" />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm">
          {file ? fileName(file) : m['content.import_file']()}
        </span>
        <span className="block truncate text-[11px] text-muted-foreground">
          {m['content.import_file_hint']()}
        </span>
      </span>
    </button>
  );
}

function WorldsStep({
  instanceId,
  selected,
  onToggle,
}: {
  instanceId: string;
  selected: string[];
  onToggle: (world: string, on: boolean) => void;
}) {
  const query = useQuery({
    queryKey: ['instance-worlds', instanceId],
    queryFn: () => instanceApi.worlds(instanceId),
    enabled: !!instanceId,
  });
  const list = query.data ?? [];

  if (!query.isPending && list.length === 0) {
    return (
      <p className="px-1 py-6 text-center text-xs text-muted-foreground">
        {m['content.no_worlds_in_instance']()}
      </p>
    );
  }
  return (
    <div className="flex flex-col gap-1.5">
      {list.map((w) => {
        const checked = selected.includes(w);
        const id = `world-${w}`;
        return (
          <label
            key={w}
            htmlFor={id}
            className={cn(
              'flex cursor-pointer items-center gap-2.5 px-3 py-2.5 text-sm ring-1 transition-colors',
              checked ? 'bg-muted ring-ember' : 'ring-border hover:bg-muted/60',
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

function ReviewStep({
  target,
  project,
  file,
  kind,
  versionId,
  onVersion,
  worlds,
}: {
  target: Target | null;
  project: ContentProject | null;
  file: string | null;
  kind: ContentKind | null;
  versionId: string;
  onVersion: (id: string) => void;
  worlds?: string[];
}) {
  const isProfile = target?.type === 'profile';
  const versions = useQuery({
    ...contentQueries.versions({
      source: project?.source ?? '',
      project: project ? projectRef(project) : '',
      loader:
        !isProfile && kind === 'mod'
          ? (target?.flavor ?? undefined)
          : undefined,
      gameVersion: !isProfile ? target?.gameVersion || undefined : undefined,
    }),
    enabled: !!project && !file,
  });
  const list = versions.data ?? [];
  const resolved = list.find((v) => v.id === versionId) ?? list[0];
  const requiredDeps =
    resolved?.dependencies.filter((d) => d.kind === 'required').length ?? 0;

  return (
    <div className="flex flex-col gap-4 p-1">
      <div className="divide-y divide-border border border-border">
        <ReviewRow
          label={m['label.content']()}
          value={project?.title ?? (file ? fileName(file) : '—')}
        />
        <ReviewRow label={m['label.target']()} value={target?.name ?? '—'} />
        {worlds && (
          <ReviewRow
            label={m['label.worlds']()}
            value={
              worlds.length ? worlds.join(', ') : m['content.none_selected']()
            }
          />
        )}
        {file ? (
          <ReviewRow
            label={m['label.version']()}
            value={m['content.local_file']()}
          />
        ) : isProfile ? null : (
          <div className="flex items-center justify-between gap-4 px-3 py-2 text-sm">
            <span className="text-xs text-muted-foreground">
              {m['label.version']()}
            </span>
            <div className="flex items-center gap-2">
              {resolved && !versionId && (
                <Badge variant="secondary" className="shrink-0">
                  {m['label.latest']()}
                </Badge>
              )}
              {resolved && (
                <VersionCombobox
                  versions={list}
                  value={resolved}
                  onChange={(v) =>
                    onVersion(v && v.id !== list[0]?.id ? v.id : '')
                  }
                />
              )}
            </div>
          </div>
        )}
      </div>

      {project && !file && requiredDeps > 0 && (
        <p className="text-[11px] text-muted-foreground">
          {m['content.dependencies']({ count: requiredDeps })}
        </p>
      )}
    </div>
  );
}

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
          onChange(v);
          requestAnimationFrame(() =>
            rootRef.current?.querySelector('input')?.blur(),
          );
        }}
        itemToStringLabel={(v: ContentVersion) => v.versionNumber}
        itemToStringValue={(v: ContentVersion) => v.versionNumber}
      >
        <ComboboxInput
          placeholder={m['content.select_version']()}
          className="w-48"
        />
        <ComboboxContent>
          <ComboboxEmpty>{m['content.no_versions']()}</ComboboxEmpty>
          <ComboboxList>
            {(v: ContentVersion) => (
              <ComboboxItem key={v.id} value={v}>
                <div className="flex min-w-0 flex-col">
                  <span className="flex items-center gap-1.5">
                    {v.versionNumber}
                    {v.id === latestId && (
                      <Badge variant="secondary" className="text-[10px]">
                        {m['label.latest']()}
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
                    {v.gameVersions.join(', ')} ·{' '}
                    {agoLabel(Date.parse(v.datePublished) / 1000)}
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
