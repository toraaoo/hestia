import {
  CaretLeftIcon,
  CaretRightIcon,
  MagnifyingGlassIcon,
} from '@phosphor-icons/react';
import { revalidateLogic } from '@tanstack/react-form';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';

import type {
  ConfigEntry,
  Flavor,
  GameVersion,
  ProvisionPhase,
  ProvisionProgress,
} from '@/api';
import { entryIcon } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
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
import {
  createWizardDefaults,
  createWizardSchema,
  detailsStepSchema,
  flavorStepSchema,
  versionStepSchema,
} from '@/features/entries/schema';
import { useAppForm } from '@/hooks/form';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { instanceQueries, useCreateInstance } from '@/queries/instance';
import { serverQueries, useCreateServer } from '@/queries/server';

type Kind = 'server' | 'instance';
type Step = 'flavor' | 'version' | 'details';

const STEPS: Step[] = ['flavor', 'version', 'details'];

const GAMEMODES: Array<{ value: string; label: () => string }> = [
  { value: 'survival', label: m['gamemode.survival'] },
  { value: 'creative', label: m['gamemode.creative'] },
  { value: 'adventure', label: m['gamemode.adventure'] },
  { value: 'spectator', label: m['gamemode.spectator'] },
];
const DIFFICULTIES: Array<{ value: string; label: () => string }> = [
  { value: 'peaceful', label: m['difficulty.peaceful'] },
  { value: 'easy', label: m['difficulty.easy'] },
  { value: 'normal', label: m['difficulty.normal'] },
  { value: 'hard', label: m['difficulty.hard'] },
];

const options = (items: Array<{ value: string; label: () => string }>) =>
  items.map((o) => ({ value: o.value, label: o.label() }));

const STEP_HINTS: Record<Step, (kind: Kind) => string> = {
  flavor: (kind) =>
    kind === 'server'
      ? m['wizard.hint_flavor_server']()
      : m['wizard.hint_flavor_instance'](),
  version: () => m['wizard.hint_version'](),
  details: (kind) =>
    kind === 'server'
      ? m['wizard.hint_details_server']()
      : m['wizard.hint_details_instance'](),
};

/** Map a live provisioning phase to a human label; falls back to the raw id. */
function phaseLabel(phase: ProvisionPhase): string {
  switch (phase) {
    case 'resolving':
      return m['phase.resolving_profile']();
    case 'java':
      return m['phase.installing_java']();
    case 'server':
      return m['phase.downloading_server']();
    case 'client':
    case 'libraries':
    case 'assets':
      return m['phase.downloading']({ name: phase });
    case 'content':
      return m['phase.mirroring']();
    default:
      return phase;
  }
}

function percentOf(progress: ProvisionProgress | null): number {
  if (!progress || progress.total <= 0) return 0;
  return Math.round((progress.current / progress.total) * 100);
}

/**
 * The New server / New instance wizard: flavor → version → details, wired to
 * the daemon's provider catalogue and the real create job. A server create
 * streams provisioning phases as a progress bar; an instance create is a quick
 * record write. On success the dialog closes and the list invalidates.
 */
