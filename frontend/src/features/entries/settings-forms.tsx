import { ArrowsClockwiseIcon, TrashIcon } from '@phosphor-icons/react';
import { useForm } from '@tanstack/react-form';

import { Button } from '@/components/ui/button';
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldSeparator,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import type { Instance, Server } from '@/features/entries/mock';
import { memGb } from '@/lib/format';

const MC_VERSIONS = ['1.21.4', '1.21.1', '1.20.1', '1.19.2'];
const LOADERS = ['vanilla', 'fabric'];
const INTERVALS = ['', '6h', '12h', '1d'];

function DangerZone({
  removeLabel,
  children,
}: {
  removeLabel: string;
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
        <Button variant="destructive" size="sm" data-icon="inline-start">
          <TrashIcon />
          {removeLabel}
        </Button>
      </div>
    </>
  );
}

export function InstanceSettingsForm({ inst }: { inst: Instance }) {
  const form = useForm({
    defaultValues: {
      name: inst.name,
      version: inst.game_version,
      loader: inst.flavor,
      memory: memGb(inst.memory),
      jvmArgs: '',
    },
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
        <form.Field name="name">
          {(field) => (
            <Field>
              <FieldLabel htmlFor={field.name}>Instance name</FieldLabel>
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
          <form.Field name="version">
            {(field) => (
              <Field>
                <FieldLabel htmlFor={field.name}>Minecraft version</FieldLabel>
                <Select
                  value={field.state.value}
                  onValueChange={(v) => {
                    if (v) field.handleChange(v);
                  }}
                >
                  <SelectTrigger id={field.name} className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      {MC_VERSIONS.map((v) => (
                        <SelectItem key={v} value={v}>
                          {v}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </Field>
            )}
          </form.Field>

          <form.Field name="loader">
            {(field) => (
              <Field>
                <FieldLabel htmlFor={field.name}>Mod loader</FieldLabel>
                <Select
                  value={field.state.value}
                  onValueChange={(v) => {
                    if (v) field.handleChange(v);
                  }}
                >
                  <SelectTrigger id={field.name} className="w-full capitalize">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      {LOADERS.map((l) => (
                        <SelectItem key={l} value={l} className="capitalize">
                          {l}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </Field>
            )}
          </form.Field>
        </div>

        <form.Field name="memory">
          {(field) => (
            <Field>
              <FieldLabel htmlFor={field.name}>
                Allocated memory — {field.state.value} GB
              </FieldLabel>
              <Slider
                id={field.name}
                className="max-w-md"
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

        <form.Field name="jvmArgs">
          {(field) => (
            <Field>
              <FieldLabel htmlFor={field.name}>Java arguments</FieldLabel>
              <Input
                id={field.name}
                className="font-mono"
                placeholder="-XX:+UseG1GC"
                value={field.state.value}
                onBlur={field.handleBlur}
                onChange={(e) => field.handleChange(e.target.value)}
              />
            </Field>
          )}
        </form.Field>

        <DangerZone removeLabel="Remove instance" />
      </FieldGroup>
    </form>
  );
}

export function ServerSettingsForm({ server }: { server: Server }) {
  const form = useForm({
    defaultValues: {
      name: server.name,
      memory: memGb(server.memory),
      jvmArgs: '',
      backupInterval: server.backup_interval,
      backupRetention: server.backup_retention,
    },
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
        <form.Field name="name">
          {(field) => (
            <Field>
              <FieldLabel htmlFor={field.name}>Server name</FieldLabel>
              <Input
                id={field.name}
                value={field.state.value}
                onBlur={field.handleBlur}
                onChange={(e) => field.handleChange(e.target.value)}
              />
            </Field>
          )}
        </form.Field>

        <form.Field name="memory">
          {(field) => (
            <Field>
              <FieldLabel htmlFor={field.name}>
                Allocated memory — {field.state.value} GB
              </FieldLabel>
              <Slider
                id={field.name}
                className="max-w-md"
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

        <form.Field name="jvmArgs">
          {(field) => (
            <Field>
              <FieldLabel htmlFor={field.name}>Java arguments</FieldLabel>
              <Input
                id={field.name}
                className="font-mono"
                placeholder="-XX:+UseG1GC"
                value={field.state.value}
                onBlur={field.handleBlur}
                onChange={(e) => field.handleChange(e.target.value)}
              />
            </Field>
          )}
        </form.Field>

        <div className="grid gap-4 sm:grid-cols-2">
          <form.Field name="backupInterval">
            {(field) => (
              <Field>
                <FieldLabel htmlFor={field.name}>Backup schedule</FieldLabel>
                <Select
                  value={field.state.value || 'off'}
                  onValueChange={(v) => {
                    if (v) field.handleChange(v === 'off' ? '' : v);
                  }}
                >
                  <SelectTrigger id={field.name} className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      {INTERVALS.map((iv) => (
                        <SelectItem key={iv || 'off'} value={iv || 'off'}>
                          {iv ? `Every ${iv}` : 'Off'}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </Field>
            )}
          </form.Field>

          <form.Field name="backupRetention">
            {(field) => (
              <Field>
                <FieldLabel htmlFor={field.name}>Keep backups</FieldLabel>
                <Input
                  id={field.name}
                  type="number"
                  min={1}
                  value={field.state.value}
                  onBlur={field.handleBlur}
                  onChange={(e) =>
                    field.handleChange(Number(e.target.value) || 1)
                  }
                />
                <FieldDescription>Newest scheduled archives.</FieldDescription>
              </Field>
            )}
          </form.Field>
        </div>

        <DangerZone removeLabel="Remove server" />
      </FieldGroup>
    </form>
  );
}
