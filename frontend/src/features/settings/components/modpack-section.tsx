import { useMutation } from '@tanstack/react-query';

import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLegend,
  FieldSet,
} from '@/components/ui/field';
import { CheckboxRow, TargetList } from '@/features/settings/components/fields';
import { m } from '@/paraglide/messages.js';
import { configMutations } from '@/queries/config';

/** The daemon stores each list as one delimited string, as docker-mc-server's
 *  own env vars do; the UI edits it as chips. */
const split = (raw: string | undefined) =>
  (raw ?? '')
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean);

export interface ModpackConfig {
  'default-excludes'?: boolean;
  'exclude-files'?: string;
  'force-include-files'?: string;
  'overrides-exclusions'?: string;
}

/**
 * Corrections over what a modpack claims about itself. Packs routinely declare
 * client-only mods as server-compatible, which is how a pack that plays as an
 * instance breaks as a server — so a shipped list holds them back, and these
 * are the ways to overrule it.
 */
export function ModpackSection({
  config,
  pending,
}: {
  config: ModpackConfig;
  pending: boolean;
}) {
  const setConfig = useMutation(configMutations.set());
  const busy = pending || setConfig.isPending;

  const commit = (key: string, values: string[]) =>
    setConfig.mutate({ key: `modpack.${key}`, value: values.join(', ') });

  const defaults = config['default-excludes'] ?? true;

  return (
    <FieldSet>
      <FieldLegend>{m['settings.modpack.title']()}</FieldLegend>
      <FieldGroup>
        <FieldDescription>{m['settings.modpack.hint']()}</FieldDescription>

        <Field>
          <CheckboxRow
            id="modpack-default-excludes"
            label={m['settings.modpack.default_excludes']()}
            checked={defaults}
            disabled={busy}
            onChange={(checked) =>
              setConfig.mutate({
                key: 'modpack.default-excludes',
                value: checked,
              })
            }
          />
          <FieldDescription>
            {m['settings.modpack.default_excludes_hint']()}
          </FieldDescription>
        </Field>

        <TargetList
          label={m['settings.modpack.force_include']()}
          placeholder={m['settings.modpack.mod_placeholder']()}
          values={split(config['force-include-files'])}
          pending={busy}
          onChange={(values) => commit('force-include-files', values)}
        />
        <TargetList
          label={m['settings.modpack.exclude']()}
          placeholder={m['settings.modpack.mod_placeholder']()}
          values={split(config['exclude-files'])}
          pending={busy}
          onChange={(values) => commit('exclude-files', values)}
        />
        <Field>
          <TargetList
            label={m['settings.modpack.overrides_exclusions']()}
            placeholder={m['settings.modpack.pattern_placeholder']()}
            values={split(config['overrides-exclusions'])}
            pending={busy}
            onChange={(values) => commit('overrides-exclusions', values)}
          />
          <FieldDescription>
            {m['settings.modpack.overrides_exclusions_hint']()}
          </FieldDescription>
        </Field>
      </FieldGroup>
    </FieldSet>
  );
}
