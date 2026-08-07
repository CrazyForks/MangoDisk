import { describe, expect, it } from 'vitest';

import { APP_SHELL_EXPANDED_MIN_WIDTH_PX, isAppShellExpanded } from './application-shell';

describe('application shell layout', () => {
  it('expands the sidebar at the desktop shell breakpoint', () => {
    expect(isAppShellExpanded(APP_SHELL_EXPANDED_MIN_WIDTH_PX - 1)).toBe(false);
    expect(isAppShellExpanded(APP_SHELL_EXPANDED_MIN_WIDTH_PX)).toBe(true);
  });
});
