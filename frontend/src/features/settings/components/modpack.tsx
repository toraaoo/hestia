import { useState } from 'react';

import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@/components/ui/accordion';
import { Badge } from '@/components/ui/badge';
import { RowList, SwitchRow } from '@/features/settings/components/controls';
import { useFiltering } from '@/features/settings/components/filtered';
import type { ModpackConfig } from '@/features/settings/use-config';
import { m } from '@/paraglide/messages.js';

/** The daemon stores each list as one delimited string, as docker-mc-server's
 *  own env vars do; the UI edits it as rows. */
const split = (raw: string | undefined) =>
  (raw ?? '')
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean);

/**
 * Corrections over what a modpack claims about itself. Packs routinely declare
 * client-only mods as server-compatible, which is how a pack that plays as an
 * instance breaks as a server — so a shipped list holds them back. The overrules
 * for it stay folded away until someone meets a pack that needs one.
 */
export function ModpackSettings({
  config,
  pending,
  onCommit,
}: {
  config: ModpackConfig;
  pending: boolean;
  onCommit: (key: string, value: unknown) => void;
}) {
  const filtering = useFiltering();
  const [opened, setOpened] = useState(false);
  const open = opened || filtering;

  const commit = (key: string, values: string[]) =>
    onCommit(`modpack.${key}`, values.join(', '));

  const lists = {
    force: split(config['force-include-files']),
    exclude: split(config['exclude-files']),
    overrides: split(config['overrides-exclusions']),
  };
  const listed =
    lists.force.length + lists.exclude.length + lists.overrides.length;

  return (
    <>
      <SwitchRow
        id="modpack-default-excludes"
        label={m['settings.modpack.default_excludes']()}
        description={m['settings.modpack.default_excludes_hint']()}
        checked={config['default-excludes'] ?? true}
        disabled={pending}
        onChange={(checked) => onCommit('modpack.default-excludes', checked)}
      />

      <Accordion
        value={open ? ['corrections'] : []}
        onValueChange={(value) => setOpened((value as string[]).length > 0)}
        className="border-t border-border"
      >
        <AccordionItem value="corrections">
          <AccordionTrigger>
            <span className="flex items-center gap-2">
              {m['settings.modpack.custom']()}
              {listed > 0 && <Badge variant="secondary">{listed}</Badge>}
            </span>
          </AccordionTrigger>
          <AccordionContent className="flex flex-col gap-5 pt-1">
            <RowList
              label={m['settings.modpack.force_include']()}
              placeholder={m['settings.modpack.mod_placeholder']()}
              values={lists.force}
              pending={pending}
              onChange={(values) => commit('force-include-files', values)}
            />
            <RowList
              label={m['settings.modpack.exclude']()}
              placeholder={m['settings.modpack.mod_placeholder']()}
              values={lists.exclude}
              pending={pending}
              onChange={(values) => commit('exclude-files', values)}
            />
            <RowList
              label={m['settings.modpack.overrides_exclusions']()}
              description={m['settings.modpack.overrides_exclusions_hint']()}
              placeholder={m['settings.modpack.pattern_placeholder']()}
              values={lists.overrides}
              pending={pending}
              onChange={(values) => commit('overrides-exclusions', values)}
            />
          </AccordionContent>
        </AccordionItem>
      </Accordion>
    </>
  );
}
