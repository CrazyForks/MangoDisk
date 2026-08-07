import type { ApplicationUninstallCandidate } from '@/lib/models/application';

export type ApplicationCatalogFilter = 'all' | 'ready' | 'requiresElevation' | 'running' | 'unavailable';
export type ApplicationCatalogSortKey = 'name' | 'status' | 'size' | 'date';
export type ApplicationCatalogSort =
  | 'nameAscending'
  | 'nameDescending'
  | 'statusAscending'
  | 'statusDescending'
  | 'sizeAscending'
  | 'sizeDescending'
  | 'dateAscending'
  | 'dateDescending';

const SORT_KEY_BY_VALUE: Record<ApplicationCatalogSort, ApplicationCatalogSortKey> = {
  nameAscending: 'name',
  nameDescending: 'name',
  statusAscending: 'status',
  statusDescending: 'status',
  sizeAscending: 'size',
  sizeDescending: 'size',
  dateAscending: 'date',
  dateDescending: 'date',
};

export function applicationCatalogSortKey(sort: ApplicationCatalogSort): ApplicationCatalogSortKey {
  return SORT_KEY_BY_VALUE[sort];
}

export function applicationCatalogSortAscending(sort: ApplicationCatalogSort): boolean {
  return sort.endsWith('Ascending');
}

export function nextApplicationCatalogSort(
  current: ApplicationCatalogSort,
  key: ApplicationCatalogSortKey
): ApplicationCatalogSort {
  if (applicationCatalogSortKey(current) === key) {
    return applicationCatalogSortAscending(current) ? `${key}Descending` : `${key}Ascending`;
  }
  // Names start alphabetically; numeric and temporal columns start with the
  // largest or most recent value because those are the usual cleanup views.
  return key === 'name' ? 'nameAscending' : `${key}Descending`;
}

function compareName(left: ApplicationUninstallCandidate, right: ApplicationUninstallCandidate) {
  return left.name.localeCompare(right.name, undefined, {
    numeric: true,
    sensitivity: 'base',
  });
}

function compareOptionalTimestamp(left: number | null, right: number | null, direction: 1 | -1) {
  if (left === right) return 0;
  // An unavailable usage date means the application has no observable recent
  // use. Treat it as the earliest value: it appears first for oldest-first
  // review and remains last when users sort by most recently used.
  if (left === null) return -direction;
  if (right === null) return direction;
  return (left - right) * direction;
}

function applicationDate(candidate: ApplicationUninstallCandidate): number | null {
  return candidate.platform === 'windowsRegistry' ? candidate.installedAtMs : candidate.lastUsedAtMs;
}

function compareOptionalSize(left: number, right: number, direction: 1 | -1) {
  if (left === right) return 0;
  // Zero is the protocol's unknown-size sentinel. Keep unknown values after
  // measured applications instead of presenting them as the smallest apps.
  if (left === 0) return 1;
  if (right === 0) return -1;
  return (left - right) * direction;
}

const CAPABILITY_RANK: Record<ApplicationUninstallCandidate['capability'], number> = {
  ready: 0,
  requiresElevation: 1,
  applicationRunning: 2,
  viewOnly: 3,
  protectedApplication: 4,
};

export function applicationSupportsUninstall(candidate: ApplicationUninstallCandidate): boolean {
  return (
    candidate.capability === 'ready' ||
    (candidate.platform === 'windowsRegistry' && candidate.capability === 'requiresElevation')
  );
}

export function applicationIsReady(candidate: ApplicationUninstallCandidate): boolean {
  return candidate.capability === 'ready';
}

export function applicationStatusKey(candidate: ApplicationUninstallCandidate): string {
  if (candidate.recordState === 'orphanedRegistration') return 'orphanedRegistration';
  if (candidate.capability === 'applicationRunning') return 'applicationRunning';
  if (candidate.capability === 'requiresElevation') return 'requiresElevation';
  if (candidate.capability === 'ready') return 'readyForReview';
  return 'viewOnly';
}

export function filterAndSortApplications(
  candidates: readonly ApplicationUninstallCandidate[],
  query: string,
  filter: ApplicationCatalogFilter,
  sort: ApplicationCatalogSort
): ApplicationUninstallCandidate[] {
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const matches = candidates.filter(candidate => {
    if (filter === 'ready' && !applicationIsReady(candidate)) return false;
    if (filter === 'requiresElevation' && candidate.capability !== 'requiresElevation') return false;
    if (filter === 'running' && candidate.capability !== 'applicationRunning') return false;
    if (
      filter === 'unavailable' &&
      (applicationSupportsUninstall(candidate) ||
        candidate.capability === 'requiresElevation' ||
        candidate.capability === 'applicationRunning')
    ) {
      return false;
    }
    if (!normalizedQuery) return true;
    return [candidate.name, candidate.publisher, candidate.primaryIdentifier]
      .filter((value): value is string => Boolean(value))
      .some(value => value.toLocaleLowerCase().includes(normalizedQuery));
  });

  return matches.toSorted((left, right) => {
    let comparison = 0;
    switch (sort) {
      case 'nameAscending':
        return compareName(left, right);
      case 'nameDescending':
        return -compareName(left, right);
      case 'statusAscending':
        comparison = CAPABILITY_RANK[left.capability] - CAPABILITY_RANK[right.capability];
        break;
      case 'statusDescending':
        comparison = CAPABILITY_RANK[right.capability] - CAPABILITY_RANK[left.capability];
        break;
      case 'sizeAscending':
        comparison = compareOptionalSize(left.totalBytes, right.totalBytes, 1);
        break;
      case 'sizeDescending':
        comparison = compareOptionalSize(left.totalBytes, right.totalBytes, -1);
        break;
      case 'dateAscending':
        comparison = compareOptionalTimestamp(applicationDate(left), applicationDate(right), 1);
        break;
      case 'dateDescending':
        comparison = compareOptionalTimestamp(applicationDate(left), applicationDate(right), -1);
        break;
    }
    return comparison || compareName(left, right);
  });
}
