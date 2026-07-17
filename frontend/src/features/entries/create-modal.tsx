import {
  CaretLeftIcon,
  CaretRightIcon,
  MagnifyingGlassIcon,
} from '@phosphor-icons/react';
import { revalidateLogic } from '@tanstack/react-form';
import { useEffect, useState } from 'react';

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
  type CatalogVersion,
  flavors,
  loaderVersions,
  versionsFor,
} from '@/features/entries/catalog';
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

type Kind = 'server' | 'instance';
type Step = 'flavor' | 'version' | 'details';

const STEPS: Step[] = ['flavor', 'version', 'details'];

/** A curated slice of `server.properties` surfaced in the wizard. */
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

/** The create flow the daemon runs, faked as timed phases for the wizard. */
const PHASES: Record<Kind, Array<() => string>> = {
  server: [
    m['phase.resolving_profile'],
    m['phase.installing_java'],
    m['phase.downloading_server'],
    m['phase.generating_properties'],
    m['phase.ready'],
  ],
  instance: [m['phase.creating_record'], m['phase.ready']],
};

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

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * The New server / New instance wizard: flavor → version → details, mirroring
 * the CLI's interactive `create`. Built on TanStack Form's multi-step wizard
 * pattern — one `useAppForm` holds the whole value nested per step, each step
 * is a `FormGroup` that validates only its own zod schema on "Next", and the
 * final submit validates the composed schema. A server also gates on the EULA.
 * Submitting plays the daemon's provisioning phases as a progress bar — nothing
 * is persisted, the library is static mock data.
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
  const [creating, setCreating] = useState(false);
  const [phase, setPhase] = useState('');
  const [progress, setProgress] = useState(0);

  const form = useAppForm({
    defaultValues: createWizardDefaults(loaderVersions[0]),
    validationLogic: revalidateLogic(),
    validators: { onDynamic: createWizardSchema(kind) },
    onSubmit: async () => {
      setCreating(true);
      const phases = PHASES[kind];
      for (let i = 0; i < phases.length; i++) {
        setPhase(phases[i]());
        setProgress(Math.round(((i + 1) / phases.length) * 100));
        await sleep(650);
      }
      await sleep(300);
      onOpenChange(false);
      setCreating(false);
    },
  });

  useEffect(() => {
    if (!open) return;
    setStep('flavor');
    setSearch('');
    setShowSnapshots(false);
    setCreating(false);
    setProgress(0);
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
              <Progress value={progress}>
                <ProgressLabel>{phase}</ProgressLabel>
                <ProgressValue />
              </Progress>
            </div>
          </div>
        ) : step === 'flavor' ? (
          <form.FormGroup
            name="flavor"
            validators={{ onDynamic: flavorStepSchema }}
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
                          summary={f.summary()}
                          selected={field.state.value === f.id}
                          onSelect={() => {
                            field.handleChange(f.id);
                            form.setFieldValue('version.version', '');
                          }}
                        />
                      ))}
                    </div>
                  )}
                </form.AppField>
              </StepForm>
            )}
          </form.FormGroup>
        ) : step === 'version' ? (
          <form.FormGroup
            name="version"
            validators={{ onDynamic: versionStepSchema }}
            onGroupSubmit={() => setStep('details')}
          >
            {(group) => (
              <StepForm onSubmit={group.handleSubmit} footer={nav}>
                <form.Subscribe selector={(s) => s.values.flavor.flavor}>
                  {(flavor) => {
                    const q = search.trim().toLowerCase();
                    const list = versionsFor(flavor).filter((v) => {
                      if (!showSnapshots && v.kind === 'snapshot') return false;
                      if (q && !v.id.toLowerCase().includes(q)) return false;
                      return true;
                    });
                    return (
                      <div className="flex flex-col gap-3">
                        <div className="relative">
                          <MagnifyingGlassIcon className="-translate-y-1/2 absolute top-1/2 left-2.5 size-3.5 text-muted-foreground" />
                          <Input
                            className="pl-8"
                            placeholder={m['wizard.filter_versions']()}
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                          />
                        </div>

                        {flavor === 'fabric' ? (
                          <form.AppField name="version.loaderVersion">
                            {(field) => (
                              <div className="flex items-center gap-2">
                                <span className="text-xs text-muted-foreground">
                                  {m['label.loader']()}
                                </span>
                                <field.SelectField
                                  options={loaderVersions.map((l) => ({
                                    value: l,
                                    label: l,
                                  }))}
                                  triggerClassName="w-32"
                                />
                              </div>
                            )}
                          </form.AppField>
                        ) : (
                          <label
                            htmlFor="wizard-snapshots"
                            className="flex w-fit cursor-pointer items-center gap-2 text-xs text-muted-foreground"
                          >
                            <Checkbox
                              id="wizard-snapshots"
                              checked={showSnapshots}
                              onCheckedChange={(c) =>
                                setShowSnapshots(c === true)
                              }
                            />
                            {m['wizard.show_snapshots']()}
                          </label>
                        )}

                        <form.AppField name="version.version">
                          {(field) => {
                            const invalid =
                              field.state.meta.isTouched &&
                              field.state.meta.errors.length > 0;
                            return (
                              <div className="flex flex-col gap-1.5">
                                <div className="max-h-52 divide-y divide-border overflow-y-auto border border-border">
                                  {list.length === 0 ? (
                                    <p className="px-3 py-6 text-center text-xs text-muted-foreground">
                                      {m['wizard.no_versions_match']()}
                                    </p>
                                  ) : (
                                    list.map((v) => (
                                      <VersionRow
                                        key={v.id}
                                        version={v}
                                        selected={field.state.value === v.id}
                                        onSelect={() =>
                                          field.handleChange(v.id)
                                        }
                                      />
                                    ))
                                  )}
                                </div>
                                {invalid && (
                                  <FieldError
                                    errors={
                                      field.state.meta.errors as Array<{
                                        message?: string;
                                      }>
                                    }
                                  />
                                )}
                              </div>
                            );
                          }}
                        </form.AppField>
                      </div>
                    );
                  }}
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
                <div className="flex flex-col gap-4">
                  <form.Subscribe
                    selector={(s) =>
                      [
                        s.values.flavor.flavor,
                        s.values.version.version,
                      ] as const
                    }
                  >
                    {([flavor, version]) => (
                      <form.AppField name="details.name">
                        {(field) => (
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
                    {(field) => (
                      <field.SliderField
                        label={m['label.memory']()}
                        formatValue={(v) => m['wizard.gb']({ value: v })}
                        min={2}
                        max={32}
                        step={1}
                      />
                    )}
                  </form.AppField>

                  {kind === 'server' && (
                    <>
                      <SectionHeader>
                        {m['wizard.server_properties']()}
                      </SectionHeader>

                      <form.AppField name="details.motd">
                        {(field) => (
                          <field.TextField label={m['wizard.motd']()} />
                        )}
                      </form.AppField>

                      <div className="grid gap-4 sm:grid-cols-2">
                        <form.AppField name="details.gamemode">
                          {(field) => (
                            <field.SelectField
                              label={m['wizard.gamemode']()}
                              options={options(GAMEMODES)}
                              triggerClassName="w-full"
                            />
                          )}
                        </form.AppField>
                        <form.AppField name="details.difficulty">
                          {(field) => (
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
                          {(field) => (
                            <field.TextField
                              label={m['wizard.max_players']()}
                              type="number"
                            />
                          )}
                        </form.AppField>
                        <form.AppField name="details.port">
                          {(field) => (
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
                        <form.AppField name="details.pvp">
                          {(field) => (
                            <PropToggle
                              id="prop-pvp"
                              label={m['wizard.pvp']()}
                              checked={field.state.value}
                              onChange={field.handleChange}
                            />
                          )}
                        </form.AppField>
                        <form.AppField name="details.onlineMode">
                          {(field) => (
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
                        {(field) => {
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
                                  onCheckedChange={(c) =>
                                    field.handleChange(c === true)
                                  }
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
                                    field.state.meta.errors as Array<{
                                      message?: string;
                                    }>
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
              </StepForm>
            )}
          </form.FormGroup>
        )}
      </DialogContent>
    </Dialog>
  );
}

/**
 * A step's scrolling body wrapped in a `<form>` whose submit runs the group.
 * The `footer` sits below the scroll area but inside the form, so the step
 * nav stays pinned while only the fields scroll — and its submit button still
 * drives the group.
 */
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
      <span className="text-xs text-muted-foreground">{summary}</span>
    </button>
  );
}

function VersionRow({
  version,
  selected,
  onSelect,
}: {
  version: CatalogVersion;
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
      <span className="flex-1 font-mono text-xs">{version.id}</span>
      {version.kind === 'snapshot' && (
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
