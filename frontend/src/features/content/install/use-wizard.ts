import { useEffect, useReducer } from 'react';

import type { ContentKind, ContentProject } from '@/api';
import { projectRef } from '@/features/content/components/content-card';
import type { PickedFile } from './targets';

export interface WizardInit {
  open: boolean;
  entryId: string;
  project?: ContentProject;
}

export interface WizardState {
  step: number;
  targetId: string;
  picked: ContentProject[];
  files: PickedFile[];
  kindFilter: ContentKind | null;
  versionIds: Record<string, string>;
  worlds: string[];
  installing: boolean;
  error: string;
}

export type WizardAction =
  | { type: 'reset'; init: WizardInit }
  | { type: 'step'; step: number }
  | { type: 'target'; id: string }
  | { type: 'toggleProject'; project: ContentProject }
  | { type: 'addFiles'; paths: string[]; kind: ContentKind }
  | { type: 'removeFile'; path: string }
  | { type: 'kindFilter'; kind: ContentKind | null }
  | { type: 'version'; ref: string; id: string }
  | { type: 'toggleWorld'; world: string; on: boolean }
  | { type: 'installStart' }
  | { type: 'installError'; message: string };

function initial(init: WizardInit): WizardState {
  return {
    step: 0,
    targetId: init.entryId,
    picked: init.project ? [init.project] : [],
    files: [],
    kindFilter: null,
    versionIds: {},
    worlds: [],
    installing: false,
    error: '',
  };
}

function reducer(state: WizardState, action: WizardAction): WizardState {
  switch (action.type) {
    case 'reset':
      return initial(action.init);
    case 'step':
      return { ...state, step: action.step };
    case 'target':
      return { ...state, targetId: action.id, worlds: [] };
    case 'toggleProject': {
      const ref = projectRef(action.project);
      const has = state.picked.some((p) => projectRef(p) === ref);
      const { [ref]: _dropped, ...versionIds } = state.versionIds;
      return {
        ...state,
        picked: has
          ? state.picked.filter((p) => projectRef(p) !== ref)
          : [...state.picked, action.project],
        versionIds,
      };
    }
    case 'addFiles':
      return {
        ...state,
        files: [
          ...state.files,
          ...action.paths
            .filter((path) => !state.files.some((f) => f.path === path))
            .map((path) => ({ path, kind: action.kind })),
        ],
      };
    case 'removeFile':
      return {
        ...state,
        files: state.files.filter((f) => f.path !== action.path),
      };
    case 'kindFilter':
      return { ...state, kindFilter: action.kind };
    case 'version': {
      const { [action.ref]: _dropped, ...rest } = state.versionIds;
      return {
        ...state,
        versionIds: action.id ? { ...rest, [action.ref]: action.id } : rest,
      };
    }
    case 'toggleWorld':
      return {
        ...state,
        worlds: action.on
          ? [...state.worlds, action.world]
          : state.worlds.filter((w) => w !== action.world),
      };
    case 'installStart':
      return { ...state, installing: true, error: '' };
    case 'installError':
      return { ...state, installing: false, error: action.message };
  }
}

/**
 * Owns the install modal's step/selection/progress state. Reset runs on the
 * open transition only, so a fixed `project`/`entry` seeds the first render and
 * later changes don't clobber an in-progress selection.
 */
export function useInstallWizard(init: WizardInit) {
  const [state, dispatch] = useReducer(reducer, init, initial);
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset only on open.
  useEffect(() => {
    if (init.open) dispatch({ type: 'reset', init });
  }, [init.open]);
  return [state, dispatch] as const;
}
