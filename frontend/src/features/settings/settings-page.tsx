import { BroomIcon, CoffeeIcon, TrashIcon } from '@phosphor-icons/react';
import { revalidateLogic } from '@tanstack/react-form';
import { useState } from 'react';

import { Page } from '@/components/page';
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
import { StatusDot } from '@/components/ui/status-dot';
import { javaReleases, javaRuntimes } from '@/features/settings/mock';
import { settingsDefaults, settingsSchema } from '@/features/settings/schema';
import { useAppForm } from '@/hooks/form';
import { daemon } from '@/lib/mock';

export function SettingsPage() {
  const [runtimes, setRuntimes] = useState(javaRuntimes);

  const form = useAppForm({
    defaultValues: settingsDefaults,
    validationLogic: revalidateLogic(),
    validators: { onDynamic: settingsSchema },
    onSubmit: async () => {},
  });

  return (
    <Page title="Settings" subtitle="Launcher, runtimes and daemon">
      <form
        className="max-w-2xl"
        onSubmit={(e) => {
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <FieldGroup>
          <FieldSet>
            <FieldLegend>General</FieldLegend>
            <FieldGroup>
              <form.AppField name="theme">
                {(field) => (
                  <field.SelectField
                    label="Theme"
                    options={[
                      { value: 'dark', label: 'Dark' },
                      { value: 'system', label: 'System' },
                    ]}
                    triggerClassName="w-full"
                  />
                )}
              </form.AppField>

              <form.AppField name="dataDir">
                {(field) => (
                  <field.TextField
                    label="Data directory"
                    description="Where instances and servers live."
                  />
                )}
              </form.AppField>

              <form.AppField name="startAtLogin">
                {(field) => (
                  <field.CheckboxField label="Start Hestia at login" />
                )}
              </form.AppField>

              <form.AppField name="keepOpen">
                {(field) => (
                  <field.CheckboxField label="Keep the launcher open while a game runs" />
                )}
              </form.AppField>
            </FieldGroup>
          </FieldSet>

          <FieldSet>
            <FieldLegend>Java & performance</FieldLegend>
            <FieldGroup>
              <form.AppField name="memory">
                {(field) => (
                  <field.SliderField
                    label="Default allocated memory"
                    formatValue={(v) => `${v} GB`}
                    sliderClassName="max-w-md"
                    min={2}
                    max={32}
                    step={1}
                    description="Instances and servers can override this individually."
                  />
                )}
              </form.AppField>

              <form.AppField name="jvmArgs">
                {(field) => (
                  <field.TextField
                    label="Default JVM arguments"
                    inputClassName="font-mono"
                  />
                )}
              </form.AppField>

              <Field>
                <FieldLabel>Installed runtimes</FieldLabel>
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
                      {rt.in_use && <Badge variant="secondary">In use</Badge>}
                      <ConfirmDialog
                        trigger={
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            aria-label="Uninstall runtime"
                            disabled={rt.in_use}
                          >
                            <TrashIcon className="size-4" />
                          </Button>
                        }
                        title="Uninstall runtime?"
                        description={
                          <>
                            <span className="font-medium text-foreground">
                              {rt.vendor} {rt.major}
                            </span>{' '}
                            will be removed. Entries that need it reinstall it
                            on the next start.
                          </>
                        }
                        destructive
                        confirmLabel="Uninstall"
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
                    Install:
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
            <FieldLegend>Storage & daemon</FieldLegend>
            <FieldGroup>
              <Field orientation="horizontal">
                <FieldLabel className="flex-1">
                  Download cache
                  <span className="font-mono text-muted-foreground">
                    1.8 GB
                  </span>
                </FieldLabel>
                <ConfirmDialog
                  trigger={
                    <Button
                      variant="outline"
                      size="sm"
                      data-icon="inline-start"
                    >
                      <BroomIcon />
                      Clear cache
                    </Button>
                  }
                  title="Clear download cache?"
                  description="Frees the cached downloads. Files are re-fetched from the network the next time they're needed."
                  confirmLabel="Clear cache"
                  onConfirm={() => {}}
                />
              </Field>

              <form.AppField name="shared">
                {(field) => (
                  <field.TextField
                    label="Shared config"
                    description="Files synced across entries."
                  />
                )}
              </form.AppField>

              <Field orientation="horizontal">
                <FieldLabel className="flex-1 gap-2 font-normal">
                  <StatusDot tone={daemon.connected ? 'on' : 'off'} />
                  Daemon {daemon.connected ? 'connected' : 'offline'}
                  <span className="font-mono text-muted-foreground">
                    v{daemon.version} · up {daemon.uptime}
                  </span>
                </FieldLabel>
                <ConfirmDialog
                  trigger={
                    <Button variant="outline" size="sm">
                      Restart daemon
                    </Button>
                  }
                  title="Restart the daemon?"
                  description="The launcher briefly disconnects while it restarts. Running servers and instances keep running."
                  confirmLabel="Restart"
                  onConfirm={() => {}}
                />
              </Field>
            </FieldGroup>
          </FieldSet>
        </FieldGroup>
      </form>
    </Page>
  );
}
