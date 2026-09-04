import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

import { WindowsStartupToolService } from './windows-startup-tool-service';

describe('WindowsStartupToolService', () => {
  beforeEach(() => invokeMock.mockReset());

  it.each(['services', 'taskScheduler'] as const)('opens only the typed %s system tool', async tool => {
    invokeMock.mockResolvedValue(undefined);

    await WindowsStartupToolService.open(tool);

    expect(invokeMock).toHaveBeenCalledWith('open_windows_startup_tool', { tool });
  });
});
