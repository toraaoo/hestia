import { CaretLeftIcon, CaretRightIcon } from '@phosphor-icons/react';
import { useMutation } from '@tanstack/react-query';

import type { ContentKind, ContentProject } from '@/api';
import { contentIcon, entryIcon } from '@/components/icons';
import { StepDots } from '@/components/step-dots';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Progress,
  ProgressLabel,
  ProgressValue,
} from '@/components/ui/progress';
import { projectRef } from '@/features/content/components/content-card';
import { m } from '@/paraglide/messages.js';
import { instanceMutations } from '@/queries/instance';
import { useJobMutation } from '@/queries/jobs';
import { profileMutations } from '@/queries/profile';
import { serverMutations } from '@/queries/server';

import { ContentStep } from './steps/content';
import { ReviewStep } from './steps/review';
import { TargetStep } from './steps/target';
import { WorldsStep } from './steps/worlds';
import { type Target, targetTakesKind, useTargets } from './targets';
import { useInstallWizard } from './use-wizard';

/**
 * The content install modal over the daemon's `content.add` (and, for a global
 * profile, `profile.edit`). It opens either way round: from Browse a `project`
 * is fixed and the user picks a target; from an entry's page the `entry` is
 * fixed and the user picks a project. The newest compatible version resolves
 * automatically (changeable in the review), a datapack on an instance chooses
 * its worlds, and the install runs as a job with live progress.
 */
