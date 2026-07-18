import {
  CaretLeftIcon,
  CaretRightIcon,
  type CheckIcon,
  MagnifyingGlassIcon,
  UploadSimpleIcon,
} from '@phosphor-icons/react';
import { revalidateLogic } from '@tanstack/react-form';
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
import { FieldError } from '@/components/ui/field';
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
import { PickRow } from '@/features/content/pick-row';
import {
  installWizardDefaults,
  installWizardSchema,
  pickStepSchema,
  reviewStepSchema,
  worldsStepSchema,
} from '@/features/content/schema';
import {
  type Instance,
  instances,
  type Server,
  servers,
} from '@/features/entries/mock';
import { type GlobalProfile, globalProfiles } from '@/features/profiles/mock';
import { useAppForm } from '@/hooks/form';
import { agoLabel } from '@/lib/format';
import type { ContentKind } from '@/lib/mock';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

/** An entry the content can be installed into, drawn from both stores. */
export interface Target {
  id: string;
  name: string;
  type: 'server' | 'instance' | 'profile';
  flavor: string;
  gameVersion: string;
  running: boolean;
  worlds: string[];
}

export const serverTarget = (s: Server): Target => ({
  id: s.id,
  name: s.name,
  type: 'server',
  flavor: s.flavor,
  gameVersion: s.gameVersion,
  running: s.running,
  worlds: [],
});

export const instanceTarget = (i: Instance): Target => ({
  id: i.id,
  name: i.name,
  type: 'instance',
  flavor: i.flavor,
  gameVersion: i.gameVersion,
  running: i.running,
  worlds: i.worlds,
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
  worlds: [],
});

/** Which kinds each entry type accepts — mirrors the daemon's install surface. */
const ACCEPTS: Record<Target['type'], ContentKind[]> = {
  profile: ['mod', 'resourcepack', 'shader'],
  server: ['mod', 'datapack'],
  instance: ['mod', 'resourcepack', 'shader', 'datapack'],
};

/** A mod needs a loader; a vanilla entry cannot take one. */
const targetTakesKind = (t: Target, kind: ContentKind): boolean =>
  ACCEPTS[t.type].includes(kind) &&
  (kind !== 'mod' || t.flavor === 'fabric' || t.type === 'profile');

const allTargets = (): Target[] => [
  ...servers.map(serverTarget),
  ...instances.map(instanceTarget),
  ...globalProfiles.map(profileTarget),
];

/** Every entry that can take this kind, across both stores. */
function targetsFor(kind: ContentKind): Target[] {
  return allTargets().filter((t) => targetTakesKind(t, kind));
}

