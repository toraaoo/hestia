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
import { m } from '@/paraglide/messages.js';

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
  const removeLabel =
    noun === 'server'
      ? m['entry_settings.remove_server']()
      : m['entry_settings.remove_instance']();
  return (
    <>
      <FieldSeparator />
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" size="sm" data-icon="inline-start">
          <ArrowsClockwiseIcon />
          {m['entry_settings.change_version']()}
        </Button>
        {children}
        <ConfirmDialog
          trigger={
            <Button variant="destructive" size="sm" data-icon="inline-start">
              <TrashIcon />
              {removeLabel}
            </Button>
          }
          title={
            noun === 'server'
              ? m['entry_settings.remove_server_title']()
              : m['entry_settings.remove_instance_title']()
          }
          description={m['entry_settings.remove_description']({ name })}
          destructive
          confirmLabel={removeLabel}
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
          {(field) => (
            <field.TextField label={m['entry_settings.instance_name']()} />
          )}
        </form.AppField>

        <div className="grid gap-4 sm:grid-cols-2">
          <form.AppField name="version">
            {(field) => (
              <field.SelectField
                label={m['entry_settings.minecraft_version']()}
                options={MC_VERSIONS.map((v) => ({ value: v, label: v }))}
                triggerClassName="w-full"
              />
            )}
          </form.AppField>
          <form.AppField name="loader">
            {(field) => (
              <field.SelectField
                label={m['entry_settings.mod_loader']()}
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
              label={m['entry_settings.allocated_memory']()}
              formatValue={(v) => m['wizard.gb']({ value: v })}
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
              label={m['entry_settings.java_arguments']()}
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
          {(field) => (
            <field.TextField label={m['entry_settings.server_name']()} />
          )}
        </form.AppField>

        <form.AppField name="memory">
          {(field) => (
            <field.SliderField
              label={m['entry_settings.allocated_memory']()}
              formatValue={(v) => m['wizard.gb']({ value: v })}
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
              label={m['entry_settings.java_arguments']()}
              placeholder="-XX:+UseG1GC"
              inputClassName="font-mono"
            />
          )}
        </form.AppField>

        <div className="grid gap-4 sm:grid-cols-2">
          <form.AppField name="backupInterval">
            {(field) => (
              <field.SelectField
                label={m['entry_settings.backup_schedule']()}
                options={INTERVALS.map((iv) => ({
                  value: iv || 'off',
                  label: iv
                    ? m['entry_settings.every_interval']({ interval: iv })
                    : m['label.off'](),
                }))}
                triggerClassName="w-full"
              />
            )}
          </form.AppField>

          <form.AppField name="backupRetention">
            {(field) => (
              <field.NumberField
                label={m['entry_settings.keep_backups']()}
                min={1}
                description={m['entry_settings.keep_backups_hint']()}
              />
            )}
          </form.AppField>
        </div>

        <DangerZone noun="server" name={server.name} />
      </FieldGroup>
    </form>
  );
}
