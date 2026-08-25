import { describe, expect, it } from 'vitest';

import { APP_SHELL_EXPANDED_MIN_WIDTH_PX, isAppShellExpanded, PAGE_IDS, PRIMARY_NAV_ITEMS } from './application-shell';
import { ICON_NAMES } from './ui';

describe('application shell layout', () => {
  it('expands the sidebar at the desktop shell breakpoint', () => {
    expect(isAppShellExpanded(APP_SHELL_EXPANDED_MIN_WIDTH_PX - 1)).toBe(false);
    expect(isAppShellExpanded(APP_SHELL_EXPANDED_MIN_WIDTH_PX)).toBe(true);
  });

  it('places system optimization immediately after disk analysis', () => {
    const pageIds = PRIMARY_NAV_ITEMS.map(item => item.id);
    expect(pageIds.indexOf(PAGE_IDS.systemOptimization)).toBe(pageIds.indexOf(PAGE_IDS.analysis) + 1);
  });

  it('uses the dedicated acceleration icon for system optimization', () => {
    expect(PRIMARY_NAV_ITEMS.find(item => item.id === PAGE_IDS.systemOptimization)?.icon).toBe(
      ICON_NAMES.systemOptimization
    );
  });
});
