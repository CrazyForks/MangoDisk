import { audioDir, documentDir, downloadDir, pictureDir, videoDir } from '@tauri-apps/api/path';

import { FolderSelectionService } from '@/lib/services/folder-selection-service';
import { PathUtils } from '@/lib/utils/path';

export const STANDARD_SCAN_FOLDER_IDS = {
  downloads: 'downloads',
  documents: 'documents',
  pictures: 'pictures',
  videos: 'videos',
  music: 'music',
} as const;

export type StandardScanFolderId = (typeof STANDARD_SCAN_FOLDER_IDS)[keyof typeof STANDARD_SCAN_FOLDER_IDS];

export interface StandardScanFolder {
  id: StandardScanFolderId;
  path: string;
}

interface StandardScanFolderDefinition {
  id: StandardScanFolderId;
  resolvePath: () => Promise<string>;
}

const standardFolderDefinitions: readonly StandardScanFolderDefinition[] = [
  { id: STANDARD_SCAN_FOLDER_IDS.downloads, resolvePath: downloadDir },
  { id: STANDARD_SCAN_FOLDER_IDS.documents, resolvePath: documentDir },
  { id: STANDARD_SCAN_FOLDER_IDS.pictures, resolvePath: pictureDir },
  { id: STANDARD_SCAN_FOLDER_IDS.videos, resolvePath: videoDir },
  { id: STANDARD_SCAN_FOLDER_IDS.music, resolvePath: audioDir },
];

/** Resolves operating-system user folders without assuming localized names or home paths. */
export class StandardScanFolderService {
  static async listAvailable(): Promise<StandardScanFolder[]> {
    const resolved = await Promise.allSettled(
      standardFolderDefinitions.map(async definition => ({
        id: definition.id,
        path: PathUtils.display(await definition.resolvePath()),
      }))
    );
    const candidates = resolved.flatMap(result => (result.status === 'fulfilled' ? [result.value] : []));
    if (candidates.length === 0) return [];

    const existing = await FolderSelectionService.filterExistingDirectories(
      candidates.map(candidate => candidate.path)
    );
    const existingKeys = new Set(existing.map(PathUtils.comparisonKey));
    return candidates.filter(candidate => existingKeys.has(PathUtils.comparisonKey(candidate.path)));
  }
}
