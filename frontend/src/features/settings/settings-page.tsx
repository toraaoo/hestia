import { BroomIcon, CoffeeIcon, TrashIcon } from '@phosphor-icons/react';
import { useForm } from '@tanstack/react-form';

import { Page } from '@/components/page';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldLegend,
  FieldSet,
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
import { StatusDot } from '@/components/ui/status-dot';
import { javaReleases, javaRuntimes } from '@/features/settings/mock';
import { daemon } from '@/lib/mock';

export function SettingsPage() {
  const form = useForm({
    defaultValues: {
      theme: 'dark',
      dataDir: '~/.hestia',
      startAtLogin: true,
      keepOpen: true,
      memory: 6,
      jvmArgs: '-XX:+UseG1GC -XX:+ParallelRefProcEnabled',
      shared: 'options.txt, config/',
    },
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
              <form.Field name="theme">
                {(field) => (
                  <Field>
                    <FieldLabel htmlFor={field.name}>Theme</FieldLabel>
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
                          <SelectItem value="dark">Dark</SelectItem>
                          <SelectItem value="system">System</SelectItem>
                        </SelectGroup>
                      </SelectContent>
                    </Select>
                  </Field>
                )}
              </form.Field>

              <form.Field name="dataDir">
                {(field) => (
                  <Field>
                    <FieldLabel htmlFor={field.name}>Data directory</FieldLabel>
                    <Input
                      id={field.name}
                      value={field.state.value}
                      onBlur={field.handleBlur}
                      onChange={(e) => field.handleChange(e.target.value)}
                    />
                    <FieldDescription>
                      Where instances and servers live.
                    </FieldDescription>
                  </Field>
                )}
              </form.Field>

              <form.Field name="startAtLogin">
                {(field) => (
                  <Field orientation="horizontal">
                    <Checkbox
                      id={field.name}
                      checked={field.state.value}
                      onCheckedChange={(c) => field.handleChange(c === true)}
                    />
                    <FieldLabel htmlFor={field.name} className="font-normal">
                      Start Hestia at login
                    </FieldLabel>
                  </Field>
                )}
              </form.Field>

              <form.Field name="keepOpen">
                {(field) => (
                  <Field orientation="horizontal">
                    <Checkbox
                      id={field.name}
                      checked={field.state.value}
                      onCheckedChange={(c) => field.handleChange(c === true)}
                    />
                    <FieldLabel htmlFor={field.name} className="font-normal">
                      Keep the launcher open while a game runs
                    </FieldLabel>
                  </Field>
                )}
              </form.Field>
            </FieldGroup>
          </FieldSet>

          <FieldSet>
            <FieldLegend>Java & performance</FieldLegend>
            <FieldGroup>
              <form.Field name="memory">
                {(field) => (
                  <Field>
                    <FieldLabel htmlFor={field.name}>
                      Default allocated memory — {field.state.value} GB
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
                    <FieldDescription>
                      Instances and servers can override this individually.
                    </FieldDescription>
                  </Field>
                )}
              </form.Field>

              <form.Field name="jvmArgs">
                {(field) => (
                  <Field>
                    <FieldLabel htmlFor={field.name}>
                      Default JVM arguments
                    </FieldLabel>
                    <Input
                      id={field.name}
                      className="font-mono"
                      value={field.state.value}
                      onBlur={field.handleBlur}
                      onChange={(e) => field.handleChange(e.target.value)}
                    />
                  </Field>
                )}
              </form.Field>

              <Field>
                <FieldLabel>Installed runtimes</FieldLabel>
                <div className="divide-y divide-border border border-border">
                  {javaRuntimes.map((rt) => (
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
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label="Uninstall runtime"
                        disabled={rt.in_use}
                      >
                        <TrashIcon className="size-4" />
                      </Button>
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
                <Button variant="outline" size="sm" data-icon="inline-start">
                  <BroomIcon />
                  Clear cache
                </Button>
              </Field>

              <form.Field name="shared">
                {(field) => (
                  <Field>
                    <FieldLabel htmlFor={field.name}>Shared config</FieldLabel>
                    <Input
                      id={field.name}
                      value={field.state.value}
                      onBlur={field.handleBlur}
                      onChange={(e) => field.handleChange(e.target.value)}
                    />
                    <FieldDescription>
                      Files synced across entries.
                    </FieldDescription>
                  </Field>
                )}
              </form.Field>

              <Field orientation="horizontal">
                <FieldLabel className="flex-1 gap-2 font-normal">
                  <StatusDot tone={daemon.connected ? 'on' : 'off'} />
                  Daemon {daemon.connected ? 'connected' : 'offline'}
                  <span className="font-mono text-muted-foreground">
                    v{daemon.version} · up {daemon.uptime}
                  </span>
                </FieldLabel>
                <Button variant="outline" size="sm">
                  Restart daemon
                </Button>
              </Field>
            </FieldGroup>
          </FieldSet>
        </FieldGroup>
      </form>
    </Page>
  );
}
