import type { ApplicationUninstallCandidate } from '@/lib/models/application';

export const UNINSTALL_CANCELLATION_TOAST_ID = 'application-uninstall-cancellation';

export function applicationBatchRequiresElevation(
  candidates: readonly Pick<ApplicationUninstallCandidate, 'capability'>[]
): boolean {
  return candidates.some(candidate => candidate.capability === 'requiresElevation');
}

export function shouldNotifyUninstallCancellation(wasOpen: boolean, nextOpen: boolean): boolean {
  return wasOpen && !nextOpen;
}
