import { z } from 'zod';

type Mode = 'browse' | 'entry';

/**
 * Zod schemas for the content install wizard, one per step. `browse` fixes the
 * project and gates on picking a target; `entry` fixes the target and gates on
 * picking a project. Selections are stored in the form as ids and looked up.
 */

export function pickStepSchema(mode: Mode) {
  return mode === 'browse'
    ? z.object({
        targetId: z.string().min(1, 'Choose where to install this.'),
        projectId: z.string(),
      })
    : z.object({
        projectId: z.string().min(1, 'Choose content to install.'),
        targetId: z.string(),
      });
}

export const worldsStepSchema = z.object({
  worlds: z.array(z.string()).min(1, 'Pick at least one world.'),
});

export const reviewStepSchema = z.object({
  versionId: z.string(),
});

export function installWizardSchema(mode: Mode) {
  return z.object({
    pick: pickStepSchema(mode),
    worlds: z.object({ worlds: z.array(z.string()) }),
    review: reviewStepSchema,
  });
}

export function installWizardDefaults(opts: {
  projectId: string;
  targetId: string;
  versionId: string;
}) {
  return {
    pick: { projectId: opts.projectId, targetId: opts.targetId },
    worlds: { worlds: [] as string[] },
    review: { versionId: opts.versionId },
  };
}
