import {
  DEFAULT_DUPLICATE_FILE_MINIMUM_BYTES,
  DEFAULT_DUPLICATE_KEEPER_RULE,
  DUPLICATE_FILE_MINIMUM_OPTIONS,
  DUPLICATE_KEEPER_RULE_IDS,
} from '@/lib/models/duplicate-file';
import { DEFAULT_LARGE_FILE_MINIMUM_BYTES, LARGE_FILE_MINIMUM_OPTIONS } from '@/lib/models/large-file';
import { isLanguageId, LANGUAGE_IDS, THEME_IDS } from '@/lib/models/settings';
import type { AppSettings } from '@/lib/models/settings';

type UnknownRecord = Readonly<Record<string, unknown>>;

/**
 * Settings validation is deterministic and independent of persistence. During
 * development, an incomplete or obsolete document is rejected instead of
 * being silently upgraded into the current shape.
 */
export class AppSettingsUtils {
  static defaults(language: AppSettings['language'] = LANGUAGE_IDS.enUS): AppSettings {
    return {
      language,
      theme: THEME_IDS.system,
      largeFileMinimumBytes: DEFAULT_LARGE_FILE_MINIMUM_BYTES,
      duplicateFileMinimumBytes: DEFAULT_DUPLICATE_FILE_MINIMUM_BYTES,
      duplicateKeeperRule: DEFAULT_DUPLICATE_KEEPER_RULE,
    };
  }

  static parse(value: unknown): AppSettings {
    if (
      !AppSettingsUtils.hasExactKeys(value, [
        'language',
        'theme',
        'largeFileMinimumBytes',
        'duplicateFileMinimumBytes',
        'duplicateKeeperRule',
      ])
    ) {
      throw new Error('Invalid app settings document');
    }
    const settings = value;
    if (
      !isLanguageId(settings.language) ||
      !AppSettingsUtils.includes(Object.values(THEME_IDS), settings.theme) ||
      !AppSettingsUtils.includes(LARGE_FILE_MINIMUM_OPTIONS, settings.largeFileMinimumBytes) ||
      !AppSettingsUtils.includes(DUPLICATE_FILE_MINIMUM_OPTIONS, settings.duplicateFileMinimumBytes) ||
      !AppSettingsUtils.includes(Object.values(DUPLICATE_KEEPER_RULE_IDS), settings.duplicateKeeperRule)
    ) {
      throw new Error('Invalid app settings value');
    }
    return {
      language: settings.language,
      theme: settings.theme,
      largeFileMinimumBytes: settings.largeFileMinimumBytes,
      duplicateFileMinimumBytes: settings.duplicateFileMinimumBytes,
      duplicateKeeperRule: settings.duplicateKeeperRule,
    };
  }

  private static hasExactKeys<const Keys extends readonly string[]>(
    value: unknown,
    expectedKeys: Keys
  ): value is UnknownRecord & Record<Keys[number], unknown> {
    if (typeof value !== 'object' || value === null || Array.isArray(value)) return false;
    const actualKeys = Object.keys(value).sort();
    return actualKeys.length === expectedKeys.length && expectedKeys.every(key => actualKeys.includes(key));
  }

  private static includes<const Values extends readonly unknown[]>(
    values: Values,
    value: unknown
  ): value is Values[number] {
    return values.includes(value);
  }
}
