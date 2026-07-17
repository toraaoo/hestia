import { z } from 'zod';

import { m } from '@/paraglide/messages.js';

/** Zod schema + defaults for the launcher settings form. */
export function settingsSchema() {
  return z.object({
    theme: z.string(),
    dataDir: z.string().min(1, m['error.data_dir_required']()),
    startAtLogin: z.boolean(),
    keepOpen: z.boolean(),
    memory: z.number().min(2).max(32),
    jvmArgs: z.string(),
    shared: z.string(),
  });
}

export const settingsDefaults = {
  theme: 'dark',
  dataDir: '~/.hestia',
  startAtLogin: true,
  keepOpen: true,
  memory: 6,
  jvmArgs: '-XX:+UseG1GC -XX:+ParallelRefProcEnabled',
  shared: 'options.txt, config/',
};
