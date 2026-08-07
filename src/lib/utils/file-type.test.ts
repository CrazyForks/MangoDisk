import { describe, expect, it } from 'vitest';

import { FILE_CATEGORY_IDS } from '@/lib/models/file-category';

import { FileTypeUtils } from './file-type';

describe('file type classification', () => {
  it('keeps executable formats in Other without losing their visual identity', () => {
    expect(FileTypeUtils.category('compiler.exe')).toBe(FILE_CATEGORY_IDS.other);
    expect(FileTypeUtils.category('package.pkg')).toBe(FILE_CATEGORY_IDS.other);
    expect(FileTypeUtils.descriptor('compiler.exe').kind).toBe('installer');
  });
});
