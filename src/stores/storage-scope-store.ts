import { defineStore } from 'pinia';

import type { DiskInfo } from '@/lib/models/disk';
import { LOG_DOMAINS, LOG_EVENTS } from '@/lib/models/telemetry';
import type { StorageScopeId, StorageScopePreferences } from '@/lib/models/storage-scope';
import { FolderSelectionService } from '@/lib/services/folder-selection-service';
import { LoggerService } from '@/lib/services/logger-service';
import { PreferenceStorageService } from '@/lib/services/preference-storage-service';
import { PathUtils } from '@/lib/utils/path';
import { StorageScopePreferenceUtils } from '@/lib/utils/storage-scope-preference';

interface StorageScopeState extends StorageScopePreferences {
  initialized: boolean;
}

export const useStorageScopeStore = defineStore('storage-scope', {
  state: (): StorageScopeState => ({
    initialized: false,
    selectedPaths: {},
    recentFolders: [],
  }),
  actions: {
    async initialize(disks: readonly DiskInfo[]) {
      if (this.initialized) return;
      this.initialized = true;

      let saved: unknown | null;
      try {
        saved = await PreferenceStorageService.loadStorageScopePreferences();
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.storageScope, LOG_EVENTS.storageScopePreferencesLoadFailed, { error });
        return;
      }
      try {
        const preferences =
          saved === null ? StorageScopePreferenceUtils.empty() : StorageScopePreferenceUtils.parse(saved);
        this.selectedPaths = preferences.selectedPaths;
        this.recentFolders = preferences.recentFolders;
      } catch (error) {
        LoggerService.warn(LOG_DOMAINS.storageScope, LOG_EVENTS.storageScopePreferencesInvalid, { error });
        this.clearPersistedPreferences();
        return;
      }

      const diskKeys = new Set(disks.map(disk => PathUtils.comparisonKey(disk.mountPoint)));
      if (!this.recentFolders.length) return;
      try {
        const existingFolders = await FolderSelectionService.filterExistingDirectories(this.recentFolders);
        const existingKeys = new Set(existingFolders.map(PathUtils.comparisonKey));
        if (existingFolders.length === this.recentFolders.length) return;

        this.recentFolders = existingFolders;
        for (const [scopeId, selectedPath] of Object.entries(this.selectedPaths)) {
          const key = PathUtils.comparisonKey(selectedPath);
          if (!diskKeys.has(key) && !existingKeys.has(key)) {
            delete this.selectedPaths[scopeId as StorageScopeId];
          }
        }
        this.persist();
      } catch (error) {
        // Retain the saved history when validation is temporarily unavailable.
        LoggerService.warn(LOG_DOMAINS.storageScope, LOG_EVENTS.storageScopeValidationFailed, { error });
      }
    },
    selectedPath(scopeId: StorageScopeId): string {
      return this.selectedPaths[scopeId] ?? '';
    },
    select(scopeId: StorageScopeId, path: string, disks: readonly DiskInfo[]) {
      const normalized = PathUtils.display(path);
      if (!normalized) return;

      this.selectedPaths[scopeId] = normalized;
      const diskKeys = new Set(disks.map(disk => PathUtils.comparisonKey(disk.mountPoint)));
      if (!diskKeys.has(PathUtils.comparisonKey(normalized))) {
        this.recentFolders = StorageScopePreferenceUtils.addRecentFolder(this.recentFolders, normalized);
      }
      this.persist();
    },
    removeFolder(path: string) {
      const removedKey = PathUtils.comparisonKey(path);
      this.recentFolders = StorageScopePreferenceUtils.removePath(this.recentFolders, path);
      for (const [scopeId, selectedPath] of Object.entries(this.selectedPaths)) {
        if (PathUtils.comparisonKey(selectedPath) === removedKey) {
          delete this.selectedPaths[scopeId as StorageScopeId];
        }
      }
      this.persist();
    },
    persist() {
      void PreferenceStorageService.saveStorageScopePreferences({
        selectedPaths: this.selectedPaths,
        recentFolders: this.recentFolders,
      }).catch(error => {
        LoggerService.warn(LOG_DOMAINS.storageScope, LOG_EVENTS.storageScopePreferencesSaveFailed, { error });
      });
    },
    clearPersistedPreferences() {
      this.selectedPaths = {};
      this.recentFolders = [];
      void PreferenceStorageService.clearStorageScopePreferences().catch(error => {
        LoggerService.warn(LOG_DOMAINS.storageScope, LOG_EVENTS.storageScopePreferencesClearFailed, { error });
      });
    },
  },
});
