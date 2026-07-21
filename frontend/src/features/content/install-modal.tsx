import {
  CaretLeftIcon,
  CaretRightIcon,
  UploadSimpleIcon,
  XIcon,
} from '@phosphor-icons/react';
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
import { PickerPanel } from '@/components/picker-panel';
import { SearchInput } from '@/components/search-input';
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
import { invalidate, keys } from '@/queries';
import { useContentSearch, useContentVersions } from '@/queries/content';
import {
  useInstanceContent,
  useInstances,
  useInstanceWorlds,
} from '@/queries/instance';
import { useGlobalProfiles } from '@/queries/profile';
import { useServerContent, useServers } from '@/queries/server';

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

/** A local file staged for import, tagged with the kind it installs as. */
interface PickedFile {
  path: string;
  kind: ContentKind;
}

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
  const [picked, setPicked] = useState<ContentProject[]>(
    project ? [project] : [],
  );
  const [files, setFiles] = useState<PickedFile[]>([]);
  const [kindFilter, setKindFilter] = useState<ContentKind | null>(null);
  const [versionIds, setVersionIds] = useState<Record<string, string>>({});
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
    setPicked(project ? [project] : []);
    setFiles([]);
    setKindFilter(null);
    setVersionIds({});
    setWorlds([]);
    setInstalling(false);
    setProgress(0);
    setPhase('');
    setError('');
  }, [open]);

  const target = targets.find((t) => t.id === targetId) ?? entry ?? null;
  const selectedCount = picked.length + files.length;
  const selectedKinds = [
    ...new Set([...picked.map((p) => p.kind), ...files.map((f) => f.kind)]),
  ];
  const needsWorlds =
    selectedKinds.includes('data_pack') && target?.type === 'instance';
  const isProfile = target?.type === 'profile';

  const toggleProject = (p: ContentProject) => {
    const ref = projectRef(p);
    setPicked((prev) =>
      prev.some((x) => projectRef(x) === ref)
        ? prev.filter((x) => projectRef(x) !== ref)
        : [...prev, p],
    );
    setVersionIds(({ [ref]: _, ...rest }) => rest);
  };
  const removeFile = (path: string) =>
    setFiles((prev) => prev.filter((f) => f.path !== path));

  const pickStep = mode === 'browse' ? 'target' : 'content';
  const steps: string[] =
    needsWorlds && !isProfile
      ? [pickStep, 'worlds', 'review']
      : [pickStep, 'review'];
  const stepId = steps[Math.min(step, steps.length - 1)];

  const Icon =
    mode === 'browse' && project
      ? contentIcon(project.kind)
      : entryIcon(entry?.type ?? 'instance');
  const title =
    mode === 'browse'
      ? m['content.install_title']({ name: project?.title ?? '' })
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
        ? selectedCount > 0
        : stepId === 'worlds'
          ? worlds.length > 0
          : selectedCount > 0;

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
      if (isProfile && picked.length > 0) {
        await profileApi.edit(target.name, { add: picked.map(projectRef) });
      } else if (target.type !== 'profile') {
        const add =
          target.type === 'server'
            ? serverApi.content.add
            : instanceApi.content.add;
        // The wire spec is per-kind, so a mixed selection installs as one
        // batch per kind; failures aggregate across batches.
        const failures: string[] = [];
        for (const k of selectedKinds) {
          const items = [
            ...picked
              .filter((p) => p.kind === k)
              .map((p) => ({
                project: projectRef(p),
                version: versionIds[projectRef(p)] ?? '',
              })),
            ...files.filter((f) => f.kind === k).map((f) => ({ path: f.path })),
          ];
          const spec = {
            kind: k,
            items,
            worlds: k === 'data_pack' && needsWorlds ? worlds : [],
          };
          const done = await add(target.id, spec, onProgress);
          // A batch "succeeds" even when every item failed to resolve/install;
          // surface those per-item failures instead of closing as if installed.
          failures.push(...done.failures.map((f) => f.message));
        }
        if (failures.length > 0) {
          setError(failures.join('; '));
          setInstalling(false);
          return;
        }
      }
      if (isProfile) {
        invalidate(keys.profiles.all);
      } else if (target.type === 'instance') {
        invalidate(keys.instances.content(target.id));
        invalidate(keys.instances.info(target.id));
      } else if (target.type === 'server') {
        invalidate(keys.servers.content(target.id));
        invalidate(keys.servers.info(target.id));
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
          <div className="flex min-h-72 flex-col justify-center px-1">
            <Progress value={progress}>
              <ProgressLabel>
                {phase || m['content.installing']()}
              </ProgressLabel>
              <ProgressValue />
            </Progress>
          </div>
        ) : (
          <>
            <div className="flex max-h-[58vh] min-h-64 flex-col overflow-hidden p-1">
              {stepId === 'target' ? (
                <TargetStep
                  kind={project?.kind ?? 'mod'}
                  targets={targets.filter(
                    (t) => project && targetTakesKind(t, project.kind),
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
                    onKindChange={setKindFilter}
                    picked={picked}
                    onToggle={toggleProject}
                    onAddFiles={(paths, k) =>
                      setFiles((prev) => [
                        ...prev,
                        ...paths
                          .filter((p) => !prev.some((f) => f.path === p))
                          .map((path) => ({ path, kind: k })),
                      ])
                    }
                  />
                )
              ) : stepId === 'worlds' ? (
                <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto">
                  <WorldsStep
                    instanceId={target?.id ?? ''}
                    selected={worlds}
                    onToggle={(w, on) =>
                      setWorlds((prev) =>
                        on ? [...prev, w] : prev.filter((x) => x !== w),
                      )
                    }
                  />
                </div>
              ) : (
                <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto">
                  <ReviewStep
                    target={target}
                    picked={picked}
                    files={files}
                    versionIds={versionIds}
                    onVersion={(ref, id) =>
                      setVersionIds(({ [ref]: _, ...rest }) =>
                        id ? { ...rest, [ref]: id } : rest,
                      )
                    }
                    onRemoveProject={toggleProject}
                    onRemoveFile={removeFile}
                    worlds={needsWorlds ? worlds : undefined}
                  />
                </div>
              )}
              {error && (
                <p className="mt-3 shrink-0 px-1 text-xs text-destructive">
                  {error}
                </p>
              )}
            </div>

            <DialogFooter className="items-center">
              <div className="mr-auto flex items-center gap-3">
                <StepDots
                  steps={steps}
                  active={Math.min(step, steps.length - 1)}
                />
                {mode === 'entry' && selectedCount > 0 && (
                  <span className="text-[11px] text-muted-foreground">
                    {m['content.selected_count']({ count: selectedCount })}
                  </span>
                )}
              </div>
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
  chips?: {
    label: string;
    active: boolean;
    disabled?: boolean;
    onClick: () => void;
  }[];
}) {
  return (
    <div className="mb-3 flex flex-col gap-2.5">
      <SearchInput
        value={search}
        onChange={onSearch}
        placeholder={placeholder}
      />
      {chips && chips.length > 1 && (
        <div className="flex flex-wrap gap-1.5">
          {chips.map((c) => (
            <button
              key={c.label}
              type="button"
              disabled={c.disabled}
              className={cn(
                chipClass(c.active),
                c.disabled && 'cursor-not-allowed opacity-40',
              )}
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
    <PickerPanel
      header={
        <FilterBar
          search={search}
          onSearch={setSearch}
          placeholder={m['search.targets']()}
        />
      }
    >
      {shown.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['browse.nothing_matches']()}
        </p>
      ) : (
        <div className="grid gap-2 p-0.5">
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
    </PickerPanel>
  );
}

/**
 * The installed pool of a target, keyed `source:projectId` — the same match the
 * CLI's browse session uses to flag an already-installed hit. Built on the
 * server/instance content-list factories; a profile holds references, not an
 * installable pool, so it reports nothing.
 */
function useInstalledRefs(target: Target, kind: ContentKind): Set<string> {
  const server = useServerContent(target.id, kind, {
    enabled: target.type === 'server',
  });
  const instance = useInstanceContent(target.id, kind, {
    enabled: target.type === 'instance',
  });
  const items = (target.type === 'server' ? server : instance).data?.items;
  return useMemo(
    () =>
      new Set(
        (items ?? [])
          .filter((i) => i.projectId)
          .map((i) => `${i.source}:${i.projectId}`),
      ),
    [items],
  );
}

function ContentStep({
  target,
  kind,
  onKindChange,
  picked,
  onToggle,
  onAddFiles,
}: {
  target: Target;
  kind: ContentKind | null;
  onKindChange: (kind: ContentKind | null) => void;
  picked: ContentProject[];
  onToggle: (p: ContentProject) => void;
  onAddFiles: (paths: string[], kind: ContentKind) => void;
}) {
  const [search, setSearch] = useState('');
  const kinds = ACCEPTS[target.type].filter((k) => targetTakesKind(target, k));
  // Datapacks land inside a world; an instance with none can take none.
  const worlds = useInstanceWorlds(target.id, {
    enabled: target.type === 'instance',
  });
  const noWorlds = target.type === 'instance' && worlds.data?.length === 0;
  const datapackBlocked = (k: ContentKind) => k === 'data_pack' && noWorlds;
  const activeKind = kind ?? kinds[0];
  const pickedRefs = new Set(picked.map(projectRef));
  const installedRefs = useInstalledRefs(target, activeKind);

  const results = useContentSearch({
    kind: activeKind,
    query: search.trim(),
    loader: activeKind === 'mod' ? target.flavor : undefined,
    gameVersion: target.gameVersion || undefined,
    limit: 30,
  });
  const hits = results.data?.hits ?? [];

  return (
    <PickerPanel
      header={
        <>
          <FilterBar
            search={search}
            onSearch={setSearch}
            placeholder={m['search.modrinth']()}
            chips={kinds.map((k) => ({
              label: kindInfo[k].label(),
              active: activeKind === k,
              disabled: datapackBlocked(k),
              onClick: () => onKindChange(k),
            }))}
          />

          {/* A global profile stores project references, never files. */}
          {target.type !== 'profile' && !datapackBlocked(activeKind) && (
            <FileImportButton
              onPickFiles={(paths) => onAddFiles(paths, activeKind)}
            />
          )}
        </>
      }
    >
      {datapackBlocked(activeKind) ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['content.no_worlds_datapack']()}
        </p>
      ) : results.isPending ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['content.installing']()}
        </p>
      ) : hits.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['browse.nothing_matches']()}
        </p>
      ) : (
        <div className="grid gap-2 p-0.5">
          {hits.map((p) => {
            const installed = installedRefs.has(`${p.source}:${p.id}`);
            return (
              <PickRow
                key={`${p.source}:${p.id}`}
                icon={contentIcon(p.kind)}
                imageUrl={p.iconUrl}
                title={p.title}
                subtitle={`${contentKindLabel[p.kind]()} · ${m['browse.by_author']({ name: p.author })}`}
                badge={installed ? m['content.installed']() : undefined}
                disabled={installed}
                selected={pickedRefs.has(projectRef(p))}
                onSelect={() => onToggle(p)}
              />
            );
          })}
        </div>
      )}
    </PickerPanel>
  );
}

