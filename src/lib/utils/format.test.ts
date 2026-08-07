import { describe, expect, it } from 'vitest';

import { FormatUtils } from './format';

describe('FormatUtils.diskCapacity', () => {
  it('uses decimal units for whole-volume capacity', () => {
    expect(FormatUtils.diskCapacity(1_000_000_000)).toBe('1 GB');
    expect(FormatUtils.diskCapacity(1_000_000_000_000)).toBe('1 TB');
    expect(FormatUtils.diskCapacity(53_400_000_000)).toBe('53.4 GB');
    expect(FormatUtils.diskCapacity(994_660_000_000)).toBe('994.7 GB');
  });

  it('handles empty and invalid values without exposing invalid numbers', () => {
    expect(FormatUtils.diskCapacity(0)).toBe('0 B');
    expect(FormatUtils.diskCapacity(Number.NaN)).toBe('0 B');
  });
});

describe('FormatUtils.dateTime', () => {
  it('uses the application locale instead of the host default', () => {
    const timestamp = new Date(2024, 2, 5, 15, 18).getTime();
    const chinese = FormatUtils.dateTime(timestamp, 'zh-CN');
    const english = FormatUtils.dateTime(timestamp, 'en-US');

    expect(chinese).toContain('2024');
    expect(chinese.indexOf('03')).toBeLessThan(chinese.indexOf('05'));
    expect(english.indexOf('03')).toBeLessThan(english.indexOf('05'));
    expect(chinese).not.toBe(english);
    expect(english).not.toContain(',');
    expect(english).toBe('03/05/2024 15:18');
  });

  it('returns the empty display text for invalid values', () => {
    expect(FormatUtils.dateTime(null, 'zh-CN')).toBe('—');
    expect(FormatUtils.dateTime(Number.NaN, 'en-US')).toBe('—');
  });
});
