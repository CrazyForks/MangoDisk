import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ByteSizeService } from '@/lib/services/byte-size-service';

const { platformMock } = vi.hoisted(() => ({ platformMock: vi.fn() }));

vi.mock('@tauri-apps/plugin-os', () => ({ platform: platformMock }));

describe('ByteSizeService', () => {
  beforeEach(() => {
    platformMock.mockReset();
  });

  it('uses decimal units for every macOS size display', () => {
    platformMock.mockReturnValue('macos');

    expect(ByteSizeService.bytes(10_842_048)).toBe('10.8 MB');
    expect(ByteSizeService.bytes(53_400_000_000)).toBe('53.4 GB');
    // The threshold stays in raw bytes; only its macOS presentation uses the decimal base.
    expect(ByteSizeService.bytes(100 * 1024 * 1024)).toBe('105 MB');
  });

  it('uses binary units for every Windows size display', () => {
    platformMock.mockReturnValue('windows');

    expect(ByteSizeService.bytes(10_842_048)).toBe('10.3 MB');
    expect(ByteSizeService.bytes(50 * 1024 * 1024 * 1024)).toBe('50.0 GB');
    expect(ByteSizeService.bytes(100 * 1024 * 1024)).toBe('100 MB');
  });
});
