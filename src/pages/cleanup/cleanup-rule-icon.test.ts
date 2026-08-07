import { describe, expect, it } from 'vitest';

import { CLEANUP_RULE_IDS } from '@/lib/models/cleanup';
import { ICON_NAMES } from '@/lib/models/ui';

import { cleanupGroupIcon, cleanupRuleIcon } from './cleanup-rule-icon';

describe('cleanup rule icons', () => {
  it('uses exact public brand icons for known products and ecosystems', () => {
    expect(cleanupRuleIcon('browser.chrome-cache', 'userCache')).toBe(ICON_NAMES.brandChrome);
    expect(cleanupRuleIcon('dev.node-tooling-cache', 'development')).toBe(ICON_NAMES.brandNodejs);
    expect(cleanupRuleIcon('dev.uv-cache', 'development')).toBe(ICON_NAMES.brandPython);
    expect(cleanupRuleIcon('dev.dart-analysis-cache', 'development')).toBe(ICON_NAMES.brandFlutter);
    expect(cleanupRuleIcon('dev.cargo-extracted-sources', 'development')).toBe(ICON_NAMES.brandRust);
    expect(cleanupRuleIcon('dev.qclaw-compile-cache', 'development')).toBe(ICON_NAMES.brandNodejs);
    expect(cleanupRuleIcon('project.rust-build-artifacts', 'project')).toBe(ICON_NAMES.brandRust);
    expect(cleanupRuleIcon('app.wechat-cache', 'userCache')).toBe(ICON_NAMES.brandWechat);
    expect(cleanupRuleIcon('app.wechat-diagnostic-cache', 'application')).toBe(ICON_NAMES.brandWechat);
    expect(cleanupRuleIcon('dev.jetbrains-cache', 'userCache')).toBe(ICON_NAMES.brandJetbrains);
    expect(cleanupRuleIcon('dev.gradle-cache', 'development')).toBe(ICON_NAMES.brandGradle);
    expect(cleanupRuleIcon('project.gradle-build-artifacts', 'project')).toBe(ICON_NAMES.brandGradle);
    expect(cleanupRuleIcon('project.godot-build-artifacts', 'project')).toBe(ICON_NAMES.brandGodot);
    expect(cleanupRuleIcon('project.cmake-build-artifacts', 'project')).toBe(ICON_NAMES.brandCmake);
    expect(cleanupRuleIcon('dev.homebrew-cache', 'userCache')).toBe(ICON_NAMES.brandHomebrew);
    expect(cleanupRuleIcon('app.lark-renderer-cache', 'userCache')).toBe(ICON_NAMES.brandLark);
    expect(cleanupRuleIcon('special.xcode-simulator-runtime', 'xcode')).toBe(ICON_NAMES.brandXcode);
    expect(cleanupRuleIcon('special.ai-model-hugging-face', 'ai')).toBe(ICON_NAMES.brandHuggingface);
    expect(cleanupRuleIcon('special.ai-model-ollama', 'ai')).toBe(ICON_NAMES.brandOllama);
    expect(cleanupRuleIcon('special.ai-model-modelscope', 'ai')).toBe(ICON_NAMES.modelRepository);
    expect(cleanupRuleIcon('special.codex-archived-sessions', 'development')).toBe(ICON_NAMES.brandOpenai);
    expect(cleanupRuleIcon('special.rust-toolchains', 'development')).toBe(ICON_NAMES.brandRust);
    expect(cleanupRuleIcon(CLEANUP_RULE_IDS.windowsRecycleBin, 'system')).toBe(ICON_NAMES.trash);
  });

  it('uses the domain icon when no exact public brand icon is mapped', () => {
    expect(cleanupRuleIcon('dev.unknown-tool-cache', 'userCache')).toBe(ICON_NAMES.cleanupUserCache);
    expect(cleanupRuleIcon('project.unknown-build-artifacts', 'project')).toBe(ICON_NAMES.cleanupProjectArtifacts);
  });

  it('keeps category icons independent from individual rule brands', () => {
    expect(cleanupGroupIcon('browser')).toBe(ICON_NAMES.cleanupBrowserData);
    expect(cleanupGroupIcon('development')).toBe(ICON_NAMES.cleanupDeveloperTools);
    expect(cleanupGroupIcon('xcode')).toBe(ICON_NAMES.brandXcode);
  });
});
