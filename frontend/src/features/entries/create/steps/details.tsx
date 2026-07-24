import { Checkbox } from '@/components/ui/checkbox';
import { FieldError } from '@/components/ui/field';
import { m } from '@/paraglide/messages.js';

import {
  DIFFICULTIES,
  GAMEMODES,
  type Kind,
  options,
  PropToggle,
  SectionHeader,
  type WizardForm,
} from '../fields';

/** The details step: name + memory, plus the server.properties block. */
export function DetailsStep({ form, kind }: { form: WizardForm; kind: Kind }) {
  return (
    <div className="flex flex-col gap-4">
      <form.Subscribe
        selector={(s: WizardForm) =>
          [s.values.flavor.flavor, s.values.version.version] as const
        }
      >
        {([flavor, version]: [string, string]) => (
          <form.AppField name="details.name">
            {(field: WizardForm) => (
              <field.TextField
                label={m['label.name']()}
                placeholder={`${flavor}-${version}`}
                description={m['wizard.name_hint']()}
              />
            )}
          </form.AppField>
        )}
      </form.Subscribe>

      <form.AppField name="details.memory">
        {(field: WizardForm) => (
          <field.SliderField
            label={m['label.memory']()}
            formatValue={(v: number) => m['wizard.gb']({ value: v })}
            min={2}
            max={32}
            step={1}
          />
        )}
      </form.AppField>

      {kind === 'server' && (
        <>
          <SectionHeader>{m['wizard.server_properties']()}</SectionHeader>

          <form.AppField name="details.motd">
            {(field: WizardForm) => (
              <field.TextField label={m['wizard.motd']()} />
            )}
          </form.AppField>

          {/* Hardcore forces difficulty=hard and gamemode=survival, so both
              selects are pinned and disabled while it is on — matching the
              daemon's invariant and Minecraft's own create-world screen. */}
          <form.Subscribe
            selector={(s: WizardForm) => s.values.details.hardcore as boolean}
          >
            {(hardcore: boolean) => (
              <div className="grid gap-4 sm:grid-cols-2">
                <form.AppField name="details.gamemode">
                  {(field: WizardForm) => (
                    <field.SelectField
                      label={m['wizard.gamemode']()}
                      options={options(GAMEMODES)}
                      triggerClassName="w-full"
                      disabled={hardcore}
                    />
                  )}
                </form.AppField>
                <form.AppField name="details.difficulty">
                  {(field: WizardForm) => (
                    <field.SelectField
                      label={m['wizard.difficulty']()}
                      options={options(DIFFICULTIES)}
                      triggerClassName="w-full"
                      disabled={hardcore}
                    />
                  )}
                </form.AppField>
              </div>
            )}
          </form.Subscribe>

          <div className="grid gap-4 sm:grid-cols-2">
            <form.AppField name="details.maxPlayers">
              {(field: WizardForm) => (
                <field.TextField
                  label={m['wizard.max_players']()}
                  type="number"
                />
              )}
            </form.AppField>
            <form.AppField name="details.port">
              {(field: WizardForm) => (
                <field.TextField
                  label={m['wizard.port']()}
                  type="number"
                  placeholder={m['wizard.port_auto']()}
                  description={m['wizard.port_hint']()}
                />
              )}
            </form.AppField>
          </div>

          <div className="grid grid-cols-2 gap-4 pt-1">
            <form.AppField name="details.hardcore">
              {(field: WizardForm) => (
                <PropToggle
                  id="prop-hardcore"
                  label={m['wizard.hardcore']()}
                  checked={field.state.value}
                  onChange={(checked) => {
                    field.handleChange(checked);
                    if (checked) {
                      form.setFieldValue('details.gamemode', 'survival');
                      form.setFieldValue('details.difficulty', 'hard');
                    }
                  }}
                />
              )}
            </form.AppField>
            <form.AppField name="details.onlineMode">
              {(field: WizardForm) => (
                <PropToggle
                  id="prop-online"
                  label={m['wizard.online_mode']()}
                  checked={field.state.value}
                  onChange={field.handleChange}
                />
              )}
            </form.AppField>
          </div>

          <form.AppField name="details.eula">
            {(field: WizardForm) => {
              const invalid =
                field.state.meta.isTouched &&
                field.state.meta.errors.length > 0;
              return (
                <div className="flex flex-col gap-1.5">
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
                      {m['wizard.eula_before']()}{' '}
                      <a
                        href="https://aka.ms/MinecraftEULA"
                        target="_blank"
                        rel="noreferrer"
                        className="text-foreground underline underline-offset-2"
                      >
                        {m['wizard.eula_link']()}
                      </a>
                      {m['wizard.eula_after']()}
                    </span>
                  </label>
                  {invalid && (
                    <FieldError
                      errors={
                        field.state.meta.errors as Array<{ message?: string }>
                      }
                    />
                  )}
                </div>
              );
            }}
          </form.AppField>
        </>
      )}
    </div>
  );
}
