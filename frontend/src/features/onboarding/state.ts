/**
 * What the user has already been shown. Every onboarding surface reads this
 * one record, so starting over is a single write.
 */
import { usePrefs } from '@/queries/prefs';

import type { TourId } from './tour/registry';

export const ONBOARDING_KEY = 'onboarding';

export interface OnboardingState {
  welcomed: boolean;
  seen: TourId[];
  /** Opted out: nothing starts on its own, replay buttons still work. */
  toursDisabled: boolean;
  checklistDismissed: boolean;
}

export const FRESH: OnboardingState = {
  welcomed: false,
  seen: [],
  toursDisabled: false,
  checklistDismissed: false,
};

// The prefs file is hand-editable, so an unrecognised record degrades to
// "nothing seen yet" rather than throwing inside a render.
function normalize(value: unknown): OnboardingState {
  if (typeof value !== 'object' || value === null) return FRESH;
  const raw = value as Partial<Record<keyof OnboardingState, unknown>>;
  return {
    welcomed: raw.welcomed === true,
    seen: Array.isArray(raw.seen)
      ? raw.seen.filter((id): id is TourId => typeof id === 'string')
      : [],
    toursDisabled: raw.toursDisabled === true,
    checklistDismissed: raw.checklistDismissed === true,
  };
}

export interface Onboarding extends OnboardingState {
  /** False until prefs have been read; nothing may auto-fire before. */
  ready: boolean;
  update: (patch: Partial<OnboardingState>) => void;
  reset: () => void;
}

export function useOnboarding(): Onboarding {
  const prefs = usePrefs();
  const state = normalize(prefs.get<unknown>(ONBOARDING_KEY, null));

  return {
    ...state,
    ready: prefs.ready,
    update: (patch) => prefs.set(ONBOARDING_KEY, { ...state, ...patch }),
    reset: () => prefs.set(ONBOARDING_KEY, FRESH),
  };
}