/** The projects an entry can take — its accepted kinds, loader-aware for mods. */
function projectsFor(target: Target): ContentProject[] {
  return contentProjects.filter((p) => targetTakesKind(target, p.kind));
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * A local-file pick travels through the form's `pick.projectId` slot as a
 * `file:<name>` marker — the daemon's `ContentAddItem` likewise takes exactly
 * one of a project or a path.
 */
const FILE_MARKER = 'file:';
const pickedFileName = (projectId: string): string | null =>
  projectId.startsWith(FILE_MARKER)
    ? projectId.slice(FILE_MARKER.length)
    : null;

const entryTypeLabel = (type: Target['type']): string =>
  type === 'server'
    ? m['entry.type_server']()
    : type === 'profile'
      ? m['entry.type_profile']()
      : m['entry.type_instance']();

/**
 * The content install wizard, mirroring the daemon's `content.add`. Built on
 * TanStack Form's multi-step wizard pattern — one `useAppForm` holds the
 * selections (as ids) nested per step, each step is a `FormGroup` validating
 * its own zod schema on "Next". It opens either way round: from Browse a
 * `project` is fixed and the user picks a target entry; from an entry's page
 * the `entry` is fixed and the user picks a project. The version auto-resolves
 * to the newest compatible build (changeable in the review), datapacks on an
 * instance choose their worlds, and required dependencies are pulled in — then
 * the install job plays as progress. Nothing is persisted; mock data.
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

  const [installing, setInstalling] = useState(false);
  const [phase, setPhase] = useState('');
  const [progress, setProgress] = useState(0);
  const [stepIndex, setStepIndex] = useState(0);

  const form = useAppForm({
    defaultValues: installWizardDefaults({
      projectId: project?.id ?? '',
      targetId: entry?.id ?? '',
      versionId: versionId ?? '',
    }),
    validationLogic: revalidateLogic(),
    validators: { onDynamic: installWizardSchema(mode) },
    onSubmit: async ({ value }) => {
      // A local file installs as-is: no resolution, no dependencies.
      const fileName = pickedFileName(value.pick.projectId);
      if (fileName) {
        setInstalling(true);
        setPhase(m['phase.importing']({ name: fileName }));
        setProgress(30);
        await sleep(700);
        setPhase(m['phase.mirroring']());
        setProgress(85);
        await sleep(500);
        setPhase(m['phase.ready']());
        setProgress(100);
        await sleep(400);
        onOpenChange(false);
        setInstalling(false);
        return;
      }
      const proj = contentProjects.find((p) => p.id === value.pick.projectId);
      if (!proj) return;
      const files = [proj, ...resolveDependencies(proj.id)];
      setInstalling(true);
      setPhase(m['phase.resolving_dependencies']());
      setProgress(4);
      await sleep(600);
      for (let i = 0; i < files.length; i++) {
        setPhase(m['phase.downloading']({ name: files[i].title }));
        setProgress(Math.round(((i + 1) / (files.length + 1)) * 90) + 4);
        await sleep(650);
      }
      setPhase(m['phase.mirroring']());
      setProgress(97);
      await sleep(500);
      setPhase(m['phase.ready']());
      setProgress(100);
      await sleep(400);
      onOpenChange(false);
      setInstalling(false);
    },
  });

  // Reset everything each time the modal opens for a fresh source/entry.
  useEffect(() => {
    if (!open) return;
    setInstalling(false);
    setProgress(0);
    setStepIndex(0);
    form.reset();
  }, [open, form]);

  const Icon =
    mode === 'browse' && project
      ? contentIcon(project.kind)
      : entryIcon(entry?.type ?? 'instance');
  const title =
    mode === 'browse'
      ? m['content.install_title']({ name: project?.title ?? '' })
      : m['content.add_to_title']({ name: entry?.name ?? '' });

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!installing) onOpenChange(o);
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <form.Subscribe
          selector={(s) =>
            [s.values.pick.projectId, s.values.pick.targetId] as const
          }
        >
          {([projectId, targetId]) => {
            const proj =
              contentProjects.find((p) => p.id === projectId) ?? null;
            const fileName = pickedFileName(projectId);
            const target = allTargets().find((t) => t.id === targetId) ?? null;
            const needsWorlds =
              proj?.kind === 'datapack' && target?.type === 'instance';
            const steps: string[] = needsWorlds
              ? [pickStep, 'worlds', 'review']
              : [pickStep, 'review'];

            return (
              <WizardBody
                form={form}
                mode={mode}
                pickStep={pickStep}
                steps={steps}
                proj={proj}
                fileName={fileName}
                target={target}
                needsWorlds={needsWorlds}
                Icon={Icon}
                title={title}
                installing={installing}
                phase={phase}
                progress={progress}
                stepIndex={stepIndex}
                setStepIndex={setStepIndex}
                onCancel={() => onOpenChange(false)}
              />
            );
          }}
        </form.Subscribe>
      </DialogContent>
    </Dialog>
  );
}

/**
 * The step-driven body. Split out so the enclosing `form.Subscribe` recomputes
 * the derived project/target/steps on every selection change.
 */
function WizardBody({
  form,
  mode,
  pickStep,
  steps,
  proj,
  fileName,
  target,
  needsWorlds,
  Icon,
  title,
  installing,
  phase,
  progress,
  stepIndex,
  setStepIndex,
  onCancel,
}: {
  // biome-ignore lint/suspicious/noExplicitAny: the app form type is opaque here.
  form: any;
  mode: 'browse' | 'entry';
  pickStep: string;
  steps: string[];
  proj: ContentProject | null;
  fileName: string | null;
  target: Target | null;
  needsWorlds: boolean;
  Icon: typeof CheckIcon;
  title: string;
  installing: boolean;
  phase: string;
  progress: number;
  stepIndex: number;
  setStepIndex: (value: number) => void;
  onCancel: () => void;
}) {
  // A selection change can drop the worlds step from under us; clamp the index.
  const index = Math.min(stepIndex, steps.length - 1);
  const stepId = steps[index];

  // The pick step's search/filter lives up here so its bar can sit pinned in
  // the StepForm header, outside the scrolling list.
  const [pickSearch, setPickSearch] = useState('');
  const [pickFilter, setPickFilter] = useState('all');

  const versions = useMemo(() => (proj ? projectVersions(proj) : []), [proj]);
  const deps = useMemo(
    () => (proj ? resolveDependencies(proj.id) : []),
    [proj],
  );

  const back = () => setStepIndex(Math.max(0, index - 1));
  const next = () => setStepIndex(Math.min(steps.length - 1, index + 1));

  const hint = installing
    ? m['content.installing']()
    : stepId === 'target'
      ? m['content.hint_target']()
      : stepId === 'content'
        ? m['content.hint_content']()
        : stepId === 'worlds'
          ? m['content.hint_worlds']()
          : m['content.hint_review']();

  const nav = (
    <DialogFooter className="items-center">
      <StepDots steps={steps} active={index} className="mr-auto" />
      {index === 0 ? (
        <Button type="button" variant="outline" onClick={onCancel}>
          {m['action.cancel']()}
        </Button>
      ) : (
        <Button
          type="button"
          variant="outline"
          onClick={back}
          data-icon="inline-start"
        >
          <CaretLeftIcon />
          {m['action.back']()}
        </Button>
      )}
      {stepId === 'review' ? (
        <Button
          type="submit"
          className="bg-ember text-ember-foreground hover:bg-ember/90"
        >
          {m['action.install']()}
        </Button>
      ) : (
        <Button
          type="submit"
          data-icon="inline-end"
          className="bg-ember text-ember-foreground hover:bg-ember/90"
        >
          {m['action.next']()}
          <CaretRightIcon />
        </Button>
      )}
    </DialogFooter>
  );

  return (
    <>
      <DialogHeader>
        <DialogTitle className="flex items-center gap-2">
          <Icon className="size-4.5 text-muted-foreground" />
          {title}
        </DialogTitle>
        <DialogDescription>{hint}</DialogDescription>
      </DialogHeader>

      {installing ? (
        <div className="min-h-[18rem] p-1">
          <div className="flex min-h-[18rem] flex-col justify-center px-1">
            <Progress value={progress}>
              <ProgressLabel>{phase}</ProgressLabel>
              <ProgressValue />
            </Progress>
          </div>
        </div>
      ) : stepId === pickStep ? (
        <form.FormGroup
          name="pick"
          validators={{ onDynamic: pickStepSchema(mode) }}
          onGroupSubmit={next}
        >
          {(group: { handleSubmit: () => void }) => (
            <StepForm
              onSubmit={group.handleSubmit}
              header={
                mode === 'browse' ? (
                  <FilterBar
                    search={pickSearch}
                    onSearch={setPickSearch}
                    placeholder={m['search.targets']()}
                    chips={[
                      {
                        label: m['label.all'](),
                        active: pickFilter === 'all',
                        onClick: () => setPickFilter('all'),
                      },
                      {
                        label: m['nav.servers'](),
                        active: pickFilter === 'server',
                        onClick: () => setPickFilter('server'),
                      },
                      {
                        label: m['nav.instances'](),
                        active: pickFilter === 'instance',
                        onClick: () => setPickFilter('instance'),
                      },
                      {
                        label: m['profiles.nav'](),
                        active: pickFilter === 'profile',
                        onClick: () => setPickFilter('profile'),
                      },
                    ]}
                  />
                ) : (
                  target && (
                    <FilterBar
                      search={pickSearch}
                      onSearch={setPickSearch}
                      placeholder={m['search.modrinth']()}
                      chips={[
                        {
                          label: m['label.all'](),
                          active: pickFilter === 'all',
                          onClick: () => setPickFilter('all'),
                        },
                        ...ACCEPTS[target.type]
                          .filter((k) => targetTakesKind(target, k))
                          .map((k) => ({
                            label: kindInfo[k].label(),
                            active: pickFilter === k,
                            onClick: () => setPickFilter(k),
                          })),
                      ]}
                    />
                  )
                )
              }
              footer={nav}
            >
              {mode === 'browse' && proj ? (
                <form.AppField name="pick.targetId">
                  {(field: FieldApi<string>) => (
                    <TargetStep
                      kind={proj.kind}
                      targets={targetsFor(proj.kind)}
                      search={pickSearch}
                      type={pickFilter as 'all' | Target['type']}
                      selectedId={field.state.value}
                      errors={fieldErrors(field)}
                      onSelect={(t) => {
                        field.handleChange(t.id);
                        form.setFieldValue('worlds.worlds', []);
                      }}
                    />
                  )}
                </form.AppField>
              ) : (
                target && (
                  <form.AppField name="pick.projectId">
                    {(field: FieldApi<string>) => (
                      <ContentStep
                        projects={projectsFor(target)}
                        search={pickSearch}
                        kind={pickFilter as 'all' | ContentKind}
                        selectedId={field.state.value}
                        fileName={fileName}
                        // A global profile stores project references, never
                        // files, so it takes no local import.
                        allowFile={target.type !== 'profile'}
                        errors={fieldErrors(field)}
                        onSelect={(p) => {
                          field.handleChange(p.id);
                          form.setFieldValue('review.versionId', '');
                          form.setFieldValue('worlds.worlds', []);
                        }}
                        onPickFile={(name) => {
                          field.handleChange(`${FILE_MARKER}${name}`);
                          form.setFieldValue('review.versionId', '');
                          form.setFieldValue('worlds.worlds', []);
                        }}
                      />
                    )}
                  </form.AppField>
                )
              )}
            </StepForm>
          )}
        </form.FormGroup>
      ) : stepId === 'worlds' ? (
        <form.FormGroup
          name="worlds"
          validators={{ onDynamic: worldsStepSchema() }}
          onGroupSubmit={next}
        >
          {(group: { handleSubmit: () => void }) => (
            <StepForm onSubmit={group.handleSubmit} footer={nav}>
              <form.AppField name="worlds.worlds">
                {(field: FieldApi<string[]>) => {
                  const errors = fieldErrors(field);
                  const worlds = field.state.value;
                  return (
                    <div className="flex flex-col gap-1.5">
                      {target?.worlds.length ? (
                        target.worlds.map((w) => (
                          <WorldRow
                            key={w}
                            world={w}
                            checked={worlds.includes(w)}
                            onToggle={(on) =>
                              field.handleChange(
                                on
                                  ? [...worlds, w]
                                  : worlds.filter((x) => x !== w),
                              )
                            }
                          />
                        ))
                      ) : (
                        <p className="px-1 py-6 text-center text-xs text-muted-foreground">
                          {m['content.no_worlds_in_instance']()}
                        </p>
                      )}
                      {errors && <FieldError errors={errors} />}
                    </div>
                  );
                }}
              </form.AppField>
            </StepForm>
          )}
        </form.FormGroup>
      ) : (
        <form.FormGroup
          name="review"
          validators={{ onDynamic: reviewStepSchema }}
          onGroupSubmit={() => form.handleSubmit()}
        >
          {(group: { handleSubmit: () => void }) =>
            proj ? (
              <StepForm onSubmit={group.handleSubmit} footer={nav}>
                <form.AppField name="review.versionId">
                  {(field: FieldApi<string>) => {
                    const resolved =
                      versions.find((v) => v.id === field.state.value) ??
                      versions[0];
                    return (
                      resolved && (
                        <ReviewStep
                          project={proj}
                          versions={versions}
                          version={resolved}
                          explicit={
                            versions.find((v) => v.id === field.state.value) ??
                            null
                          }
                          onVersion={(v) => field.handleChange(v?.id ?? '')}
                          target={target}
                          deps={deps}
                          reviewWorlds={needsWorlds}
                          form={form}
                        />
                      )
                    );
                  }}
                </form.AppField>
              </StepForm>
            ) : fileName ? (
              <StepForm onSubmit={group.handleSubmit} footer={nav}>
                <FileReview name={fileName} target={target} />
              </StepForm>
            ) : (
              <div className="min-h-[18rem]" />
            )
          }
        </form.FormGroup>
      )}
    </>
  );
}

type FieldApi<T> = {
  name: string;
  state: { value: T; meta: { isTouched: boolean; errors: unknown[] } };
  handleChange: (value: T) => void;
};

/** A touched field's errors in the shape `FieldError` wants, or undefined. */
function fieldErrors<T>(field: FieldApi<T>) {
  const { isTouched, errors } = field.state.meta;
  return isTouched && errors.length > 0
    ? (errors as Array<{ message?: string }>)
    : undefined;
}

/**
 * A step's scrolling body wrapped in a `<form>` whose submit runs the group.
 * The `footer` sits below the scroll area but inside the form, so the step
 * nav stays pinned while only the body scrolls — and its submit button still
 * drives the group.
 */
function StepForm({
  onSubmit,
  header,
  footer,
  children,
}: {
  onSubmit: () => void;
  /** Pinned above the scroll area (the pick steps' search/filter bar). */
  header?: React.ReactNode;
  footer: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <form
      className="flex min-h-0 flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        e.stopPropagation();
        onSubmit();
      }}
    >
      {header && <div className="px-1 pt-1">{header}</div>}
      <div
        className={cn(
          'max-h-[58vh] overflow-x-hidden overflow-y-auto p-1',
          header ? 'min-h-[14rem]' : 'min-h-[18rem]',
        )}
      >
        {children}
      </div>
      {footer}
    </form>
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
    <div className="flex flex-col gap-2.5">
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
  search,
  type,
  selectedId,
  errors,
  onSelect,
}: {
  kind: ContentKind;
  targets: Target[];
  search: string;
  type: 'all' | Target['type'];
  selectedId: string;
  errors?: Array<{ message?: string }>;
  onSelect: (target: Target) => void;
}) {
  const q = search.trim().toLowerCase();
  const shown = targets.filter((t) => {
    if (type !== 'all' && t.type !== type) return false;
    return !q || t.name.toLowerCase().includes(q);
  });

  return (
    <div>
      {targets.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['content.no_target_for_kind']({
            kind: contentKindLabel[kind]().toLowerCase(),
          })}
        </p>
      ) : shown.length === 0 ? (
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
              subtitle={`${entryTypeLabel(t.type)} · ${t.flavor} · ${t.gameVersion}`}
              badge={t.running ? m['content.stop_to_install']() : undefined}
              disabled={t.running}
              selected={selectedId === t.id}
              onSelect={() => onSelect(t)}
            />
          ))}
        </div>
      )}
      {errors && <FieldError className="mt-2" errors={errors} />}
    </div>
  );
}

