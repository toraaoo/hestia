import { CaretLeftIcon, CaretRightIcon } from '@phosphor-icons/react';
import { revalidateLogic } from '@tanstack/react-form';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';

import type { ConfigEntry, Flavor } from '@/api';
import { entryIcon } from '@/components/icons';
import { StepDots } from '@/components/step-dots';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ProvisionProgressView } from '@/features/entries/components/provision-progress';
import {
  createWizardDefaults,
  createWizardSchema,
  detailsStepSchema,
  flavorStepSchema,
  versionStepSchema,
} from '@/features/entries/lib/schema';
import { useAppForm } from '@/hooks/form';
import { m } from '@/paraglide/messages.js';
import { instanceMutations, instanceQueries } from '@/queries/instance';
import { backgroundJob, foregroundJob, useJobMutation } from '@/queries/jobs';
import { serverMutations, serverQueries } from '@/queries/server';

import {
  FlavorOption,
  flavorSummary,
  type Kind,
  STEP_HINTS,
  STEPS,
  type Step,
  StepForm,
} from './fields';
import { DetailsStep } from './steps/details';
import { VersionStep } from './steps/version';

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

  const createServer = useJobMutation(serverMutations.create());
  const createInstance = useMutation(instanceMutations.create());
  const creating = createServer.isPending || createInstance.isPending;
  const progress = createServer.progress;
  const job = createServer.job;

  useEffect(() => {
    if (creating && job?.status === 'running') foregroundJob(job.id);
  }, [creating, job?.id, job?.status]);

  const close = () => {
    if (job?.status === 'running') backgroundJob(job.id);
    onOpenChange(false);
  };

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
      } catch {
        // The global mutation error handler surfaces the toast; swallow here so
        // the dialog stays open without a second notification.
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
      <StepDots steps={STEPS} active={stepIndex} className="mr-auto" />
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
      onOpenChange={(o) => (o ? onOpenChange(true) : close())}
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
          <ProvisionProgressView
            progress={progress}
            indeterminate={kind === 'instance'}
            className="min-h-[18rem] justify-center px-1"
          />
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
    ...instanceParams(value),
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
    loaderVersion: value.version.loaderVersion || undefined,
    config: [{ key: 'memory', value: `${d.memory}G` }],
  };
}
