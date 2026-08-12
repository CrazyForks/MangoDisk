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

/**
 * 按平台路径规则查找标准资料夹。
 *
 * 标准资料夹的真实路径通常不会随应用语言变化，例如繁体中文界面中的“下载”
 * 在磁盘上仍可能是 `Downloads`。调用方应使用匹配结果中的稳定 ID 读取本地化文案，
 * 不应直接把路径末级名称展示给用户。
 */
export function findStandardScanFolderByPath(
  folders: readonly StandardScanFolder[],
  path: string
): StandardScanFolder | null {
  const pathKey = PathUtils.comparisonKey(path);
  return folders.find(folder => PathUtils.comparisonKey(folder.path) === pathKey) ?? null;
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