export function ContentInstallModal({
  project,
  entry,
  open,
  onOpenChange,
}: {
  project?: ContentProject;
  entry?: Target;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const mode: 'browse' | 'entry' = project ? 'browse' : 'entry';
  const targets = useTargets();

  const [state, dispatch] = useInstallWizard({
    open,
    entryId: entry?.id ?? '',
    project,
  });
  const {
    step,
    targetId,
    picked,
    files,
    kindFilter,
    versionIds,
    worlds,
    installing,
    error,
  } = state;

  const addServer = useJobMutation(serverMutations.content.add(targetId));
  const addInstance = useJobMutation(instanceMutations.content.add(targetId));
  const editProfile = useMutation(profileMutations.edit());

  const target = targets.find((t) => t.id === targetId) ?? entry ?? null;
  const selectedCount = picked.length + files.length;
  const fileKinds = files
    .map((f) => f.kind)
    .filter((k): k is ContentKind => k !== null);
  const selectedKinds = [
    ...new Set([...picked.map((p) => p.kind), ...fileKinds]),
  ];
  // A staged file blocks install until it is installable and has a kind.
  const filesReady = files.every((f) => f.valid && f.kind !== null);
  const needsWorlds =
    selectedKinds.includes('data_pack') && target?.type === 'instance';
  const isProfile = target?.type === 'profile';

  const toggleProject = (p: ContentProject) =>
    dispatch({ type: 'toggleProject', project: p });

  const pickStep = mode === 'browse' ? 'target' : 'content';
  const steps: string[] =
    needsWorlds && !isProfile
      ? [pickStep, 'worlds', 'review']
      : [pickStep, 'review'];
  const stepId = steps[Math.min(step, steps.length - 1)];

  const Icon =
    mode === 'browse' && project
      ? contentIcon(project.kind)
      : entryIcon(entry?.type ?? 'instance');
  const title =
    mode === 'browse'
      ? m['content.install_title']({ name: project?.title ?? '' })
      : m['content.add_to_title']({ name: entry?.name ?? '' });

  const hint = installing
    ? m['content.installing']()
    : stepId === 'target'
      ? m['content.hint_target']()
      : stepId === 'content'
        ? m['content.hint_content']()
        : stepId === 'worlds'
          ? m['content.hint_worlds']()
          : m['content.hint_review']();

  const canAdvance =
    stepId === 'target'
      ? !!targetId
      : stepId === 'content'
        ? selectedCount > 0
        : stepId === 'worlds'
          ? worlds.length > 0
          : selectedCount > 0 && filesReady;

  const liveProgress =
    (target?.type === 'server' ? addServer : addInstance).progress ?? null;
  const progressPct =
    liveProgress && liveProgress.total > 0
      ? Math.round((liveProgress.current / liveProgress.total) * 100)
      : 0;
  const progressPhase = liveProgress?.detail || liveProgress?.phase || '';

  async function install() {
    if (!target) return;
    // A profile takes only projects; guard the no-op (e.g. only files staged)
    // so it never closes as if it installed.
    if (isProfile && picked.length === 0) {
      dispatch({
        type: 'installError',
        message: m['content.profile_no_projects'](),
      });
      return;
    }
    dispatch({ type: 'installStart' });
    try {
      if (isProfile) {
        await editProfile.mutateAsync({
          name: target.name,
          add: picked.map(projectRef),
        });
      } else {
        const add = target.type === 'server' ? addServer : addInstance;
        // The wire spec is per-kind, so a mixed selection installs as one
        // batch per kind; failures aggregate across batches.
        const failures: string[] = [];
        for (const k of selectedKinds) {
          const items = [
            ...picked
              .filter((p) => p.kind === k)
              .map((p) => ({
                project: projectRef(p),
                version: versionIds[projectRef(p)] ?? '',
              })),
            ...files.filter((f) => f.kind === k).map((f) => ({ path: f.path })),
          ];
          const done = await add.mutateAsync({
            kind: k,
            items,
            worlds: k === 'data_pack' && needsWorlds ? worlds : [],
          });
          failures.push(...done.failures.map((f) => f.message));
        }
        if (failures.length > 0) {
          dispatch({ type: 'installError', message: failures.join('; ') });
          return;
        }
      }
      onOpenChange(false);
    } catch (e) {
      dispatch({
        type: 'installError',
        message: e instanceof Error ? e.message : String(e),
      });
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!installing) onOpenChange(o);
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Icon className="size-4.5 text-muted-foreground" />
            {title}
          </DialogTitle>
          <DialogDescription>{hint}</DialogDescription>
        </DialogHeader>

        {installing ? (
          <div className="flex min-h-72 flex-col justify-center px-1">
            <Progress value={progressPct}>
              <ProgressLabel>
                {progressPhase || m['content.installing']()}
              </ProgressLabel>
              <ProgressValue />
            </Progress>
          </div>
        ) : (
          <>
            <div className="flex max-h-[58vh] min-h-64 flex-col overflow-hidden p-1">
              {stepId === 'target' ? (
                <TargetStep
                  kind={project?.kind ?? 'mod'}
                  targets={targets.filter(
                    (t) => project && targetTakesKind(t, project.kind),
                  )}
                  selectedId={targetId}
                  onSelect={(t) => dispatch({ type: 'target', id: t.id })}
                />
              ) : stepId === 'content' ? (
                target && (
                  <ContentStep
                    target={target}
                    kind={kindFilter}
                    onKindChange={(kind) =>
                      dispatch({ type: 'kindFilter', kind })
                    }
                    picked={picked}
                    onToggle={toggleProject}
                    onAddFiles={(files) =>
                      dispatch({ type: 'addFiles', files })
                    }
                  />
                )
              ) : stepId === 'worlds' ? (
                <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto">
                  <WorldsStep
                    instanceId={target?.id ?? ''}
                    selected={worlds}
                    onToggle={(world, on) =>
                      dispatch({ type: 'toggleWorld', world, on })
                    }
                  />
                </div>
              ) : (
                <div className="min-h-0 flex-1 overflow-x-hidden overflow-y-auto">
                  <ReviewStep
                    target={target}
                    picked={picked}
                    files={files}
                    versionIds={versionIds}
                    onVersion={(ref, id) =>
                      dispatch({ type: 'version', ref, id })
                    }
                    onRemoveProject={toggleProject}
                    onRemoveFile={(path) =>
                      dispatch({ type: 'removeFile', path })
                    }
                    onSetFileKind={(path, kind) =>
                      dispatch({ type: 'setFileKind', path, kind })
                    }
                    worlds={needsWorlds ? worlds : undefined}
                  />
                </div>
              )}
              {error && (
                <p className="mt-3 shrink-0 px-1 text-xs text-destructive">
                  {error}
                </p>
              )}
            </div>

            <DialogFooter className="items-center">
              <div className="mr-auto flex items-center gap-3">
                <StepDots
                  steps={steps}
                  active={Math.min(step, steps.length - 1)}
                />
                {mode === 'entry' && selectedCount > 0 && (
                  <span className="text-[11px] text-muted-foreground">
                    {m['content.selected_count']({ count: selectedCount })}
                  </span>
                )}
              </div>
              {step === 0 ? (
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                >
                  {m['action.cancel']()}
                </Button>
              ) : (
                <Button
                  type="button"
                  variant="outline"
                  data-icon="inline-start"
                  onClick={() =>
                    dispatch({ type: 'step', step: Math.max(0, step - 1) })
                  }
                >
                  <CaretLeftIcon />
                  {m['action.back']()}
                </Button>
              )}
              {stepId === 'review' ? (
                <Button
                  type="button"
                  disabled={!canAdvance}
                  className="bg-ember text-ember-foreground hover:bg-ember/90"
                  onClick={install}
                >
                  {m['action.install']()}
                </Button>
              ) : (
                <Button
                  type="button"
                  data-icon="inline-end"
                  disabled={!canAdvance}
                  className="bg-ember text-ember-foreground hover:bg-ember/90"
                  onClick={() => dispatch({ type: 'step', step: step + 1 })}
                >
                  {m['action.next']()}
                  <CaretRightIcon />
                </Button>
              )}
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
