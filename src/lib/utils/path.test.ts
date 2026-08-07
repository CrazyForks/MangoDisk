import { describe, expect, it } from 'vitest';

import { PathUtils } from '@/lib/utils/path';

describe('PathUtils.collapseOverlappingRoots', () => {
  it('ignores descendants already covered by a selected parent', () => {
    expect(
      PathUtils.collapseOverlappingRoots([
        '/Users/developer/Downloads',
        '/Users/developer/Downloads/projects',
        '/Users/developer/Documents',
      ])
    ).toEqual(['/Users/developer/Downloads', '/Users/developer/Documents']);
  });

  it('replaces earlier descendants when a parent is added later', () => {
    expect(
      PathUtils.collapseOverlappingRoots([
        '/Users/developer/Downloads/projects',
        '/Users/developer/Downloads/assets',
        '/Users/developer/Downloads',
      ])
    ).toEqual(['/Users/developer/Downloads']);
  });

  it('compares Windows roots without case or separator differences', () => {
    expect(
      PathUtils.collapseOverlappingRoots([
        'C:\\Users\\Developer\\Downloads',
        'c:/users/developer/downloads/projects',
        'D:\\Archive',
      ])
    ).toEqual(['C:\\Users\\Developer\\Downloads', 'D:\\Archive']);
  });

  it('keeps siblings whose names only share a prefix', () => {
    expect(
      PathUtils.collapseOverlappingRoots(['/Users/developer/Downloads', '/Users/developer/Downloads-archive'])
    ).toEqual(['/Users/developer/Downloads', '/Users/developer/Downloads-archive']);
  });
});
