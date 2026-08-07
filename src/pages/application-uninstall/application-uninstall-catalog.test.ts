import { describe, expect, it } from 'vitest';

import type { ApplicationUninstallCandidate } from '@/lib/models/application';

import {
  applicationIsReady,
  applicationStatusKey,
  applicationSupportsUninstall,
  filterAndSortApplications,
  nextApplicationCatalogSort,
} from './application-uninstall-catalog';

function candidate(
  name: string,
  estimatedBytes: number,
  lastUsedAtMs: number | null,
  capability: ApplicationUninstallCandidate['capability'] = 'ready'
): ApplicationUninstallCandidate {
  return {
    applicationId: `application-${name}`,
    primaryIdentifier: `com.example.${name}`,
    name,
    version: null,
    publisher: 'Example',
    estimatedBytes,
    lastUsedAtMs,
    installedAtMs: null,
    platform: 'macosBundle',
    installerKind: null,
    executionMode: null,
    capability,
    recordState: 'installed',
    applicationPath: null,
    possibleRelatedPaths: [],
    iconPath: null,
    runningProcesses: [],
    totalBytes: estimatedBytes,
    defaultSelectedBytes: estimatedBytes,
    associatedDataComplete: true,
    components: [],
  };
}

describe('application uninstall catalog', () => {
  const applications = [
    candidate('Small', 10, 100),
    candidate('Large', 30, null),
    candidate('Medium', 20, 300, 'applicationRunning'),
    {
      ...candidate('Elevated', 25, 250, 'requiresElevation'),
      platform: 'windowsRegistry',
      installedAtMs: 250,
    },
    candidate('Unknown', 0, 200),
  ];

  it('sorts by size and uses the name as a stable tie breaker', () => {
    expect(filterAndSortApplications(applications, '', 'all', 'sizeDescending').map(item => item.name)).toEqual([
      'Large',
      'Elevated',
      'Medium',
      'Small',
      'Unknown',
    ]);
    expect(filterAndSortApplications(applications, '', 'all', 'sizeAscending').map(item => item.name)).toEqual([
      'Small',
      'Medium',
      'Elevated',
      'Large',
      'Unknown',
    ]);
  });

  it('treats unavailable platform dates as the earliest timestamp', () => {
    expect(filterAndSortApplications(applications, '', 'all', 'dateDescending').map(item => item.name)).toEqual([
      'Medium',
      'Elevated',
      'Unknown',
      'Small',
      'Large',
    ]);
    expect(filterAndSortApplications(applications, '', 'all', 'dateAscending').map(item => item.name)).toEqual([
      'Large',
      'Small',
      'Unknown',
      'Elevated',
      'Medium',
    ]);
  });

  it('sorts Windows applications by their install or update date', () => {
    const windowsApplications = [
      { ...candidate('Older', 10, 900), platform: 'windowsRegistry' as const, installedAtMs: 100 },
      { ...candidate('Newer', 20, 100), platform: 'windowsRegistry' as const, installedAtMs: 300 },
      { ...candidate('Unknown date', 30, 500), platform: 'windowsRegistry' as const },
    ];

    expect(filterAndSortApplications(windowsApplications, '', 'all', 'dateDescending').map(item => item.name)).toEqual([
      'Newer',
      'Older',
      'Unknown date',
    ]);
  });

  it('keeps elevated Windows uninstallers actionable and sorts by capability', () => {
    const elevated = applications.find(item => item.name === 'Elevated');
    expect(elevated && applicationSupportsUninstall(elevated)).toBe(true);
    expect(elevated && applicationIsReady(elevated)).toBe(false);
    expect(
      elevated &&
        applicationSupportsUninstall({
          ...elevated,
          platform: 'macosBundle',
        })
    ).toBe(false);
    expect(filterAndSortApplications(applications, '', 'ready', 'nameAscending').map(item => item.name)).toEqual([
      'Large',
      'Small',
      'Unknown',
    ]);
    expect(filterAndSortApplications(applications, '', 'all', 'statusAscending').map(item => item.name)).toEqual([
      'Large',
      'Small',
      'Unknown',
      'Elevated',
      'Medium',
    ]);
  });

  it('combines capability filtering and text search', () => {
    expect(
      filterAndSortApplications(applications, 'medium', 'running', 'nameAscending').map(item => item.name)
    ).toEqual(['Medium']);
    expect(filterAndSortApplications(applications, 'medium', 'ready', 'nameAscending')).toEqual([]);
  });

  it('keeps non-actionable entries in an explicit unavailable filter', () => {
    const unavailable = [
      ...applications,
      candidate('Administrator owned', 35, null, 'requiresElevation'),
      candidate('Protected', 40, null, 'protectedApplication'),
      candidate('View only', 50, null, 'viewOnly'),
    ];

    expect(
      filterAndSortApplications(unavailable, '', 'requiresElevation', 'nameAscending').map(item => item.name)
    ).toEqual(['Administrator owned', 'Elevated']);
    expect(filterAndSortApplications(unavailable, '', 'unavailable', 'nameAscending').map(item => item.name)).toEqual([
      'Protected',
      'View only',
    ]);
  });

  it('labels an orphaned Windows registration separately from a generic unavailable entry', () => {
    const unavailable = {
      ...candidate('Removed', 0, null, 'viewOnly'),
      platform: 'windowsRegistry' as const,
    };

    expect(applicationStatusKey(unavailable)).toBe('viewOnly');
    expect(
      applicationStatusKey({
        ...unavailable,
        recordState: 'orphanedRegistration',
        possibleRelatedPaths: ['C:\\Users\\fixture\\AppData\\Local\\com.example.removed'],
      })
    ).toBe('orphanedRegistration');
  });

  it('uses table-header sorting defaults and toggles the active column', () => {
    expect(nextApplicationCatalogSort('sizeDescending', 'name')).toBe('nameAscending');
    expect(nextApplicationCatalogSort('nameAscending', 'name')).toBe('nameDescending');
    expect(nextApplicationCatalogSort('nameDescending', 'date')).toBe('dateDescending');
    expect(nextApplicationCatalogSort('dateDescending', 'date')).toBe('dateAscending');
    expect(nextApplicationCatalogSort('dateAscending', 'status')).toBe('statusDescending');
  });
});
