import { z } from 'zod';
import type { Instance, Server } from '@/features/entries/mock';
import { memGb } from '@/lib/format';

type Kind = 'server' | 'instance';

/**
 * Zod schemas for the create wizard, one per step. Each `FormGroup`'s
 * `onDynamic` validator uses its step schema, so "Next" only gates on the
 * current step; the whole-form `onDynamic` composes them for the final submit.
 */

export const flavorStepSchema = z.object({
  flavor: z.string().min(1, 'Pick a flavor to continue.'),
});

export const versionStepSchema = z.object({
  version: z.string().min(1, 'Choose a version to continue.'),
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
      .regex(/^\d+$/, 'Enter a whole number.')
      .refine((v) => Number(v) >= 1, 'At least one player.'),
    port: z
      .string()
      .regex(/^\d*$/, 'Port must be a number.')
      .refine((v) => v === '' || Number(v) <= 65535, 'Port is out of range.'),
    pvp: z.boolean(),
    onlineMode: z.boolean(),
    eula:
      kind === 'server'
        ? z.literal(true, { error: 'Accept the EULA to create a server.' })
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
    name: z.string().min(1, 'A name is required.'),
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
    name: z.string().min(1, 'A name is required.'),
    memory: z.number().min(2).max(32),
    jvmArgs: jvmArgsField,
    backupInterval: z.string(),
    backupRetention: z
      .number()
      .int('Whole number of backups.')
      .min(1, 'Keep at least one.'),
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
