import { afterEach, describe, expect, it, vi } from 'vitest';

import { i18n } from '@/i18n';
import { LANGUAGE_IDS, LANGUAGE_OPTIONS } from '@/lib/models/settings';
import { LanguageService } from '@/lib/services/language-service';
import enUS from '@/locales/en-US.json';
import zhCN from '@/locales/zh-CN.json';

function leafKeys(value: unknown, prefix = ''): string[] {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return [prefix];
  return Object.entries(value).flatMap(([key, child]) => leafKeys(child, prefix ? `${prefix}.${key}` : key));
}

function leafValues(value: unknown): unknown[] {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return [value];
  return Object.values(value).flatMap(leafValues);
}

function leafEntries(value: unknown, prefix = ''): Array<[string, unknown]> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return [[prefix, value]];
  return Object.entries(value).flatMap(([key, child]) => leafEntries(child, prefix ? `${prefix}.${key}` : key));
}

describe('i18n resources', () => {
  afterEach(() => {
    i18n.global.locale.value = LANGUAGE_IDS.zhCN;
    vi.unstubAllGlobals();
  });

  it('keeps shared schema keys aligned across locales', () => {
    const chineseKeys = new Set(leafKeys(zhCN));
    const englishKeys = new Set(leafKeys(enUS));
    const missingEnglishKeys = [...chineseKeys].filter(key => !englishKeys.has(key));
    // Curated rule entries are optional and can be translated independently.
    const unexpectedEnglishKeys = [...englishKeys].filter(
      key => !chineseKeys.has(key) && !key.startsWith('cleanupRules.entries.')
    );

    expect(missingEnglishKeys).toEqual([]);
    expect(unexpectedEnglishKeys).toEqual([]);
  });

  it('keeps selectable languages aligned with bundled locale resources', () => {
    expect(i18n.global.availableLocales.toSorted()).toEqual(LANGUAGE_OPTIONS.map(option => option.id).toSorted());
    for (const locale of i18n.global.availableLocales) {
      for (const option of LANGUAGE_OPTIONS) {
        expect(i18n.global.te(option.labelKey, locale)).toBe(true);
      }
    }
  });

  it('contains only non-empty localized strings', () => {
    for (const resource of [zhCN, enUS]) {
      const invalidValues = leafValues(resource).filter(value => typeof value !== 'string' || !value.trim());
      expect(invalidValues).toEqual([]);
    }
  });

  it('keeps compact interface copy free of trailing periods', () => {
    for (const resource of [zhCN, enUS]) {
      const keysWithTrailingPeriods = leafEntries(resource)
        .filter(([key, value]) => {
          if (key.startsWith('cleanupRules.entries.')) return false;
          return typeof value === 'string' && (value.endsWith('。') || value.endsWith('.'));
        })
        .map(([key]) => key);

      expect(keysWithTrailingPeriods).toEqual([]);
    }
  });

  it('keeps every curated rule presentation complete', () => {
    const incompleteRules = Object.entries(enUS.cleanupRules.entries)
      .filter(([, rule]) => !rule.name.trim() || !rule.description.trim() || !rule.impact.trim())
      .map(([ruleId]) => ruleId);

    expect(incompleteRules).toEqual([]);
  });

  it('synchronizes the composer and document language', () => {
    const documentStub = { documentElement: { lang: '' } };
    vi.stubGlobal('document', documentStub);

    LanguageService.apply(LANGUAGE_IDS.enUS);

    expect(i18n.global.locale.value).toBe(LANGUAGE_IDS.enUS);
    expect(documentStub.documentElement.lang).toBe(LANGUAGE_IDS.enUS);
    expect(i18n.global.t('common.cancel')).toBe('Cancel');
  });

  it('matches supported system languages and falls back to English', () => {
    expect(LanguageService.resolveSupportedLanguage(['zh-Hans-CN', 'en-US'])).toBe(LANGUAGE_IDS.zhCN);
    expect(LanguageService.resolveSupportedLanguage(['fr-FR', 'en-GB'])).toBe(LANGUAGE_IDS.enUS);
    expect(LanguageService.resolveSupportedLanguage(['ja-JP'])).toBe(LANGUAGE_IDS.enUS);
  });

  it('applies interpolation and pluralization for the active locale', () => {
    i18n.global.locale.value = LANGUAGE_IDS.zhCN;
    expect(i18n.global.t('common.fileCount', { count: 2 }, 2)).toBe('2 个文件');

    i18n.global.locale.value = LANGUAGE_IDS.enUS;
    expect(i18n.global.t('common.fileCount', { count: 1 }, 1)).toBe('1 file');
    expect(i18n.global.t('common.fileCount', { count: 2 }, 2)).toBe('2 files');
  });
});
