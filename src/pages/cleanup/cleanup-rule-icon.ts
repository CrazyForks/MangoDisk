import { CLEANUP_RULE_IDS, type CleanupResultGroup } from '@/lib/models/cleanup';
import { ICON_NAMES, type IconName } from '@/lib/models/ui';

const CLEANUP_GROUP_ICONS: Readonly<Record<CleanupResultGroup, IconName>> = {
  system: ICON_NAMES.cleanupSystemCache,
  userCache: ICON_NAMES.cleanupUserCache,
  application: ICON_NAMES.cleanupApplicationCache,
  browser: ICON_NAMES.cleanupBrowserData,
  development: ICON_NAMES.cleanupDeveloperTools,
  project: ICON_NAMES.cleanupProjectArtifacts,
  xcode: ICON_NAMES.brandXcode,
  applicationOptimization: ICON_NAMES.cleanupApplicationOptimization,
  ai: ICON_NAMES.cleanupAiModelCache,
  container: ICON_NAMES.cleanupContainerCache,
};

// Rule IDs are stable protocol values, so brand selection remains independent
// from localized labels and private filesystem paths. Unknown rules deliberately
// fall back to their domain icon instead of guessing a visually similar brand.
const CLEANUP_RULE_ICONS: Readonly<Record<string, IconName>> = {
  'system.user-temp': ICON_NAMES.cleanupTemporaryFiles,
  'system.darwin-user-cache': ICON_NAMES.cleanupStaleCache,
  'system.old-diagnostic-logs': ICON_NAMES.cleanupDiagnosticLogs,
  'system.apple-media-cache': ICON_NAMES.brandApple,
  'system.apple-intelligence-cache': ICON_NAMES.brandApple,
  'system.directx-shader-cache': ICON_NAMES.brandWindows,
  [CLEANUP_RULE_IDS.windowsRecycleBin]: ICON_NAMES.trash,
  'special.windows-system-logs': ICON_NAMES.brandWindows,
  'special.windows-internet-cache': ICON_NAMES.brandWindows,
  'special.windows-delivery-optimization': ICON_NAMES.brandWindows,
  'special.windows-defender-cache': ICON_NAMES.brandWindows,
  'special.windows-update-cleanup': ICON_NAMES.brandWindows,

  'app.slack-cache': ICON_NAMES.brandSlack,
  'app.discord-cache': ICON_NAMES.brandDiscord,
  'app.zoom-cache': ICON_NAMES.brandZoom,
  'app.teams-cache': ICON_NAMES.brandTeams,
  'app.teams-msix-cache': ICON_NAMES.brandTeams,
  'app.chatgpt-cache': ICON_NAMES.brandOpenai,
  'app.figma-cache': ICON_NAMES.brandFigma,
  'app.adobe-media-cache': ICON_NAMES.brandAdobe,
  'app.office-cache': ICON_NAMES.brandOffice,
  'app.outlook-cache': ICON_NAMES.brandOffice,
  'app.apple-mail-cache': ICON_NAMES.brandApple,
  'app.iwork-cache': ICON_NAMES.brandApple,
  'app.core-device-cache': ICON_NAMES.brandApple,
  'app.google-updater-cache': ICON_NAMES.brandGoogle,
  'app.wechat-cache': ICON_NAMES.brandWechat,
  'app.wecom-cache': ICON_NAMES.brandWechat,
  'app.wechat-diagnostic-cache': ICON_NAMES.brandWechat,
  'app.dingtalk-diagnostic-cache': ICON_NAMES.brandDingtalk,
  'app.telegram-cache': ICON_NAMES.brandTelegram,
  'app.whatsapp-cache': ICON_NAMES.brandWhatsapp,
  'app.qq-cache': ICON_NAMES.brandQq,
  'app.zoom-diagnostic-cache': ICON_NAMES.brandZoom,
  'app.lark-renderer-cache': ICON_NAMES.brandLark,

  'browser.chrome-cache': ICON_NAMES.brandChrome,
  'browser.chrome-offline-cache': ICON_NAMES.brandChrome,
  'browser.edge-cache': ICON_NAMES.brandEdge,
  'browser.edge-offline-cache': ICON_NAMES.brandEdge,
  'browser.arc-cache': ICON_NAMES.brandArc,
  'browser.vivaldi-cache': ICON_NAMES.brandVivaldi,
  'browser.firefox-cache': ICON_NAMES.brandFirefox,
  'browser.opera-cache': ICON_NAMES.brandOpera,

  'dev.npm-cache': ICON_NAMES.brandNpm,
  'dev.npx-cache': ICON_NAMES.brandNpm,
  'dev.node-gyp-cache': ICON_NAMES.brandNodejs,
  'dev.node-tooling-cache': ICON_NAMES.brandNodejs,
  'dev.pnpm-cache': ICON_NAMES.brandPnpm,
  'dev.yarn-cache': ICON_NAMES.brandYarn,
  'dev.pip-cache': ICON_NAMES.brandPython,
  'dev.python-cache': ICON_NAMES.brandPython,
  'dev.python-tooling-cache': ICON_NAMES.brandPython,
  'dev.uv-cache': ICON_NAMES.brandPython,
  'dev.dart-analysis-cache': ICON_NAMES.brandFlutter,
  'dev.cargo-cache': ICON_NAMES.brandRust,
  'dev.cargo-extracted-sources': ICON_NAMES.brandRust,
  'dev.qclaw-compile-cache': ICON_NAMES.brandNodejs,
  'dev.go-cache': ICON_NAMES.brandGo,
  'dev.go-module-cache': ICON_NAMES.brandGo,
  'dev.swiftpm-cache': ICON_NAMES.brandSwift,
  'dev.vscode-cache': ICON_NAMES.brandVscode,
  'dev.visual-studio-cache': ICON_NAMES.brandVisualStudio,
  'dev.android-cache': ICON_NAMES.brandAndroid,
  'dev.android-user-cache': ICON_NAMES.brandAndroid,
  'dev.deno-cache': ICON_NAMES.brandDeno,
  'dev.copilot-cli-cache': ICON_NAMES.brandGithubCopilot,
  'dev.jetbrains-cache': ICON_NAMES.brandJetbrains,
  'dev.gradle-cache': ICON_NAMES.brandGradle,
  'dev.homebrew-cache': ICON_NAMES.brandHomebrew,

  'dev.xcode-derived-data': ICON_NAMES.brandXcode,
  'dev.xcode-auxiliary-cache': ICON_NAMES.brandXcode,
  'dev.xcode-simulator-cache': ICON_NAMES.brandXcode,
  'special.xcode-device-support': ICON_NAMES.brandXcode,
  'special.xcode-simulator-runtime': ICON_NAMES.brandXcode,
  'special.xcode-archives': ICON_NAMES.brandXcode,

  'special.ai-model-openai-clip': ICON_NAMES.brandOpenai,
  'ai.huggingface-xet-cache': ICON_NAMES.brandHuggingface,
  'special.ai-model-hugging-face': ICON_NAMES.brandHuggingface,
  'special.ai-model-ollama': ICON_NAMES.brandOllama,
  [CLEANUP_RULE_IDS.aiModelModelScope]: ICON_NAMES.modelRepository,
  'special.codex-archived-sessions': ICON_NAMES.brandOpenai,
  'special.rust-toolchains': ICON_NAMES.brandRust,

  'project.rust-build-artifacts': ICON_NAMES.brandRust,
  'project.node-build-artifacts': ICON_NAMES.brandNodejs,
  'project.react-native-build-artifacts': ICON_NAMES.brandReactNative,
  'project.python-build-artifacts': ICON_NAMES.brandPython,
  'project.dotnet-build-artifacts': ICON_NAMES.brandCSharp,
  'project.swift-build-artifacts': ICON_NAMES.brandSwift,
  'project.flutter-build-artifacts': ICON_NAMES.brandFlutter,
  'project.terraform-build-artifacts': ICON_NAMES.brandTerraform,
  'project.unity-build-artifacts': ICON_NAMES.brandUnity,
  'project.gradle-build-artifacts': ICON_NAMES.brandGradle,
  'project.godot-build-artifacts': ICON_NAMES.brandGodot,
  'project.cmake-build-artifacts': ICON_NAMES.brandCmake,

  [CLEANUP_RULE_IDS.dockerBuildCache]: ICON_NAMES.cleanupContainerCache,
  [CLEANUP_RULE_IDS.macosUniversalBinaries]: ICON_NAMES.cleanupUniversalBinary,
};

export function cleanupGroupIcon(group: CleanupResultGroup): IconName {
  return CLEANUP_GROUP_ICONS[group];
}

export function cleanupRuleIcon(ruleId: string, group: CleanupResultGroup): IconName {
  return CLEANUP_RULE_ICONS[ruleId] ?? cleanupGroupIcon(group);
}
