import { BroomIcon, CoffeeIcon, TrashIcon } from '@phosphor-icons/react';
import { revalidateLogic } from '@tanstack/react-form';
import { useState } from 'react';
import { toast } from 'sonner';

import { Page } from '@/components/page';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from '@/components/ui/field';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { StatusDot } from '@/components/ui/status-dot';
import { javaReleases, javaRuntimes } from '@/features/settings/mock';
import { settingsDefaults, settingsSchema } from '@/features/settings/schema';
import { useAppForm } from '@/hooks/form';
import { type Locale, useLocale } from '@/hooks/locale';
import { bytes } from '@/lib/format';
import { m } from '@/paraglide/messages.js';
import { locales } from '@/paraglide/runtime.js';
import { useCacheInfo, useClearCache } from '@/queries/cache';
import { useDaemon } from '@/queries/daemon';

/** Endonyms — a language always names itself, whatever locale is active. */
const LANGUAGE_NAMES: Record<string, string> = {
  en: 'English',
  'pt-BR': 'Português (Brasil)',
};

function LanguageField() {
  const { locale, changeLocale } = useLocale();
  return (
    <Field>
      <FieldLabel htmlFor="language">{m['settings.language']()}</FieldLabel>
      <Select
        value={locale}
        onValueChange={(value) => {
          if (value) changeLocale(value as Locale);
        }}
      >
        <SelectTrigger id="language" className="w-full">
          <SelectValue>
            {(value: string) => LANGUAGE_NAMES[value] ?? value}
          </SelectValue>
        </SelectTrigger>
        <SelectContent align="start" alignItemWithTrigger={false}>
          <SelectGroup>
            {locales.map((l) => (
              <SelectItem key={l} value={l}>
                {LANGUAGE_NAMES[l] ?? l}
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
    </Field>
  );
}

export function SettingsPage() {
  const [runtimes, setRuntimes] = useState(javaRuntimes);
  const daemon = useDaemon();
  const cache = useCacheInfo();
  const clearCache = useClearCache();

  const form = useAppForm({
    defaultValues: settingsDefaults,
    validationLogic: revalidateLogic(),
    validators: { onDynamic: settingsSchema() },
    onSubmit: async () => {},
  });

  return (
    <Page
      title={m['nav.settings']()}
      subtitle={m['settings.subtitle']()}
      skeleton={
        <div className="max-w-2xl space-y-8">
          {[0, 1, 2].map((group) => (
            <div key={group} className="space-y-5">
              <Bone className="h-4 w-32" />
              {[0, 1].map((field) => (
                <div key={field} className="space-y-2">
                  <Bone className="h-3 w-24" />
                  <Bone className="h-9 max-w-md" />
                </div>
              ))}
            </div>
          ))}
        </div>
      }
    >
      <form
        className="max-w-2xl"
        onSubmit={(e) => {
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <FieldGroup>
          <FieldSet>
            <FieldLegend>{m['settings.general']()}</FieldLegend>
            <FieldGroup>
              <LanguageField />

              <form.AppField name="theme">
                {(field) => (
                  <field.SelectField
                    label={m['settings.theme']()}
                    options={[
                      { value: 'dark', label: m['settings.theme_dark']() },
                      { value: 'system', label: m['settings.theme_system']() },
                    ]}
                    triggerClassName="w-full"
                  />
                )}
              </form.AppField>

              <form.AppField name="dataDir">
                {(field) => (
                  <field.TextField
                    label={m['settings.data_dir']()}
                    description={m['settings.data_dir_hint']()}
                  />
                )}
              </form.AppField>

              <form.AppField name="startAtLogin">
                {(field) => (
                  <field.CheckboxField label={m['settings.start_at_login']()} />
                )}
              </form.AppField>

              <form.AppField name="keepOpen">
                {(field) => (
                  <field.CheckboxField label={m['settings.keep_open']()} />
                )}
              </form.AppField>
            </FieldGroup>
          </FieldSet>

          <FieldSet>
            <FieldLegend>{m['settings.java_performance']()}</FieldLegend>
            <FieldGroup>
              <form.AppField name="memory">
                {(field) => (
                  <field.SliderField
                    label={m['settings.default_memory']()}
                    formatValue={(v) => m['wizard.gb']({ value: v })}
                    sliderClassName="max-w-md"
                    min={2}
                    max={32}
                    step={1}
                    description={m['settings.default_memory_hint']()}
                  />
                )}
              </form.AppField>

              <form.AppField name="jvmArgs">
                {(field) => (
                  <field.TextField
                    label={m['settings.default_jvm_args']()}
                    inputClassName="font-mono"
                  />
                )}
              </form.AppField>

              <Field>
                <FieldLabel>{m['settings.installed_runtimes']()}</FieldLabel>
                <div className="divide-y divide-border border border-border">
                  {runtimes.map((rt) => (
                    <div
                      key={`${rt.vendor}-${rt.major}`}
                      className="flex items-center gap-3 px-3 py-2"
                    >
                      <CoffeeIcon className="size-4 shrink-0 text-muted-foreground" />
                      <div className="min-w-0 flex-1">
                        <div className="text-sm">
                          {rt.vendor} {rt.major}
                        </div>
                        <div className="font-mono text-[11px] text-muted-foreground">
                          {rt.version}
                        </div>
                      </div>
                      {rt.inUse && (
                        <Badge variant="secondary">
                          {m['settings.in_use']()}
                        </Badge>
                      )}
                      <ConfirmDialog
                        trigger={
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            aria-label={m['settings.uninstall_runtime']()}
                            disabled={rt.inUse}
                          >
                            <TrashIcon className="size-4" />
                          </Button>
                        }
                        title={m['settings.uninstall_runtime_title']()}
                        description={m[
                          'settings.uninstall_runtime_description'
                        ]({ name: `${rt.vendor} ${rt.major}` })}
                        destructive
                        confirmLabel={m['action.uninstall']()}
                        onConfirm={() =>
                          setRuntimes((rts) =>
                            rts.filter((r) => r.major !== rt.major),
                          )
                        }
                      />
                    </div>
                  ))}
                </div>
                <div className="mt-2 flex flex-wrap items-center gap-1.5">
                  <span className="mr-1 text-xs text-muted-foreground">
                    {m['settings.install_prompt']()}
                  </span>
                  {javaReleases.map((r) => (
                    <Button
                      key={r.major}
                      variant="outline"
                      size="xs"
                      disabled={r.installed}
                    >
                      {r.major}
                      {r.lts ? ' LTS' : ''}
                      {r.installed ? ' ✓' : ''}
                    </Button>
                  ))}
                </div>
              </Field>
            </FieldGroup>
          </FieldSet>

          <FieldSet>
            <FieldLegend>{m['settings.storage_daemon']()}</FieldLegend>
            <FieldGroup>
              <Field orientation="horizontal">
                <FieldLabel className="flex-1">
                  {m['settings.download_cache']()}
                  <span className="font-mono text-muted-foreground">
                    {cache.data ? bytes(cache.data.bytes) : '—'}
                  </span>
                </FieldLabel>
                <ConfirmDialog
                  trigger={
                    <Button
                      variant="outline"
                      size="sm"
                      data-icon="inline-start"
                      disabled={clearCache.isPending || !cache.data?.entries}
                    >
                      <BroomIcon />
                      {m['settings.clear_cache']()}
                    </Button>
                  }
                  title={m['settings.clear_cache_title']()}
                  description={m['settings.clear_cache_description']()}
                  confirmLabel={m['settings.clear_cache']()}
                  onConfirm={() =>
                    clearCache.mutate(undefined, {
                      onSuccess: (usage) =>
                        toast.success(
                          m['toast.cache_cleared']({
                            size: bytes(usage.bytes),
                          }),
                        ),
                    })
                  }
                />
              </Field>

              <form.AppField name="shared">
                {(field) => (
                  <field.TextField
                    label={m['settings.shared_config']()}
                    description={m['settings.shared_config_hint']()}
                  />
                )}
              </form.AppField>

              <Field orientation="horizontal">
                <FieldLabel className="flex-1 gap-2 font-normal">
                  <StatusDot tone={daemon.connected ? 'on' : 'off'} />
                  {daemon.connected
                    ? m['daemon.connected_label']()
                    : m['daemon.offline_label']()}
                  {daemon.status && (
                    <span className="font-mono text-muted-foreground">
                      {m['daemon.version_uptime']({
                        version: daemon.status.version,
                        uptime: daemon.uptime ?? '0s',
                      })}
                    </span>
                  )}
                </FieldLabel>
                {daemon.busy ? (
                  <Button variant="outline" size="sm" disabled>
                    {daemon.restart.isPending
                      ? m['daemon.restarting']()
                      : m['daemon.starting']()}
                  </Button>
                ) : daemon.connected ? (
                  <ConfirmDialog
                    trigger={
                      <Button variant="outline" size="sm">
                        {m['daemon.restart']()}
                      </Button>
                    }
                    title={m['daemon.restart_title']()}
                    description={m['daemon.restart_description']()}
                    confirmLabel={m['action.restart']()}
                    onConfirm={() => daemon.restart.mutate()}
                  />
                ) : (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => daemon.start.mutate()}
                  >
                    {m['daemon.start']()}
                  </Button>
                )}
              </Field>
            </FieldGroup>
          </FieldSet>
        </FieldGroup>
      </form>
    </Page>
  );
}