function FileImportButton({
  onPickFiles,
}: {
  onPickFiles: (paths: string[]) => void;
}) {
  return (
    <button
      type="button"
      onClick={async () => {
        const paths = await dialog.pickContentFiles();
        if (paths.length > 0) onPickFiles(paths);
      }}
      className="mb-2 flex w-full items-center gap-3 border border-dashed border-border p-3 text-left outline-none transition-colors hover:bg-muted/60 focus-visible:ring-1 focus-visible:ring-ring"
    >
      <UploadSimpleIcon className="size-4 shrink-0 text-muted-foreground" />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm">
          {m['content.import_file']()}
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
  const query = useInstanceWorlds(instanceId);
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
      {list.map((w) => {
        const checked = selected.includes(w);
        const id = `world-${w}`;
        return (
          <label
            key={w}
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
              onCheckedChange={(c) => onToggle(w, c)}
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
  picked,
  files,
  versionIds,
  onVersion,
  onRemoveProject,
  onRemoveFile,
  worlds,
}: {
  target: Target | null;
  picked: ContentProject[];
  files: PickedFile[];
  versionIds: Record<string, string>;
  onVersion: (ref: string, id: string) => void;
  onRemoveProject: (p: ContentProject) => void;
  onRemoveFile: (path: string) => void;
  worlds?: string[];
}) {
  const isProfile = target?.type === 'profile';

  return (
    <div className="flex flex-col gap-4 p-1">
      <div className="divide-y divide-border border border-border">
        <ReviewRow label={m['label.target']()} value={target?.name ?? '—'} />
        {worlds && (
          <ReviewRow
            label={m['label.worlds']()}
            value={
              worlds.length ? worlds.join(', ') : m['content.none_selected']()
            }
          />
        )}
      </div>

      <div className="divide-y divide-border border border-border">
        {picked.map((p) => (
          <ReviewItemRow
            key={projectRef(p)}
            target={target}
            project={p}
            isProfile={isProfile}
            versionId={versionIds[projectRef(p)] ?? ''}
            onVersion={(id) => onVersion(projectRef(p), id)}
            onRemove={() => onRemoveProject(p)}
          />
        ))}
        {files.map((f) => (
          <div
            key={f.path}
            className="flex items-center justify-between gap-4 px-3 py-2 text-sm"
          >
            <div className="min-w-0">
              <span className="block truncate">{fileName(f.path)}</span>
              <span className="block truncate text-[11px] text-muted-foreground">
                {m['content.local_file']()}
              </span>
            </div>
            <RemoveButton onClick={() => onRemoveFile(f.path)} />
          </div>
        ))}
      </div>
    </div>
  );
}

function RemoveButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={m['action.remove']()}
      className="flex size-6 shrink-0 items-center justify-center border border-border text-muted-foreground outline-none transition-colors hover:border-destructive/40 hover:bg-destructive/10 hover:text-destructive focus-visible:ring-1 focus-visible:ring-ring"
    >
      <XIcon weight="bold" className="size-3.5" />
    </button>
  );
}

function ReviewItemRow({
  target,
  project,
  isProfile,
  versionId,
  onVersion,
  onRemove,
}: {
  target: Target | null;
  project: ContentProject;
  isProfile: boolean;
  versionId: string;
  onVersion: (id: string) => void;
  onRemove: () => void;
}) {
  const versions = useContentVersions(
    {
      source: project.source,
      project: projectRef(project),
      loader:
        !isProfile && project.kind === 'mod'
          ? (target?.flavor ?? undefined)
          : undefined,
      gameVersion: !isProfile ? target?.gameVersion || undefined : undefined,
    },
    { enabled: !isProfile },
  );
  const list = versions.data ?? [];
  const resolved = list.find((v) => v.id === versionId) ?? list[0];
  const requiredDeps =
    resolved?.dependencies.filter((d) => d.kind === 'required').length ?? 0;

  return (
    <div className="flex items-center justify-between gap-4 px-3 py-2 text-sm">
      <div className="min-w-0">
        <span className="block truncate">{project.title}</span>
        <span className="block truncate text-[11px] text-muted-foreground">
          {contentKindLabel[project.kind]()}
          {requiredDeps > 0 &&
            ` · ${m['content.dependencies']({ count: requiredDeps })}`}
        </span>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        {!isProfile && (
          <>
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
          </>
        )}
        <RemoveButton onClick={onRemove} />
      </div>
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
