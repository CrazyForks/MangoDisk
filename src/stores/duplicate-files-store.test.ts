import { createPinia, setActivePinia } from 'pinia';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { DUPLICATE_GROUP_KINDS, type DuplicateFilesResult, type DuplicateGroup } from '@/lib/models/duplicate-file';
import { FILE_CATEGORY_IDS } from '@/lib/models/file-category';
import { DuplicateFileService } from '@/lib/services/duplicate-file-service';
import { LoggerService } from '@/lib/services/logger-service';

import { useAppStore } from './app-store';
import { useDuplicateFilesStore } from './duplicate-files-store';
import { useHistoryStore } from './history-store';

function createGroup(id: string, name: string): DuplicateGroup {
  return {
    id,
    hash: `hash-${id}`,
    kind: DUPLICATE_GROUP_KINDS.file,
    bytesPerFile: 1024,
    fileCountPerEntry: 1,
    reclaimableBytes: 1024,
    entries: [
      { name, path: `/one/${name}`, parentPath: '/one', bytes: 1024, modifiedAtMs: 1 },
      { name, path: `/two/${name}`, parentPath: '/two', bytes: 1024, modifiedAtMs: 2 },
    ],
  };
}

function createResult(groups: DuplicateGroup[]): DuplicateFilesResult {
  return {
    scanId: 7,
    roots: ['/fixture'],
    scannedAtMs: 1,
    scannedFileCount: 6,
    skippedCount: 0,
    duplicateFileCount: groups.length * 2,
    totalDuplicateBytes: groups.length * 2048,
    reclaimableBytes: groups.length * 1024,
    totalGroupCount: 3,
    returnedGroupCount: 3,
    truncated: false,
    groups,
  };
}

describe('duplicate files store pagination', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.restoreAllMocks();
  });

  it('skips unrelated pages until the selected category receives a group', async () => {
    const initialGroup = createGroup('initial', 'document.pdf');
    const archiveGroup = createGroup('archive', 'archive.zip');
    const audioGroup = createGroup('audio', 'recording.mp3');
    const page = vi
      .spyOn(DuplicateFileService, 'page')
      .mockResolvedValueOnce({ scanId: 7, offset: 1, nextOffset: 2, totalCount: 3, groups: [archiveGroup] })
      .mockResolvedValueOnce({ scanId: 7, offset: 2, nextOffset: null, totalCount: 3, groups: [audioGroup] });
    const store = useDuplicateFilesStore();
    store.result = createResult([initialGroup]);
    store.resultComplete = true;
    store.nextPageOffset = 1;

    await store.loadMore(FILE_CATEGORY_IDS.audio);

    expect(page).toHaveBeenCalledTimes(2);
    expect(store.result?.groups).toEqual([initialGroup, archiveGroup, audioGroup]);
    expect(store.nextPageOffset).toBeNull();
    expect(store.loadingMore).toBe(false);
  });

  it('loads only one page when every category is shown', async () => {
    const initialGroup = createGroup('initial', 'document.pdf');
    const archiveGroup = createGroup('archive', 'archive.zip');
    const page = vi.spyOn(DuplicateFileService, 'page').mockResolvedValue({
      scanId: 7,
      offset: 1,
      nextOffset: 2,
      totalCount: 3,
      groups: [archiveGroup],
    });
    const store = useDuplicateFilesStore();
    store.result = createResult([initialGroup]);
    store.resultComplete = true;
    store.nextPageOffset = 1;

    await store.loadMore(FILE_CATEGORY_IDS.all);

    expect(page).toHaveBeenCalledOnce();
    expect(store.result?.groups).toEqual([initialGroup, archiveGroup]);
    expect(store.nextPageOffset).toBe(2);
  });

  it('returns partial deletion results without raising a second global error', async () => {
    const group = createGroup('partial-delete', 'archive.zip');
    const removed = group.entries[0]!;
    const failed = group.entries[1]!;
    const operation = {
      removedPaths: [removed.path],
      failed: [{ path: failed.path, message: 'fixture item failure' }],
      releasedBytes: removed.bytes,
    };
    vi.spyOn(DuplicateFileService, 'deletePermanently').mockResolvedValue(operation);
    vi.spyOn(useHistoryStore(), 'load').mockResolvedValue();
    const warn = vi.spyOn(LoggerService, 'warn').mockImplementation(() => undefined);
    const appStore = useAppStore();
    const store = useDuplicateFilesStore();
    store.result = createResult([group]);
    store.resultComplete = true;

    const result = await store.deletePermanently([removed, failed]);

    expect(result).toEqual(operation);
    expect(appStore.errorCode).toBeNull();
    expect(warn).toHaveBeenCalledWith('duplicate-files', 'delete_completed_with_failures', {
      removedCount: 1,
      failedCount: 1,
      releasedBytes: removed.bytes,
    });
  });
});
