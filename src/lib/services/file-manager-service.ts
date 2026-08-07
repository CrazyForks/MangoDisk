import { invoke } from '@tauri-apps/api/core';

/** Exposes native file-manager actions without leaking Tauri into pages. */
export class FileManagerService {
  static reveal(path: string): Promise<void> {
    return invoke<void>('reveal_in_file_manager', { path });
  }

  static openApplicationLogs(): Promise<void> {
    return invoke<void>('open_application_log_directory');
  }
}
