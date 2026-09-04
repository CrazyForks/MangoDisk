import { invoke } from '@tauri-apps/api/core';

export type WindowsStartupTool = 'services' | 'taskScheduler';

export const WindowsStartupToolService = {
  async open(tool: WindowsStartupTool): Promise<void> {
    await invoke<void>('open_windows_startup_tool', { tool });
  },
};
