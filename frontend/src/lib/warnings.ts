/**
 * Surfacing daemon warnings — the degraded outcomes an operation returns
 * alongside its success. One helper so every call site renders them the same
 * way: the localized headline as the toast, the remediation as its description.
 */
import { toast } from 'sonner';
import { type WarningInfo, warningHint, warningMessage } from '@/api';

export function toastWarnings(warnings: WarningInfo[] | undefined): void {
  for (const warning of warnings ?? []) {
    toast.warning(warningMessage(warning), {
      description: warningHint(warning),
    });
  }
}
