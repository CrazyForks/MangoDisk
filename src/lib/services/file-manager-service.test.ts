import { beforeEach, describe, expect, it, vi } from 'vitest';

import { FileManagerService } from '@/lib/services/file-manager-service';

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));

describe('FileManagerService', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
  });

  it('reveals the requested item through the native adapter', async () => {
    await FileManagerService.reveal('/private/item');

    expect(invokeMock).toHaveBeenCalledWith('reveal_in_file_manager', { path: '/private/item' });
  });

  it('opens the application log directory without receiving a path from the page', async () => {
    await FileManagerService.openApplicationLogs();

    expect(invokeMock).toHaveBeenCalledWith('open_application_log_directory');
  });
});
