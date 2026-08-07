import type { DuplicateKeeperRuleId } from './duplicate-file';

export const LANGUAGE_IDS = {
  zhCN: 'zh-CN',
  enUS: 'en-US',
} as const;

export type LanguageId = (typeof LANGUAGE_IDS)[keyof typeof LANGUAGE_IDS];

/*
 * Locale files own every translated label. This registry only describes the
 * stable locale ID and the translation key used by the settings selector, so
 * adding a language does not require another conditional branch in the page.
 */
export const LANGUAGE_OPTIONS = [
  { id: LANGUAGE_IDS.zhCN, labelKey: 'settings.languageNames.zhCN', browserLanguagePrefixes: ['zh'] },
  { id: LANGUAGE_IDS.enUS, labelKey: 'settings.languageNames.enUS', browserLanguagePrefixes: ['en'] },
] as const satisfies readonly {
  id: LanguageId;
  labelKey: string;
  browserLanguagePrefixes: readonly string[];
}[];

export function isLanguageId(value: unknown): value is LanguageId {
  return typeof value === 'string' && LANGUAGE_OPTIONS.some(option => option.id === value);
}

export const THEME_IDS = {
  system: 'system',
  light: 'light',
  dark: 'dark',
} as const;

export interface AppSettings {
  language: LanguageId;
  theme: (typeof THEME_IDS)[keyof typeof THEME_IDS];
  largeFileMinimumBytes: number;
  duplicateFileMinimumBytes: number;
  duplicateKeeperRule: DuplicateKeeperRuleId;
}
