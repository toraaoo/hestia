import { ArrowsClockwiseIcon, TrashIcon } from '@phosphor-icons/react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useEffect, useMemo, useState } from 'react';
import { toast } from 'sonner';

import type { ConfigEntry, InstanceInfo } from '@/api';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldSeparator,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { memGb } from '@/lib/format';
import { m } from '@/paraglide/messages.js';
import { instanceMutations, instanceQueries } from '@/queries/instance';

function configValue(config: ConfigEntry[] | undefined, key: string): string {
  return config?.find((e) => e.key === key)?.value ?? '';
}

export function InstanceSettingsTab({
  instance,
  config,
  running,
}: {
  instance: InstanceInfo;
  config?: ConfigEntry[];
  running: boolean;
}) {
  const navigate = useNavigate();
  const rename = useMutation(instanceMutations.rename(instance.id));
  const setConfig = useMutation(instanceMutations.setConfig(instance.id));
  const remove = useMutation(instanceMutations.remove(instance.id));

  const [name, setName] = useState(instance.name);
  const [memory, setMemory] = useState(4);
  const [jvmArgs, setJvmArgs] = useState('');
  const [changing, setChanging] = useState(false);

  useEffect(() => {
    setName(instance.name);
  }, [instance.name]);

  useEffect(() => {
    if (!config) return;
    setMemory(memGb(configValue(config, 'memory') || '4G'));
    setJvmArgs(configValue(config, 'jvm-args'));
  }, [config]);

  const saveConfig = async () => {
    try {
      await setConfig.mutateAsync({ key: 'memory', value: `${memory}G` });
      await setConfig.mutateAsync({ key: 'jvm-args', value: jvmArgs });
      toast.success(m['toast.saved']());
    } catch (error) {
      toast.error(error instanceof Error ? error.message : String(error));
    }
  };

  const doRename = () => {
    const trimmed = name.trim();
    if (!trimmed || trimmed === instance.name) return;
    rename.mutate(trimmed, {
      onSuccess: (updated) =>
        toast.success(m['toast.renamed']({ name: updated.name })),
      onError: (error) => toast.error(error.message),
    });
  };

  return (
    <div className="max-w-lg">
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="instance-name">
            {m['entry_settings.instance_name']()}
          </FieldLabel>
          <div className="flex gap-2">
            <Input
              id="instance-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              disabled={running}
            />
            <Button
              variant="outline"
              onClick={doRename}
              disabled={running || rename.isPending || name === instance.name}
            >
              {m['action.apply']()}
            </Button>
          </div>
        </Field>

        <Field>
          <FieldLabel>
            {m['entry_settings.allocated_memory']()}
            <span className="ml-2 font-mono text-muted-foreground">
              {m['wizard.gb']({ value: memory })}
            </span>
          </FieldLabel>
          <Slider
            value={[memory]}
            min={2}
            max={32}
            step={1}
            onValueChange={(v) => setMemory(Array.isArray(v) ? v[0] : v)}
            className="max-w-md"
          />
        </Field>

        <Field>
          <FieldLabel htmlFor="jvm-args">
            {m['entry_settings.java_arguments']()}
          </FieldLabel>
          <Input
            id="jvm-args"
            value={jvmArgs}
            onChange={(e) => setJvmArgs(e.target.value)}
            placeholder="-XX:+UseG1GC"
            className="font-mono"
          />
        </Field>

        <div>
          <Button onClick={saveConfig} disabled={setConfig.isPending}>
            {m['action.apply']()}
          </Button>
        </div>

        <FieldSeparator />

        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            size="sm"
            data-icon="inline-start"
            disabled={running}
            onClick={() => setChanging(true)}
          >
            <ArrowsClockwiseIcon />
            {m['entry_settings.change_version']()}
          </Button>
          <ConfirmDialog
            trigger={
              <Button
                variant="destructive"
                size="sm"
                data-icon="inline-start"
                disabled={running}
              >
                <TrashIcon />
                {m['entry_settings.remove_instance']()}
              </Button>
            }
            title={m['entry_settings.remove_instance_title']()}
            description={m['entry_settings.remove_description']({
              name: instance.name,
            })}
            destructive
            confirmLabel={m['entry_settings.remove_instance']()}
            onConfirm={() =>
              remove.mutate(undefined, {
                onSuccess: () => {
                  toast.success(m['toast.removed']({ name: instance.name }));
                  navigate({ to: '/instances' });
                },
                onError: (error) => toast.error(error.message),
              })
            }
          />
        </div>
      </FieldGroup>

      <ChangeVersionDialog
        instance={instance}
        open={changing}
        onOpenChange={setChanging}
      />
    </div>
  );
}

function ChangeVersionDialog({
  instance,
  open,
  onOpenChange,
}: {
  instance: InstanceInfo;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const versions = useQuery(instanceQueries.versions(instance.flavor));
  const update = useMutation(instanceMutations.update(instance.id));
  const [version, setVersion] = useState('');
  const [downgrade, setDowngrade] = useState(false);

  const options = useMemo(
    () => (versions.data ?? []).filter((v) => v.id !== instance.gameVersion),
    [versions.data, instance.gameVersion],
  );

  const pending = update.isPending;

  const apply = async () => {
    if (!version) return;
    try {
      await update.mutateAsync({ version, allowDowngrade: downgrade });
      toast.success(m['toast.updated']({ name: instance.name }));
      onOpenChange(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{m['entry_settings.change_version']()}</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          <Select
            value={version}
            onValueChange={(v) => setVersion(v ?? '')}
            disabled={pending}
          >
            <SelectTrigger className="w-full">
              <SelectValue placeholder={m['label.version']()} />
            </SelectTrigger>
            <SelectContent>
              {options.map((v) => (
                <SelectItem key={v.id} value={v.id} className="font-mono">
                  {v.id}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <label
            htmlFor="allow-downgrade"
            className="flex cursor-pointer items-center gap-2 text-xs text-muted-foreground"
          >
            <Checkbox
              id="allow-downgrade"
              checked={downgrade}
              onCheckedChange={(c) => setDowngrade(c === true)}
              disabled={pending}
            />
            {m['entry_settings.allow_downgrade']()}
          </label>
        </div>
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={pending}
          >
            {m['action.cancel']()}
          </Button>
          <Button onClick={apply} disabled={!version || pending}>
            {pending
              ? m['status.preparing']()
              : m['entry_settings.change_version']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