export function CreateEntryModal({
  kind,
  open,
  onOpenChange,
}: {
  kind: Kind;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [step, setStep] = useState<Step>('flavor');
  const [search, setSearch] = useState('');
  const [showSnapshots, setShowSnapshots] = useState(false);

  const serverFlavors = useQuery({
    ...serverQueries.flavors(),
    enabled: kind === 'server',
  });
  const instanceFlavors = useQuery({
    ...instanceQueries.flavors(),
    enabled: kind === 'instance',
  });
  const flavorsQuery = kind === 'server' ? serverFlavors : instanceFlavors;
  const flavors: Flavor[] = useMemo(
    () => flavorsQuery.data ?? [],
    [flavorsQuery.data],
  );

  const createServer = useCreateServer();
  const createInstance = useCreateInstance();
  const creating = createServer.isPending || createInstance.isPending;
  const progress = createServer.progress;

  const form = useAppForm({
    defaultValues: createWizardDefaults(''),
    validationLogic: revalidateLogic(),
    validators: { onDynamic: createWizardSchema(kind) },
    onSubmit: async ({ value }) => {
      try {
        if (kind === 'server') {
          const server = await createServer.mutateAsync(serverParams(value));
          toast.success(m['toast.created']({ name: server.name }));
        } else {
          const instance = await createInstance.mutateAsync(
            instanceParams(value),
          );
          toast.success(m['toast.created']({ name: instance.name }));
        }
        onOpenChange(false);
      } catch (error) {
        toast.error(errorMessage(error));
      }
    },
  });

  useEffect(() => {
    if (!open) return;
    setStep('flavor');
    setSearch('');
    setShowSnapshots(false);
    form.reset();
  }, [open, form]);

  const Icon = entryIcon(kind);
  const stepIndex = STEPS.indexOf(step);

  const nav = (
    <DialogFooter className="items-center">
      <StepDots active={stepIndex} className="mr-auto" />
      {stepIndex === 0 ? (
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
          onClick={() => setStep(STEPS[stepIndex - 1])}
          data-icon="inline-start"
        >
          <CaretLeftIcon />
          {m['action.back']()}
        </Button>
      )}
      {step === 'details' ? (
        <Button
          type="submit"
          className="bg-ember text-ember-foreground hover:bg-ember/90"
        >
          {kind === 'server'
            ? m['wizard.create_server']()
            : m['wizard.create_instance']()}
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
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!creating) onOpenChange(o);
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Icon className="size-4.5 text-muted-foreground" />
            {kind === 'server' ? m['servers.new']() : m['instances.new']()}
          </DialogTitle>
          <DialogDescription>
            {creating
              ? kind === 'server'
                ? m['wizard.provisioning_server']()
                : m['wizard.provisioning_instance']()
              : STEP_HINTS[step](kind)}
          </DialogDescription>
        </DialogHeader>

        {creating ? (
          <div className="min-h-[18rem] p-1">
            <div className="flex min-h-[18rem] flex-col justify-center px-1">
              <Progress value={percentOf(progress)}>
                <ProgressLabel>
                  {progress
                    ? phaseLabel(progress.phase)
                    : m['phase.resolving_profile']()}
                </ProgressLabel>
                <ProgressValue />
              </Progress>
            </div>
          </div>
        ) : step === 'flavor' ? (
          <form.FormGroup
            name="flavor"
            validators={{ onDynamic: flavorStepSchema() }}
            onGroupSubmit={() => setStep('version')}
          >
            {(group) => (
              <StepForm onSubmit={group.handleSubmit} footer={nav}>
                <form.AppField name="flavor.flavor">
                  {(field) => (
                    <div className="grid gap-2">
                      {flavors.map((f) => (
                        <FlavorOption
                          key={f.id}
                          name={f.name}
                          summary={flavorSummary(f.id)}
                          selected={field.state.value === f.id}
                          onSelect={() => {
                            field.handleChange(f.id);
                            form.setFieldValue('version.version', '');
                            form.setFieldValue('version.loaderVersion', '');
                          }}
                        />
                      ))}
                      {flavors.length === 0 && (
                        <p className="px-1 py-6 text-center text-xs text-muted-foreground">
                          {flavorsQuery.isPending
                            ? m['common.loading']()
                            : m['wizard.no_versions_match']()}
                        </p>
                      )}
                    </div>
                  )}
                </form.AppField>
              </StepForm>
            )}
          </form.FormGroup>
        ) : step === 'version' ? (
          <form.FormGroup
            name="version"
            validators={{ onDynamic: versionStepSchema() }}
            onGroupSubmit={() => setStep('details')}
          >
            {(group) => (
              <StepForm onSubmit={group.handleSubmit} footer={nav}>
                <form.Subscribe selector={(s) => s.values.flavor.flavor}>
                  {(flavor) => (
                    <VersionStep
                      form={form}
                      kind={kind}
                      flavor={flavor}
                      search={search}
                      onSearch={setSearch}
                      showSnapshots={showSnapshots}
                      onShowSnapshots={setShowSnapshots}
                    />
                  )}
                </form.Subscribe>
              </StepForm>
            )}
          </form.FormGroup>
        ) : (
          <form.FormGroup
            name="details"
            validators={{ onDynamic: detailsStepSchema(kind) }}
            onGroupSubmit={() => form.handleSubmit()}
          >
            {(group) => (
              <StepForm onSubmit={group.handleSubmit} footer={nav}>
                <DetailsStep form={form} kind={kind} />
              </StepForm>
            )}
          </form.FormGroup>
        )}
      </DialogContent>
    </Dialog>
  );
}

/** The static per-flavor blurb, by id; empty for an unknown flavor. */
function flavorSummary(id: string): string {
  if (id === 'vanilla') return m['flavor.vanilla_summary']();
  if (id === 'fabric') return m['flavor.fabric_summary']();
  return '';
}

// biome-ignore lint/suspicious/noExplicitAny: the wizard form's generic type is internal to TanStack Form.
type WizardForm = any;

/** The version list, snapshot toggle, and loader-build picker for a flavor. */
function VersionStep({
  form,
  kind,
  flavor,
  search,
  onSearch,
  showSnapshots,
  onShowSnapshots,
}: {
  form: WizardForm;
  kind: Kind;
  flavor: string;
  search: string;
  onSearch: (value: string) => void;
  showSnapshots: boolean;
  onShowSnapshots: (value: boolean) => void;
}) {
  const selected = form.state.values.version.version as string;

  const serverVersions = useQuery({
    ...serverQueries.versions(flavor),
    enabled: kind === 'server',
  });
  const instanceVersions = useQuery({
    ...instanceQueries.versions(flavor),
    enabled: kind === 'instance',
  });
  const versionsQuery = kind === 'server' ? serverVersions : instanceVersions;
  const versions: GameVersion[] = versionsQuery.data ?? [];

  const serverLoaders = useQuery({
    ...serverQueries.loaders(flavor, selected),
    enabled: kind === 'server' && flavor !== '' && selected !== '',
  });
  const instanceLoaders = useQuery({
    ...instanceQueries.loaders(flavor, selected),
    enabled: kind === 'instance' && flavor !== '' && selected !== '',
  });
  const loadersQuery = kind === 'server' ? serverLoaders : instanceLoaders;
  const loaders = loadersQuery.data;

  const q = search.trim().toLowerCase();
  const list = versions.filter((v) => {
    const isRelease = v.kind === 'release';
    if (!showSnapshots && !isRelease) return false;
    if (q && !v.id.toLowerCase().includes(q)) return false;
    return true;
  });

  useEffect(() => {
    const current = form.state.values.version.loaderVersion as string;
    if (loaders && loaders.length > 0) {
      if (!current || !loaders.includes(current)) {
        form.setFieldValue('version.loaderVersion', loaders[0]);
      }
    } else if (current) {
      form.setFieldValue('version.loaderVersion', '');
    }
  }, [loaders, form]);

  return (
    <div className="flex flex-col gap-3">
      <div className="relative">
        <MagnifyingGlassIcon className="-translate-y-1/2 absolute top-1/2 left-2.5 size-3.5 text-muted-foreground" />
        <Input
          className="pl-8"
          placeholder={m['wizard.filter_versions']()}
          value={search}
          onChange={(e) => onSearch(e.target.value)}
        />
      </div>

      <label
        htmlFor="wizard-snapshots"
        className="flex w-fit cursor-pointer items-center gap-2 text-xs text-muted-foreground"
      >
        <Checkbox
          id="wizard-snapshots"
          checked={showSnapshots}
          onCheckedChange={(c) => onShowSnapshots(c === true)}
        />
        {m['wizard.show_snapshots']()}
      </label>

      <form.AppField name="version.version">
        {(field: WizardForm) => {
          const invalid =
            field.state.meta.isTouched && field.state.meta.errors.length > 0;
          return (
            <div className="flex flex-col gap-1.5">
              <div className="max-h-52 divide-y divide-border overflow-y-auto border border-border">
                {list.length === 0 ? (
                  <p className="px-3 py-6 text-center text-xs text-muted-foreground">
                    {versionsQuery.isPending
                      ? m['common.loading']()
                      : m['wizard.no_versions_match']()}
                  </p>
                ) : (
                  list.map((v) => (
                    <VersionRow
                      key={v.id}
                      id={v.id}
                      snapshot={v.kind !== 'release'}
                      selected={field.state.value === v.id}
                      onSelect={() => field.handleChange(v.id)}
                    />
                  ))
                )}
              </div>
              {invalid && (
                <FieldError
                  errors={
                    field.state.meta.errors as Array<{ message?: string }>
                  }
                />
              )}
            </div>
          );
        }}
      </form.AppField>

      {loaders && loaders.length > 0 && (
        <form.AppField name="version.loaderVersion">
          {(field: WizardForm) => (
            <div className="flex items-center gap-2">
              <span className="text-xs text-muted-foreground">
                {m['label.loader']()}
              </span>
              <field.SelectField
                options={loaders.map((l: string) => ({ value: l, label: l }))}
                triggerClassName="w-40"
              />
            </div>
          )}
        </form.AppField>
      )}
    </div>
  );
}

/** The details step: name + memory, plus the server.properties block. */
function DetailsStep({ form, kind }: { form: WizardForm; kind: Kind }) {
  return (
    <div className="flex flex-col gap-4">
      <form.Subscribe
        selector={(s: WizardForm) =>
          [s.values.flavor.flavor, s.values.version.version] as const
        }
      >
        {([flavor, version]: [string, string]) => (
          <form.AppField name="details.name">
            {(field: WizardForm) => (
              <field.TextField
                label={m['label.name']()}
                placeholder={`${flavor}-${version}`}
                description={m['wizard.name_hint']()}
              />
            )}
          </form.AppField>
        )}
      </form.Subscribe>

      <form.AppField name="details.memory">
        {(field: WizardForm) => (
          <field.SliderField
            label={m['label.memory']()}
            formatValue={(v: number) => m['wizard.gb']({ value: v })}
            min={2}
            max={32}
            step={1}
          />
        )}
      </form.AppField>

      {kind === 'server' && (
        <>
          <SectionHeader>{m['wizard.server_properties']()}</SectionHeader>

          <form.AppField name="details.motd">
            {(field: WizardForm) => (
              <field.TextField label={m['wizard.motd']()} />
            )}
          </form.AppField>

          <div className="grid gap-4 sm:grid-cols-2">
            <form.AppField name="details.gamemode">
              {(field: WizardForm) => (
                <field.SelectField
                  label={m['wizard.gamemode']()}
                  options={options(GAMEMODES)}
                  triggerClassName="w-full"
                />
              )}
            </form.AppField>
            <form.AppField name="details.difficulty">
              {(field: WizardForm) => (
                <field.SelectField
                  label={m['wizard.difficulty']()}
                  options={options(DIFFICULTIES)}
                  triggerClassName="w-full"
                />
              )}
            </form.AppField>
          </div>

          <div className="grid gap-4 sm:grid-cols-2">
            <form.AppField name="details.maxPlayers">
              {(field: WizardForm) => (
                <field.TextField
                  label={m['wizard.max_players']()}
                  type="number"
                />
              )}
            </form.AppField>
            <form.AppField name="details.port">
              {(field: WizardForm) => (
                <field.TextField
                  label={m['wizard.port']()}
                  type="number"
                  placeholder={m['wizard.port_auto']()}
                  description={m['wizard.port_hint']()}
                />
              )}
            </form.AppField>
          </div>

          <div className="grid grid-cols-2 gap-4 pt-1">
            <form.AppField name="details.hardcore">
              {(field: WizardForm) => (
                <PropToggle
                  id="prop-hardcore"
                  label={m['wizard.hardcore']()}
                  checked={field.state.value}
                  onChange={field.handleChange}
                />
              )}
            </form.AppField>
            <form.AppField name="details.onlineMode">
              {(field: WizardForm) => (
                <PropToggle
                  id="prop-online"
                  label={m['wizard.online_mode']()}
                  checked={field.state.value}
                  onChange={field.handleChange}
                />
              )}
            </form.AppField>
          </div>

          <form.AppField name="details.eula">
            {(field: WizardForm) => {
              const invalid =
                field.state.meta.isTouched &&
                field.state.meta.errors.length > 0;
              return (
                <div className="flex flex-col gap-1.5">
                  <label
                    htmlFor={field.name}
                    className="flex cursor-pointer items-center gap-2.5 border border-border px-3 py-2.5"
                  >
                    <Checkbox
                      id={field.name}
                      checked={field.state.value}
                      onCheckedChange={(c) => field.handleChange(c === true)}
                    />
                    <span className="text-xs text-muted-foreground">
                      {m['wizard.eula_before']()}{' '}
                      <a
                        href="https://aka.ms/MinecraftEULA"
                        target="_blank"
                        rel="noreferrer"
                        className="text-foreground underline underline-offset-2"
                      >
                        {m['wizard.eula_link']()}
                      </a>
                      {m['wizard.eula_after']()}
                    </span>
                  </label>
                  {invalid && (
                    <FieldError
                      errors={
                        field.state.meta.errors as Array<{ message?: string }>
                      }
                    />
                  )}
                </div>
              );
            }}
          </form.AppField>
        </>
      )}
    </div>
  );
}

/** Build the server create params from the wizard's collected values. */
// biome-ignore lint/suspicious/noExplicitAny: the wizard value is the form's internal shape.
function serverParams(value: any) {
  const d = value.details;
  const config: ConfigEntry[] = [
    { key: 'memory', value: `${d.memory}G` },
    { key: 'motd', value: d.motd },
    { key: 'gamemode', value: d.gamemode },
    { key: 'difficulty', value: d.difficulty },
    { key: 'max-players', value: d.maxPlayers },
    { key: 'hardcore', value: String(d.hardcore) },
    { key: 'online-mode', value: String(d.onlineMode) },
  ];
  return {
    name: d.name || undefined,
    flavor: value.flavor.flavor,
    version: value.version.version,
    loader_version: value.version.loaderVersion || undefined,
    eula: true,
    port: d.port ? Number(d.port) : undefined,
    config,
  };
}

// biome-ignore lint/suspicious/noExplicitAny: the wizard value is the form's internal shape.
function instanceParams(value: any) {
  const d = value.details;
  return {
    name: d.name || undefined,
    flavor: value.flavor.flavor,
    version: value.version.version,
    loader_version: value.version.loaderVersion || undefined,
    config: [{ key: 'memory', value: `${d.memory}G` }],
  };
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function StepForm({
  onSubmit,
  footer,
  children,
}: {
  onSubmit: () => void;
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
      <div className="min-h-[18rem] max-h-[58vh] overflow-x-hidden overflow-y-auto p-1">
        {children}
      </div>
      {footer}
    </form>
  );
}

function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-2.5 pt-1">
      <span className="text-[10px] font-semibold tracking-wide text-muted-foreground uppercase">
        {children}
      </span>
      <div className="h-px flex-1 bg-border" />
    </div>
  );
}

function PropToggle({
  id,
  label,
  checked,
  onChange,
}: {
  id: string;
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label
      htmlFor={id}
      className="flex cursor-pointer items-center gap-2.5 border border-border px-3 py-2 text-xs font-medium leading-none transition-colors hover:bg-muted/40"
    >
      <Checkbox
        id={id}
        checked={checked}
        onCheckedChange={(c) => onChange(c === true)}
      />
      {label}
    </label>
  );
}

function FlavorOption({
  name,
  summary,
  selected,
  onSelect,
}: {
  name: string;
  summary: string;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
      className={cn(
        'flex flex-col items-start gap-0.5 p-3 text-left ring-1 transition-colors outline-none focus-visible:ring-ring',
        selected
          ? 'bg-muted ring-ember'
          : 'ring-border hover:bg-muted/60 hover:ring-foreground/20',
      )}
    >
      <span className="text-sm font-medium">{name}</span>
      {summary && (
        <span className="text-xs text-muted-foreground">{summary}</span>
      )}
    </button>
  );
}

function VersionRow({
  id,
  snapshot,
  selected,
  onSelect,
}: {
  id: string;
  snapshot: boolean;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
      className={cn(
        'flex w-full items-center gap-2 px-3 py-2 text-left outline-none transition-colors focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset',
        selected ? 'bg-muted text-foreground' : 'hover:bg-muted/50',
      )}
    >
      <span
        className={cn(
          'size-1.5 rounded-full',
          selected ? 'bg-ember' : 'bg-transparent',
        )}
      />
      <span className="flex-1 font-mono text-xs">{id}</span>
      {snapshot && (
        <Badge variant="outline" className="text-[10px]">
          {m['wizard.snapshot']()}
        </Badge>
      )}
    </button>
  );
}

function StepDots({
  active,
  className,
}: {
  active: number;
  className?: string;
}) {
  return (
    <div className={cn('flex items-center gap-1.5', className)}>
      {STEPS.map((s, i) => (
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
