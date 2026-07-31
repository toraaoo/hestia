import { Field, FieldDescription, FieldLabel } from '@/components/ui/field';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { SavedInput, SwitchRow } from '@/features/settings/components/controls';
import {
  Setting,
  SettingsSection,
} from '@/features/settings/components/filtered';
import { UpdatePanel } from '@/features/settings/components/update';
import { useConfig } from '@/features/settings/use-config';
import { type Locale, useLocale } from '@/hooks/locale';
import { m } from '@/paraglide/messages.js';
import { locales } from '@/paraglide/runtime.js';
import { usePrefs } from '@/queries/prefs';
import {
  CLOSE_ACTION_KEY,
  CLOSE_ACTIONS,
  type CloseAction,
} from '@/queries/sessions';

/** Endonyms — a language always names itself, whatever locale is active. */
const LANGUAGE_NAMES: Record<string, string> = {
  en: 'English',
  'pt-BR': 'Português (Brasil)',
};

const CLOSE_ACTION_LABELS: Record<CloseAction, () => string> = {
  quit: m['settings.close_action.quit'],
  tray: m['settings.close_action.tray'],
  'stop-daemon': m['settings.close_action.stop_daemon'],
};

export function GeneralTab() {
  const { entries, commit, save } = useConfig();
  const prefs = usePrefs();

  return (
    <>
      <SettingsSection group="general" legend={m['settings.general']()}>
        <Setting id="language">
          <LanguageField />
        </Setting>

        <Setting id="data-dir">
          <Field>
            <FieldLabel htmlFor="data-dir">
              {m['settings.data_dir']()}
            </FieldLabel>
            <SavedInput
              id="data-dir"
              mono
              value={entries.home ?? ''}
              onSave={(value) => {
                if (value) save('home', value);
              }}
              confirm={{
                title: m['settings.data_dir_confirm_title'](),
                description: m['settings.data_dir_confirm_description'](),
              }}
            />
            <FieldDescription>{m['settings.data_dir_hint']()}</FieldDescription>
          </Field>
        </Setting>

        <Setting id="autostart">
          <SwitchRow
            id="start-at-login"
            label={m['settings.start_at_login']()}
            checked={entries.autostart ?? false}
            disabled={import.meta.env.DEV}
            onChange={(checked) => commit('autostart', checked)}
          />
        </Setting>

        <Setting id="keep-open">
          <SwitchRow
            id="keep-open"
            label={m['settings.keep_open']()}
            checked={prefs.get('keepOpen', true)}
            onChange={(checked) => prefs.set('keepOpen', checked)}
          />
        </Setting>

        <Setting id="close-action">
          <Field>
            <FieldLabel htmlFor="close-action">
              {m['settings.close_action.label']()}
            </FieldLabel>
            <Select
              value={prefs.get<CloseAction>(CLOSE_ACTION_KEY, 'quit')}
              onValueChange={(value) => {
                if (value) prefs.set(CLOSE_ACTION_KEY, value);
              }}
            >
              <SelectTrigger id="close-action" className="w-full max-w-md">
                <SelectValue>
                  {(value: string) =>
                    CLOSE_ACTION_LABELS[value as CloseAction]()
                  }
                </SelectValue>
              </SelectTrigger>
              <SelectContent align="start">
                <SelectGroup>
                  {CLOSE_ACTIONS.map((action) => (
                    <SelectItem key={action} value={action}>
                      {CLOSE_ACTION_LABELS[action]()}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            <FieldDescription>
              {m['settings.close_action.hint']()}
            </FieldDescription>
          </Field>
        </Setting>

        <Setting id="announcements">
          <SwitchRow
            id="announcements-enabled"
            label={m['settings.news.enabled_label']()}
            description={m['settings.news.enabled_description']()}
            checked={entries.announcements?.enabled ?? true}
            onChange={(checked) => commit('announcements.enabled', checked)}
          />
        </Setting>

        <Setting id="discord">
          <SwitchRow
            id="discord-enabled"
            label={m['settings.discord.enabled_label']()}
            description={m['settings.discord.enabled_description']()}
            checked={entries.discord?.enabled ?? true}
            onChange={(checked) => commit('discord.enabled', checked)}
          />
        </Setting>
      </SettingsSection>

      <SettingsSection group="updates" legend={m['settings.update.title']()}>
        <Setting id="update">
          <UpdatePanel />
        </Setting>
      </SettingsSection>
    </>
  );
}

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
        <SelectTrigger id="language" className="w-full max-w-md">
          <SelectValue>
            {(value: string) => LANGUAGE_NAMES[value] ?? value}
          </SelectValue>
        </SelectTrigger>
        <SelectContent align="start">
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
