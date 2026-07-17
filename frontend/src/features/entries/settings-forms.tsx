import { ArrowsClockwiseIcon, TrashIcon } from '@phosphor-icons/react';
import { revalidateLogic } from '@tanstack/react-form';

import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { FieldGroup, FieldSeparator } from '@/components/ui/field';
import type { Instance, Server } from '@/features/entries/mock';
import {
  instanceSettingsDefaults,
  instanceSettingsSchema,
  serverSettingsDefaults,
  serverSettingsSchema,
} from '@/features/entries/schema';
import { useAppForm } from '@/hooks/form';

const MC_VERSIONS = ['1.21.4', '1.21.1', '1.20.1', '1.19.2'];
const LOADERS = ['vanilla', 'fabric'];
const INTERVALS = ['', '6h', '12h', '1d'];

function DangerZone({
  noun,
  name,
  children,
}: {
  noun: 'server' | 'instance';
  name: string;
  children?: React.ReactNode;
}) {
  return (
    <>
      <FieldSeparator />
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" size="sm" data-icon="inline-start">
          <ArrowsClockwiseIcon />
          Change version
        </Button>
        {children}
        <ConfirmDialog
          trigger={
            <Button variant="destructive" size="sm" data-icon="inline-start">
              <TrashIcon />
              Remove {noun}
            </Button>
          }
          title={`Remove ${noun}?`}
          description={
            <>
              <span className="font-medium text-foreground">{name}</span> and
              all its data — worlds, backups, and installed content — are
              permanently deleted. This cannot be undone.
            </>
          }
          destructive
          confirmLabel={`Remove ${noun}`}
          onConfirm={() => {}}
        />
      </div>
    </>
  );
}

export function InstanceSettingsForm({ inst }: { inst: Instance }) {
  const form = useAppForm({
    defaultValues: instanceSettingsDefaults(inst),
    validationLogic: revalidateLogic(),
    validators: { onDynamic: instanceSettingsSchema() },
    onSubmit: async () => {},
  });

  return (
    <form
      className="max-w-lg"
      onSubmit={(e) => {
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <FieldGroup>
        <form.AppField name="name">
          {(field) => <field.TextField label="Instance name" />}
        </form.AppField>

        <div className="grid gap-4 sm:grid-cols-2">
          <form.AppField name="version">
            {(field) => (
              <field.SelectField
                label="Minecraft version"
                options={MC_VERSIONS.map((v) => ({ value: v, label: v }))}
                triggerClassName="w-full"
              />
            )}
          </form.AppField>
          <form.AppField name="loader">
            {(field) => (
              <field.SelectField
                label="Mod loader"
                options={LOADERS.map((l) => ({
                  value: l,
                  label: l,
                  className: 'capitalize',
                }))}
                triggerClassName="w-full capitalize"
              />
            )}
          </form.AppField>
        </div>

        <form.AppField name="memory">
          {(field) => (
            <field.SliderField
              label="Allocated memory"
              formatValue={(v) => `${v} GB`}
              sliderClassName="max-w-md"
              min={2}
              max={32}
              step={1}
            />
          )}
        </form.AppField>

        <form.AppField name="jvmArgs">
          {(field) => (
            <field.TextField
              label="Java arguments"
              placeholder="-XX:+UseG1GC"
              inputClassName="font-mono"
            />
          )}
        </form.AppField>

        <DangerZone noun="instance" name={inst.name} />
      </FieldGroup>
    </form>
  );
}

export function ServerSettingsForm({ server }: { server: Server }) {
  const form = useAppForm({
    defaultValues: serverSettingsDefaults(server),
    validationLogic: revalidateLogic(),
    validators: { onDynamic: serverSettingsSchema() },
    onSubmit: async () => {},
  });

  return (
    <form
      className="max-w-lg"
      onSubmit={(e) => {
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <FieldGroup>
        <form.AppField name="name">
          {(field) => <field.TextField label="Server name" />}
        </form.AppField>

        <form.AppField name="memory">
          {(field) => (
            <field.SliderField
              label="Allocated memory"
              formatValue={(v) => `${v} GB`}
              sliderClassName="max-w-md"
              min={2}
              max={32}
              step={1}
            />
          )}
        </form.AppField>

        <form.AppField name="jvmArgs">
          {(field) => (
            <field.TextField
              label="Java arguments"
              placeholder="-XX:+UseG1GC"
              inputClassName="font-mono"
            />
          )}
        </form.AppField>

        <div className="grid gap-4 sm:grid-cols-2">
          <form.AppField name="backupInterval">
            {(field) => (
              <field.SelectField
                label="Backup schedule"
                options={INTERVALS.map((iv) => ({
                  value: iv || 'off',
                  label: iv ? `Every ${iv}` : 'Off',
                }))}
                triggerClassName="w-full"
              />
            )}
          </form.AppField>

          <form.AppField name="backupRetention">
            {(field) => (
              <field.NumberField
                label="Keep backups"
                min={1}
                description="Newest scheduled archives."
              />
            )}
          </form.AppField>
        </div>

        <DangerZone noun="server" name={server.name} />
      </FieldGroup>
    </form>
  );
}
