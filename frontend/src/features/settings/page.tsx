import { BroomIcon, CoffeeIcon, TrashIcon } from '@phosphor-icons/react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';

import { Page } from '@/components/page';
import { Bone } from '@/components/skeleton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Slider } from '@/components/ui/slider';
import { StatusDot } from '@/components/ui/status-dot';
import {
  CheckboxRow,
  LanguageField,
} from '@/features/settings/components/fields';
import { SyncSection } from '@/features/settings/components/sync-section';
import { bytes, memGb } from '@/lib/format';
import { m } from '@/paraglide/messages.js';
import { cacheMutations, cacheQueries } from '@/queries/cache';
import { configMutations, configQueries } from '@/queries/config';
import { useDaemon } from '@/queries/daemon';
import { javaMutations, javaQueries } from '@/queries/java';
import { useJobMutation } from '@/queries/jobs';
import { usePrefs } from '@/queries/prefs';

/** The daemon config entries the settings page reads (`config.list`). */
interface ConfigEntries {
  home?: string;
  autostart?: boolean;
  defaults?: { memory?: string; 'jvm-args'?: string };
}

export function SettingsPage() {
  const daemon = useDaemon();
  const cache = useQuery(cacheQueries.info());
  const clearCache = useMutation(cacheMutations.clear());
  const config = useQuery(configQueries.list());
  const setConfig = useMutation(configMutations.set());
  const prefs = usePrefs();
  const runtimesQuery = useQuery(javaQueries.runtimes());
  const releasesQuery = useQuery(javaQueries.releases());
  const install = useJobMutation(javaMutations.install());
  const uninstall = useMutation(javaMutations.uninstall());

  const entries = (config.data ?? {}) as ConfigEntries;
  const runtimes = runtimesQuery.data ?? [];
  const releases = releasesQuery.data ?? [];
  const installedMajors = new Set(runtimes.map((rt) => rt.major));

  const commitConfig = (key: string, value: unknown) =>
    setConfig.mutate(
      { key, value },
      { onError: (error) => toast.error(error.message) },
    );

  const defaultMemory = entries.defaults?.memory ?? '';
  const [memoryDraft, setMemoryDraft] = useState<number | null>(null);
  const memoryValue = memoryDraft ?? (defaultMemory ? memGb(defaultMemory) : 4);

  return (
    <Page
      title={m['nav.settings']()}
      subtitle={m['settings.subtitle']()}
      loading={config.isPending || !prefs.ready}
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
      <div className="max-w-2xl">
        <FieldGroup>
          <FieldSet>
            <FieldLegend>{m['settings.general']()}</FieldLegend>
            <FieldGroup>
              <LanguageField />

              <Field>
                <FieldLabel htmlFor="data-dir">
                  {m['settings.data_dir']()}
                </FieldLabel>
                <Input
                  id="data-dir"
                  key={entries.home ?? ''}
                  defaultValue={entries.home ?? ''}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') e.currentTarget.blur();
                  }}
                  onBlur={(e) => {
                    const value = e.target.value.trim();
                    if (value && value !== entries.home)
                      commitConfig('home', value);
                  }}
                />
                <FieldDescription>
                  {m['settings.data_dir_hint']()}
                </FieldDescription>
              </Field>

              <CheckboxRow
                id="start-at-login"
                label={m['settings.start_at_login']()}
                checked={entries.autostart ?? false}
                onChange={(checked) => commitConfig('autostart', checked)}
              />

              <CheckboxRow
                id="keep-open"
                label={m['settings.keep_open']()}
                checked={prefs.get('keepOpen', true)}
                onChange={(checked) => prefs.set('keepOpen', checked)}
              />
            </FieldGroup>
          </FieldSet>

          <FieldSet>
            <FieldLegend>{m['settings.java_performance']()}</FieldLegend>
            <FieldGroup>
              <Field>
                <FieldLabel htmlFor="default-memory">
                  {m['settings.default_memory']()} —{' '}
                  {defaultMemory || memoryDraft !== null
                    ? m['wizard.gb']({ value: memoryValue })
                    : m['settings.no_default']()}
                </FieldLabel>
                <Slider
                  id="default-memory"
                  className="max-w-md"
                  min={2}
                  max={32}
                  step={1}
                  value={memoryValue}
                  onValueChange={(v) =>
                    setMemoryDraft(Array.isArray(v) ? v[0] : v)
                  }
                  onValueCommitted={(v) => {
                    setMemoryDraft(null);
                    const gb = Array.isArray(v) ? v[0] : v;
                    commitConfig('defaults.memory', `${gb}G`);
                  }}
                />
                <FieldDescription>
                  {m['settings.default_memory_hint']()}
                </FieldDescription>
              </Field>

              <Field>
                <FieldLabel htmlFor="default-jvm-args">
                  {m['settings.default_jvm_args']()}
                </FieldLabel>
                <Input
                  id="default-jvm-args"
                  className="font-mono"
                  key={entries.defaults?.['jvm-args'] ?? ''}
                  defaultValue={entries.defaults?.['jvm-args'] ?? ''}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') e.currentTarget.blur();
                  }}
                  onBlur={(e) => {
                    const value = e.target.value.trim();
                    if (value !== (entries.defaults?.['jvm-args'] ?? ''))
                      commitConfig('defaults.jvm-args', value);
                  }}
                />
              </Field>

              <Field>
                <FieldLabel>{m['settings.installed_runtimes']()}</FieldLabel>
                {runtimesQuery.isPending ? (
                  <div className="space-y-2">
                    <Bone className="h-10" />
                    <Bone className="h-10" />
                  </div>
                ) : (
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
                            {rt.releaseName}
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
                              disabled={rt.inUse || uninstall.isPending}
                            >
                              <TrashIcon className="size-4" />
                            </Button>
                          }
                          title={m['settings.uninstall_runtime_title']()}
                          description={m[
                            'settings.uninstall_runtime_description'
                          ]({
                            name: `${rt.vendor} ${rt.major}`,
                          })}
                          destructive
                          confirmLabel={m['action.uninstall']()}
                          onConfirm={() =>
                            uninstall.mutate(rt.major, {
                              onError: (error) => toast.error(error.message),
                            })
                          }
                        />
                      </div>
                    ))}
                  </div>
                )}
                <div className="mt-2 flex flex-wrap items-center gap-1.5">
                  <span className="mr-1 text-xs text-muted-foreground">
                    {m['settings.install_prompt']()}
                  </span>
                  {releases.map((r) => {
                    const installed = installedMajors.has(r.major);
                    return (
                      <Button
                        key={r.major}
                        variant="outline"
                        size="xs"
                        disabled={installed || install.isPending}
                        onClick={() =>
                          install.mutate(
                            { major: r.major },
                            {
                              onError: (error) => toast.error(error.message),
                            },
                          )
                        }
                      >
                        {r.major}
                        {r.lts ? ' LTS' : ''}
                        {installed ? ' ✓' : ''}
                      </Button>
                    );
                  })}
                </div>
              </Field>
            </FieldGroup>
          </FieldSet>

          <SyncSection />

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
      </div>
    </Page>
  );
}
