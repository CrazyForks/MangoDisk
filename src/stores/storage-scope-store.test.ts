import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { DiskInfo } from '@/lib/models/disk';
import { STORAGE_SCOPE_IDS } from '@/lib/models/storage-scope';
import { FolderSelectionService } from '@/lib/services/folder-selection-service';
import { PreferenceStorageService } from '@/lib/services/preference-storage-service';

import { useStorageScopeStore } from './storage-scope-store';

const { loadMock, values } = vi.hoisted(() => {
  const storedValues = new Map<string, unknown>();
  const store = {
    get: vi.fn(async (key: string) => storedValues.get(key)),
    set: vi.fn(async (key: string, value: unknown) => {
      storedValues.set(key, value);
    }),
    delete: vi.fn(async (key: string) => storedValues.delete(key)),
    save: vi.fn(async () => undefined),
  };
  return {
    loadMock: vi.fn(async () => store),
    values: storedValues,
  };
});

vi.mock('@tauri-apps/plugin-store', () => ({ load: loadMock }));
const disks: DiskInfo[] = [
  {
    name: 'System',
    mountPoint: '/',
    totalBytes: 1_000,
    availableBytes: 400,
    usedBytes: 600,
  },
];

describe('storage scope store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    values.clear();
    vi.restoreAllMocks();
  });

  it('persists separate page selections and shared recent folders', async () => {
    const store = useStorageScopeStore();

    store.select(STORAGE_SCOPE_IDS.analysis, '/Users/example/Downloads', disks);
    store.select(STORAGE_SCOPE_IDS.largeFiles, '/Users/example/Movies', disks);
    store.select(STORAGE_SCOPE_IDS.duplicateFiles, '/Users/example/Documents', disks);

    expect(store.selectedPath(STORAGE_SCOPE_IDS.analysis)).toBe('/Users/example/Downloads');
    expect(store.selectedPath(STORAGE_SCOPE_IDS.largeFiles)).toBe('/Users/example/Movies');
    expect(store.selectedPath(STORAGE_SCOPE_IDS.duplicateFiles)).toBe('/Users/example/Documents');
    expect(store.recentFolders).toEqual([
      '/Users/example/Documents',
      '/Users/example/Movies',
      '/Users/example/Downloads',
    ]);
    await expect(PreferenceStorageService.loadStorageScopePreferences()).resolves.toEqual({
      selectedPaths: {
        analysis: '/Users/example/Downloads',
        'large-files': '/Users/example/Movies',
        'duplicate-files': '/Users/example/Documents',
      },
      recentFolders: ['/Users/example/Documents', '/Users/example/Movies', '/Users/example/Downloads'],
    });
  });

  it('does not add disk roots to folder history', () => {
    const store = useStorageScopeStore();

    store.select(STORAGE_SCOPE_IDS.analysis, '/', disks);

    expect(store.selectedPath(STORAGE_SCOPE_IDS.analysis)).toBe('/');
    expect(store.recentFolders).toEqual([]);
  });

  it('removes a folder from history and every page selection', () => {
    const store = useStorageScopeStore();
    store.select(STORAGE_SCOPE_IDS.analysis, '/Users/example/Downloads', disks);
    store.select(STORAGE_SCOPE_IDS.largeFiles, '/Users/example/Downloads', disks);
    store.select(STORAGE_SCOPE_IDS.duplicateFiles, '/Users/example/Downloads', disks);

    store.removeFolder('/Users/example/Downloads');

    expect(store.selectedPaths).toEqual({});
    expect(store.recentFolders).toEqual([]);
  });

  it('drops missing folders while restoring saved preferences', async () => {
    await PreferenceStorageService.saveStorageScopePreferences({
      selectedPaths: {
        analysis: '/Users/example/Missing',
        'large-files': '/Users/example/Downloads',
      },
      recentFolders: ['/Users/example/Missing', '/Users/example/Downloads'],
    });
    vi.spyOn(FolderSelectionService, 'filterExistingDirectories').mockResolvedValue(['/Users/example/Downloads']);

    const store = useStorageScopeStore();
    await store.initialize(disks);

    expect(store.selectedPaths).toEqual({
      'large-files': '/Users/example/Downloads',
    });
    expect(store.recentFolders).toEqual(['/Users/example/Downloads']);
    await vi.waitFor(() => {
      expect(values.get('storageScopePreferences')).toMatchObject({
        selectedPaths: {
          'large-files': '/Users/example/Downloads',
        },
      });
    });
  });
});
