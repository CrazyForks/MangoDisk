import { describe, expect, it } from 'vitest';

import { DEFAULT_LARGE_FILE_MINIMUM_BYTES } from '@/lib/models/large-file';
import { LANGUAGE_IDS } from '@/lib/models/settings';
import { AppSettingsUtils } from '@/lib/utils/app-settings';

describe('AppSettingsUtils', () => {
  it('uses English and a 100 MB large-file threshold by default', () => {
    const settings = AppSettingsUtils.defaults();

    expect(settings.language).toBe(LANGUAGE_IDS.enUS);
    expect(settings.largeFileMinimumBytes).toBe(100 * 1024 * 1024);
    expect(settings.largeFileMinimumBytes).toBe(DEFAULT_LARGE_FILE_MINIMUM_BYTES);
  });

  it('rejects incomplete persisted settings', () => {
    expect(() => AppSettingsUtils.parse({})).toThrow('Invalid app settings document');
  });
});
