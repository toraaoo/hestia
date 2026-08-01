import { createContext, useContext } from 'react';
import type { Target } from '../lib';

/**
 * Where the wizard is installing to. Every step and row resolves its labels and
 * accepted kinds against it, so it is read where it is needed rather than
 * threaded through each step's props.
 */
export const TargetCtx = createContext<Target | null>(null);

export function useTarget(): Target | null {
  return useContext(TargetCtx);
}

export function useIsProfileTarget(): boolean {
  return useTarget()?.type === 'profile';
}
