import { EMPTY_DISPLAY_TEXT } from '@/lib/models/ui';

const LOCAL_INTEGER_FORMATTER = new Intl.NumberFormat();
const LOCAL_DISK_CAPACITY_FORMATTER = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 1,
});
const LIST_FORMATTERS = new Map<string, Intl.ListFormat>();
const DATE_TIME_FORMATTERS = new Map<string, Intl.DateTimeFormat>();

const DATE_TIME_FORMAT_OPTIONS: Intl.DateTimeFormatOptions = {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  hourCycle: 'h23',
};

export class FormatUtils {
  static bytes(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'] as const;
    const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
    const value = bytes / 1024 ** index;
    const digits = value >= 100 || index === 0 ? 0 : value >= 10 ? 1 : 2;
    return `${value.toFixed(digits)} ${units[index]}`;
  }

  /*
   * macOS and modern storage settings report whole-volume capacity with
   * decimal SI units. Keep this separate from file-size formatting so changing
   * the system-disk summary does not alter cleanup results or file lists.
   */
  static diskCapacity(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
    const units = ['B', 'KB', 'MB', 'GB', 'TB'] as const;
    const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1000)), units.length - 1);
    const value = bytes / 1000 ** index;
    return `${LOCAL_DISK_CAPACITY_FORMATTER.format(value)} ${units[index]}`;
  }

  static storageThreshold(bytes: number): string {
    const units = [
      { bytes: 1024 ** 4, label: 'TB' },
      { bytes: 1024 ** 3, label: 'GB' },
      { bytes: 1024 ** 2, label: 'MB' },
      { bytes: 1024, label: 'KB' },
    ] as const;
    const exactUnit = units.find(unit => bytes >= unit.bytes && bytes % unit.bytes === 0);
    return exactUnit ? `${bytes / exactUnit.bytes} ${exactUnit.label}` : this.bytes(bytes);
  }

  static dateTime(timestamp: number | null | undefined, locale?: string): string {
    if (timestamp === null || timestamp === undefined || !Number.isFinite(timestamp)) {
      return EMPTY_DISPLAY_TEXT;
    }
    const cacheKey = locale || 'system';
    let formatter = DATE_TIME_FORMATTERS.get(cacheKey);
    if (!formatter) {
      // Dates follow the language selected inside MangoDisk instead of the
      // WebView locale. Keeping the time zone implicit still respects the
      // operating system location while preventing ambiguous month/day order.
      formatter = new Intl.DateTimeFormat(locale || undefined, DATE_TIME_FORMAT_OPTIONS);
      DATE_TIME_FORMATTERS.set(cacheKey, formatter);
    }
    return formatter
      .formatToParts(new Date(timestamp))
      .map(part => (part.type === 'literal' ? part.value.replace(/,\s*/g, ' ') : part.value))
      .join('');
  }

  static elapsedSeconds(startedAtMs: number, finishedAtMs: number): string {
    if (!Number.isFinite(startedAtMs) || !Number.isFinite(finishedAtMs)) {
      return EMPTY_DISPLAY_TEXT;
    }
    const seconds = Math.max(0, Math.round((finishedAtMs - startedAtMs) / 1000));
    return LOCAL_INTEGER_FORMATTER.format(seconds);
  }

  static integer(value: number): string {
    return LOCAL_INTEGER_FORMATTER.format(value);
  }

  static list(values: readonly string[], locale?: string): string {
    const cacheKey = locale || 'system';
    let formatter = LIST_FORMATTERS.get(cacheKey);
    if (!formatter) {
      formatter = new Intl.ListFormat(locale || undefined, {
        style: 'short',
        type: 'conjunction',
      });
      LIST_FORMATTERS.set(cacheKey, formatter);
    }
    return formatter.format(values);
  }

  static percent(value: number, total: number): number {
    if (total <= 0) return 0;
    return Math.min(100, Math.max(0, (value / total) * 100));
  }
}
