import {
  CaretLeftIcon,
  CaretRightIcon,
  MagnifyingGlassIcon,
} from '@phosphor-icons/react';
import { useForm } from '@tanstack/react-form';
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
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from '@/components/ui/progress';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import {
  type CatalogVersion,
  flavors,
  loaderVersions,
  versionsFor,
} from '@/features/entries/catalog';
import { cn } from '@/lib/utils';

type Kind = 'server' | 'instance';
type Step = 'flavor' | 'version' | 'details';

const STEPS: Step[] = ['flavor', 'version', 'details'];

/** A curated slice of `server.properties` surfaced in the wizard. */
const GAMEMODES = ['survival', 'creative', 'adventure', 'spectator'];
const DIFFICULTIES = ['peaceful', 'easy', 'normal', 'hard'];

/** Strip the browser's number-input spinner buttons. */
const NO_SPINNER =
  '[appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none';

/** The create flow the daemon runs, faked as timed phases for the wizard. */
const PHASES: Record<Kind, string[]> = {
  server: [
    'Resolving profile',
    'Installing Java runtime',
    'Downloading server',
    'Generating properties',
    'Ready',
  ],
  instance: ['Creating record', 'Ready'],
};

const STEP_HINTS: Record<Step, (noun: string) => string> = {
  flavor: (noun) => `Pick the distribution your ${noun} runs.`,
  version: () => 'Choose the Minecraft version.',
  details: (noun) => `Name your ${noun} and set its resources.`,
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/**
 * The New server / New instance wizard: flavor → version → details, mirroring
 * the CLI's interactive `create`. A server also gates on the EULA. Submitting
 * plays the daemon's provisioning phases as a progress bar — nothing is
 * persisted, the library is static mock data.
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

  const form = useForm({
    defaultValues: {
      flavor: 'vanilla',
      version: '',
      loaderVersion: loaderVersions[0],
      name: '',
      memory: 4,
      port: '',
      motd: 'A Minecraft Server',
      gamemode: 'survival',
      difficulty: 'normal',
      maxPlayers: '20',
      pvp: true,
      onlineMode: true,
      eula: false,
    },
    onSubmit: async () => {
      setCreating(true);
      const phases = PHASES[kind];
      for (let i = 0; i < phases.length; i++) {
        setPhase(phases[i]);
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
  const noun = kind === 'server' ? 'server' : 'instance';
  const stepIndex = STEPS.indexOf(step);
  const back = () => setStep(STEPS[Math.max(0, stepIndex - 1)]);
  const next = () => setStep(STEPS[Math.min(STEPS.length - 1, stepIndex + 1)]);

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
            New {noun}
          </DialogTitle>
          <DialogDescription>
            {creating ? `Provisioning your ${noun}…` : STEP_HINTS[step](noun)}
          </DialogDescription>
        </DialogHeader>

        <div className="min-h-[18rem] max-h-[58vh] overflow-x-hidden overflow-y-auto p-1">
          {creating ? (
            <div className="flex min-h-[18rem] flex-col justify-center px-1">
              <Progress value={progress}>
                <ProgressLabel>{phase}</ProgressLabel>
                <ProgressValue />
              </Progress>
            </div>
          ) : step === 'flavor' ? (
            <form.Field name="flavor">
              {(field) => (
                <div className="grid gap-2">
                  {flavors.map((f) => (
                    <FlavorOption
                      key={f.id}
                      name={f.name}
                      summary={f.summary}
                      selected={field.state.value === f.id}
                      onSelect={() => {
                        field.handleChange(f.id);
                        form.setFieldValue('version', '');
                      }}
                    />
                  ))}
                </div>
              )}
            </form.Field>
          ) : step === 'version' ? (
            <form.Subscribe selector={(s) => s.values.flavor}>
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
                        placeholder="Filter versions"
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                      />
                    </div>

                    {flavor === 'fabric' ? (
                      <form.Field name="loaderVersion">
                        {(field) => (
                          <div className="flex items-center gap-2">
                            <span className="text-xs text-muted-foreground">
                              Loader
                            </span>
                            <Select
                              value={field.state.value}
                              onValueChange={(v) => {
                                if (v) field.handleChange(v);
                              }}
                            >
                              <SelectTrigger size="sm" className="w-32">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent
                                align="start"
                                alignItemWithTrigger={false}
                              >
                                <SelectGroup>
                                  {loaderVersions.map((l) => (
                                    <SelectItem key={l} value={l}>
                                      {l}
                                    </SelectItem>
                                  ))}
                                </SelectGroup>
                              </SelectContent>
                            </Select>
                          </div>
                        )}
                      </form.Field>
                    ) : (
                      <label
                        htmlFor="wizard-snapshots"
                        className="flex w-fit cursor-pointer items-center gap-2 text-xs text-muted-foreground"
                      >
                        <Checkbox
                          id="wizard-snapshots"
                          checked={showSnapshots}
                          onCheckedChange={(c) => setShowSnapshots(c === true)}
                        />
                        Show snapshots
                      </label>
                    )}

                    <form.Field name="version">
                      {(field) => (
                        <div className="max-h-52 divide-y divide-border overflow-y-auto border border-border">
                          {list.length === 0 ? (
                            <p className="px-3 py-6 text-center text-xs text-muted-foreground">
                              No versions match.
                            </p>
                          ) : (
                            list.map((v) => (
                              <VersionRow
                                key={v.id}
                                version={v}
                                selected={field.state.value === v.id}
                                onSelect={() => field.handleChange(v.id)}
                              />
                            ))
                          )}
                        </div>
                      )}
                    </form.Field>
                  </div>
                );
              }}
            </form.Subscribe>
          ) : (
            <div className="flex flex-col gap-4">
              <form.Subscribe
                selector={(s) => [s.values.flavor, s.values.version] as const}
              >
                {([flavor, version]) => (
                  <form.Field name="name">
                    {(field) => (
                      <Field>
                        <FieldLabel htmlFor={field.name}>Name</FieldLabel>
                        <Input
                          id={field.name}
                          value={field.state.value}
                          placeholder={`${flavor}-${version}`}
                          onBlur={field.handleBlur}
                          onChange={(e) => field.handleChange(e.target.value)}
                        />
                        <FieldDescription>
                          Leave blank to use the flavor and version.
                        </FieldDescription>
                      </Field>
                    )}
                  </form.Field>
                )}
              </form.Subscribe>

              <form.Field name="memory">
                {(field) => (
                  <Field>
                    <FieldLabel htmlFor={field.name}>
                      Memory — {field.state.value} GB
                    </FieldLabel>
                    <Slider
                      id={field.name}
                      min={2}
                      max={32}
                      step={1}
                      value={field.state.value}
                      onValueChange={(v) =>
                        field.handleChange(Array.isArray(v) ? v[0] : v)
                      }
                    />
                  </Field>
                )}
              </form.Field>

              {kind === 'server' && (
                <>
                  <SectionHeader>Server properties</SectionHeader>

                  <form.Field name="motd">
                    {(field) => (
                      <Field>
                        <FieldLabel htmlFor={field.name}>
                          Message of the day
                        </FieldLabel>
                        <Input
                          id={field.name}
                          value={field.state.value}
                          onBlur={field.handleBlur}
                          onChange={(e) => field.handleChange(e.target.value)}
                        />
                      </Field>
                    )}
                  </form.Field>

                  <div className="grid gap-4 sm:grid-cols-2">
                    <form.Field name="gamemode">
                      {(field) => (
                        <PropSelect
                          id={field.name}
                          label="Gamemode"
                          value={field.state.value}
                          options={GAMEMODES}
                          onChange={field.handleChange}
                        />
                      )}
                    </form.Field>

                    <form.Field name="difficulty">
                      {(field) => (
                        <PropSelect
                          id={field.name}
                          label="Difficulty"
                          value={field.state.value}
                          options={DIFFICULTIES}
                          onChange={field.handleChange}
                        />
                      )}
                    </form.Field>
                  </div>

                  <div className="grid gap-4 sm:grid-cols-2">
                    <form.Field name="maxPlayers">
                      {(field) => (
                        <Field>
                          <FieldLabel htmlFor={field.name}>
                            Max players
                          </FieldLabel>
                          <Input
                            id={field.name}
                            type="number"
                            min={1}
                            className={NO_SPINNER}
                            value={field.state.value}
                            onBlur={field.handleBlur}
                            onChange={(e) => field.handleChange(e.target.value)}
                          />
                        </Field>
                      )}
                    </form.Field>

                    <form.Field name="port">
                      {(field) => (
                        <Field>
                          <FieldLabel htmlFor={field.name}>Port</FieldLabel>
                          <Input
                            id={field.name}
                            type="number"
                            placeholder="auto"
                            className={NO_SPINNER}
                            value={field.state.value}
                            onBlur={field.handleBlur}
                            onChange={(e) => field.handleChange(e.target.value)}
                          />
                          <FieldDescription>
                            Blank picks the lowest free.
                          </FieldDescription>
                        </Field>
                      )}
                    </form.Field>
                  </div>

                  <div className="grid grid-cols-2 gap-4 pt-1">
                    <form.Field name="pvp">
                      {(field) => (
                        <PropToggle
                          id="prop-pvp"
                          label="PVP"
                          checked={field.state.value}
                          onChange={field.handleChange}
                        />
                      )}
                    </form.Field>
                    <form.Field name="onlineMode">
                      {(field) => (
                        <PropToggle
                          id="prop-online"
                          label="Online mode"
                          checked={field.state.value}
                          onChange={field.handleChange}
                        />
                      )}
                    </form.Field>
                  </div>
                </>
              )}

              {kind === 'server' && (
                <form.Field name="eula">
                  {(field) => (
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
                        I accept the{' '}
                        <a
                          href="https://aka.ms/MinecraftEULA"
                          target="_blank"
                          rel="noreferrer"
                          className="text-foreground underline underline-offset-2"
                        >
                          Minecraft EULA
                        </a>
                        . A server won't start until you do.
                      </span>
                    </label>
                  )}
                </form.Field>
              )}
            </div>
          )}
        </div>

        {!creating && (
          <DialogFooter className="items-center">
            <StepDots active={stepIndex} className="mr-auto" />
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

            {step === 'details' ? (
              <form.Subscribe selector={(s) => s.values.eula}>
                {(eula) => (
                  <Button
                    disabled={kind === 'server' && !eula}
                    className="bg-ember text-ember-foreground hover:bg-ember/90"
                    onClick={() => form.handleSubmit()}
                  >
                    Create {noun}
                  </Button>
                )}
              </form.Subscribe>
            ) : (
              <form.Subscribe selector={(s) => s.values.version}>
                {(version) => (
                  <Button
                    disabled={step === 'version' && !version}
                    onClick={next}
                    data-icon="inline-end"
                    className="bg-ember text-ember-foreground hover:bg-ember/90"
                  >
                    Next
                    <CaretRightIcon />
                  </Button>
                )}
              </form.Subscribe>
            )}
          </DialogFooter>
        )}
      </DialogContent>
    </Dialog>
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

function PropSelect({
  id,
  label,
  value,
  options,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  options: string[];
  onChange: (value: string) => void;
}) {
  return (
    <Field>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Select
        value={value}
        onValueChange={(v) => {
          if (v) onChange(v);
        }}
      >
        <SelectTrigger id={id} className="w-full capitalize">
          <SelectValue />
        </SelectTrigger>
        <SelectContent align="start" alignItemWithTrigger={false}>
          <SelectGroup>
            {options.map((o) => (
              <SelectItem key={o} value={o} className="capitalize">
                {o}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    </Field>
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
          snapshot
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
