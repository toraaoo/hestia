import { z } from 'zod';
import type { Instance, Server } from '@/features/entries/mock';
import { memGb } from '@/lib/format';
import { m } from '@/paraglide/messages.js';

type Kind = 'server' | 'instance';

/**
 * Zod schemas for the create wizard, one per step. Each `FormGroup`'s
 * `onDynamic` validator uses its step schema, so "Next" only gates on the
 * current step; the whole-form `onDynamic` composes them for the final submit.
 */

export const flavorStepSchema = z.object({
  flavor: z.string().min(1, m['error.pick_flavor']()),
});

export const versionStepSchema = z.object({
  version: z.string().min(1, m['error.choose_version']()),
  loaderVersion: z.string(),
});

export function detailsStepSchema(kind: Kind) {
  return z.object({
    name: z.string(),
    memory: z.number().min(2).max(32),
    motd: z.string(),
    gamemode: z.string(),
    difficulty: z.string(),
    maxPlayers: z
      .string()
      .regex(/^\d+$/, m['error.whole_number']())
      .refine((v) => Number(v) >= 1, m['error.min_players']()),
    port: z
      .string()
      .regex(/^\d*$/, m['error.port_number']())
      .refine((v) => v === '' || Number(v) <= 65535, m['error.port_range']()),
    pvp: z.boolean(),
    onlineMode: z.boolean(),
    eula:
      kind === 'server'
        ? z.literal(true, { error: m['error.eula']() })
        : z.boolean(),
  });
}

export function createWizardSchema(kind: Kind) {
  return z.object({
    flavor: flavorStepSchema,
    version: versionStepSchema,
    details: detailsStepSchema(kind),
  });
}

export function createWizardDefaults(loaderVersion: string) {
  return {
    flavor: { flavor: 'vanilla' },
    version: { version: '', loaderVersion },
    details: {
      name: '',
      memory: 4,
      motd: 'A Minecraft Server',
      gamemode: 'survival',
      difficulty: 'normal',
      maxPlayers: '20',
      port: '',
      pvp: true,
      onlineMode: true,
      eula: false,
    },
  };
}

/** Settings-form schemas — single step, but validated all the same. */

export const jvmArgsField = z.string();

export function instanceSettingsSchema() {
  return z.object({
    name: z.string().min(1, m['error.name_required']()),
    version: z.string().min(1),
    loader: z.string().min(1),
    memory: z.number().min(2).max(32),
    jvmArgs: jvmArgsField,
  });
}

export function instanceSettingsDefaults(inst: Instance) {
  return {
    name: inst.name,
    version: inst.game_version,
    loader: inst.flavor,
    memory: memGb(inst.memory),
    jvmArgs: '',
  };
}

export function serverSettingsSchema() {
  return z.object({
    name: z.string().min(1, m['error.name_required']()),
    memory: z.number().min(2).max(32),
    jvmArgs: jvmArgsField,
    backupInterval: z.string(),
    backupRetention: z
      .number()
      .int(m['error.backup_whole']())
      .min(1, m['error.backup_min']()),
  });
}

export function serverSettingsDefaults(server: Server) {
  return {
    name: server.name,
    memory: memGb(server.memory),
    jvmArgs: '',
    // The Select can't carry an empty value; 'off' is the sentinel for disabled.
    backupInterval: server.backup_interval || 'off',
    backupRetention: server.backup_retention,
  };
}