function ContentStep({
  projects,
  search,
  kind,
  selectedId,
  fileName,
  allowFile,
  errors,
  onSelect,
  onPickFile,
}: {
  projects: ContentProject[];
  search: string;
  kind: 'all' | ContentKind;
  selectedId: string;
  fileName: string | null;
  allowFile: boolean;
  errors?: Array<{ message?: string }>;
  onSelect: (project: ContentProject) => void;
  onPickFile: (name: string) => void;
}) {
  const fileRef = useRef<HTMLInputElement>(null);
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
      {allowFile && (
        <>
          <input
            ref={fileRef}
            type="file"
            accept=".jar,.zip,.mrpack"
            className="hidden"
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) onPickFile(file.name);
              e.target.value = '';
            }}
          />
          <button
            type="button"
            aria-pressed={fileName !== null}
            onClick={() => fileRef.current?.click()}
            className={cn(
              'mb-2 flex w-full items-center gap-3 border border-dashed p-3 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-ring',
              fileName
                ? 'border-ember bg-muted'
                : 'border-border hover:bg-muted/60',
            )}
          >
            <UploadSimpleIcon className="size-4 shrink-0 text-muted-foreground" />
            <span className="min-w-0 flex-1">
              <span className="block truncate text-sm">
                {fileName ?? m['content.import_file']()}
              </span>
              <span className="block truncate text-[11px] text-muted-foreground">
                {m['content.import_file_hint']()}
              </span>
            </span>
          </button>
        </>
      )}
      {shown.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['browse.nothing_matches']()}
        </p>
      ) : (
        <div className="grid gap-2">
          {shown.map((p) => (
            <PickRow
              key={p.id}
              icon={contentIcon(p.kind)}
              title={p.title}
              subtitle={`${contentKindLabel[p.kind]()} · ${m['browse.by_author']({ name: p.author })}`}
              selected={selectedId === p.id}
              onSelect={() => onSelect(p)}
            />
          ))}
        </div>
      )}
      {errors && <FieldError className="mt-2" errors={errors} />}
    </div>
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
  deps,
  reviewWorlds,
  form,
}: {
  project: ContentProject;
  versions: ContentVersion[];
  version: ContentVersion;
  explicit: ContentVersion | null;
  onVersion: (version: ContentVersion | null) => void;
  target: Target | null;
  deps: ContentProject[];
  reviewWorlds: boolean;
  // biome-ignore lint/suspicious/noExplicitAny: the app form type is opaque here.
  form: any;
}) {
  return (
    <div className="flex flex-col gap-4 p-1">
      <div className="divide-y divide-border border border-border">
        <ReviewRow label={m['label.content']()} value={project.title} />
        <ReviewRow label={m['label.target']()} value={target?.name ?? '—'} />
        {reviewWorlds && (
          <form.Subscribe
            selector={(s: WizardValues) => s.values.worlds.worlds}
          >
            {(worlds: string[]) => (
              <ReviewRow
                label={m['label.worlds']()}
                value={
                  worlds.length
                    ? worlds.join(', ')
                    : m['content.none_selected']()
                }
              />
            )}
          </form.Subscribe>
        )}
        <div className="flex items-center justify-between gap-4 px-3 py-2 text-sm">
          <span className="text-xs text-muted-foreground">
            {m['label.version']()}
          </span>
          <div className="flex items-center gap-2">
            {!explicit && (
              <Badge variant="secondary" className="shrink-0">
                {m['label.latest']()}
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
            {m['content.dependencies']({ count: deps.length })}
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
                  <span className="text-muted-foreground">
                    {m['label.required']()}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

/** The review body for a local-file import: no versions, no dependencies. */
function FileReview({ name, target }: { name: string; target: Target | null }) {
  return (
    <div className="flex flex-col gap-4 p-1">
      <div className="divide-y divide-border border border-border">
        <ReviewRow label={m['label.content']()} value={name} />
        <ReviewRow label={m['label.target']()} value={target?.name ?? '—'} />
        <ReviewRow
          label={m['label.version']()}
          value={m['content.local_file']()}
        />
      </div>
      <p className="text-xs text-muted-foreground">
        {m['content.import_file_hint']()}
      </p>
    </div>
  );
}

type WizardValues = { values: { worlds: { worlds: string[] } } };

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
                    {v.gameVersions.join(', ')} · {agoLabel(v.publishedUnix)}
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
