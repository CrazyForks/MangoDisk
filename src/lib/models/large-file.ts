export const LARGE_FILE_SORT_KEYS = {
  name: 'name',
  bytes: 'bytes',
  modified: 'modified',
} as const;

export const LARGE_FILE_MINIMUM_OPTIONS = [
  50 * 1024 * 1024,
  100 * 1024 * 1024,
  500 * 1024 * 1024,
  1024 * 1024 * 1024,
  5 * 1024 * 1024 * 1024,
] as const;

export const DEFAULT_LARGE_FILE_MINIMUM_BYTES = 100 * 1024 * 1024;
export const LARGE_FILE_RENDER_BATCH_SIZE = 80;

export interface LargeFileEntry {
  name: string;
  path: string;
  parentPath: string;
  bytes: number;
  modifiedAtMs: number | null;
}

export interface LargeFilesResult {
  scanId: number;
  root: string;
  scannedAtMs: number;
  minimumBytes: number;
  totalBytes: number;
  totalCount: number;
  returnedCount: number;
  truncated: boolean;
  skippedCount: number;
  cacheReused: boolean;
  entries: LargeFileEntry[];
}
