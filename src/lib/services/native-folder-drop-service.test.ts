import { beforeEach, describe, expect, it, vi } from 'vitest';

const onDragDropEvent = vi.fn();

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({ onDragDropEvent }),
}));

import { NativeFolderDropService, type NativeFolderDropEvent } from './native-folder-drop-service';

describe('NativeFolderDropService', () => {
  beforeEach(() => {
    onDragDropEvent.mockReset();
  });

  it('forwards the native payload without exposing the Tauri event envelope', async () => {
    let nativeListener: ((event: { payload: NativeFolderDropEvent }) => void) | undefined;
    const stop = vi.fn();
    onDragDropEvent.mockImplementation(async listener => {
      nativeListener = listener;
      return stop;
    });
    const listener = vi.fn();
    const unlisten = await NativeFolderDropService.listen(listener);
    const payload: NativeFolderDropEvent = {
      type: 'drop',
      paths: ['/tmp/example'],
      position: { x: 20, y: 40 } as Extract<NativeFolderDropEvent, { type: 'drop' }>['position'],
    };

    nativeListener?.({ payload });

    expect(listener).toHaveBeenCalledWith(payload);
    expect(unlisten).toBe(stop);
  });
});
