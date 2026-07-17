import { z } from 'zod';

/** Zod schema + defaults for the launcher settings form. */
export const settingsSchema = z.object({
  theme: z.string(),
  dataDir: z.string().min(1, 'A data directory is required.'),
  startAtLogin: z.boolean(),
  keepOpen: z.boolean(),
  memory: z.number().min(2).max(32),
  jvmArgs: z.string(),
  shared: z.string(),
});

export const settingsDefaults = {
  theme: 'dark',
  dataDir: '~/.hestia',
  startAtLogin: true,
  keepOpen: true,
  memory: 6,
  jvmArgs: '-XX:+UseG1GC -XX:+ParallelRefProcEnabled',
  shared: 'options.txt, config/',
};
