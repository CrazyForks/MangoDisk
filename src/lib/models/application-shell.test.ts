import { describe, expect, it } from 'vitest';

import {
  APP_SHELL_EXPANDED_MIN_WIDTH_PX,
  isAppShellExpanded,
  PAGE_IDS,
  PRIMARY_NAV_GROUPS,
} from './application-shell';
import { ICON_NAMES } from './ui';

describe('application shell layout', () => {
  it('expands the sidebar at the desktop shell breakpoint', () => {
    expect(isAppShellExpanded(APP_SHELL_EXPANDED_MIN_WIDTH_PX - 1)).toBe(false);
    expect(isAppShellExpanded(APP_SHELL_EXPANDED_MIN_WIDTH_PX)).toBe(true);
  });

  it('groups storage and system tools by user task', () => {
    expect(PRIMARY_NAV_GROUPS.map(group => group.id)).toEqual(['storage', 'system']);
    expect(PRIMARY_NAV_GROUPS[0].items.map(item => item.id)).toEqual([
      PAGE_IDS.cleanup,
      PAGE_IDS.largeFiles,
      PAGE_IDS.duplicateFiles,
      PAGE_IDS.analysis,
    ]);
    expect(PRIMARY_NAV_GROUPS[1].items.map(item => item.id)).toEqual([
      PAGE_IDS.applicationUninstall,
      PAGE_IDS.startup,
      PAGE_IDS.systemOptimization,
    ]);
  });

  it('uses the dedicated acceleration icon for system optimization', () => {
    expect(
      PRIMARY_NAV_GROUPS.flatMap(group => group.items).find(item => item.id === PAGE_IDS.systemOptimization)?.icon
    ).toBe(ICON_NAMES.systemOptimization);
  });
});
