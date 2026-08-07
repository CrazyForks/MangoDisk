import { createI18n } from 'vue-i18n';

import { LANGUAGE_IDS, type LanguageId } from '@/lib/models/settings';
import enUS from '@/locales/en-US.json';
import zhCN from '@/locales/zh-CN.json';

export type MessageSchema = typeof zhCN;
export type SupportedLocale = LanguageId;

/**
 * Both locale resources are small and must switch immediately while offline.
 * Static bundling avoids an unnecessary loading and failure state.
 */
export const i18n = createI18n<[MessageSchema], SupportedLocale>({
  legacy: false,
  globalInjection: false,
  locale: LANGUAGE_IDS.enUS,
  fallbackLocale: LANGUAGE_IDS.enUS,
  messages: {
    [LANGUAGE_IDS.zhCN]: zhCN,
    [LANGUAGE_IDS.enUS]: enUS,
  },
});
