use mangodisk_platform::PlatformSystemSettingValue;

use super::{
    SystemSettingCategory, SystemSettingRiskLevel, SystemSettingSelectionKind,
    SystemSettingsPlatform,
};

#[derive(Clone, Copy)]
pub(super) enum DefinitionValue {
    Missing,
    Boolean(bool),
    Integer(i64),
    Text(&'static str),
}

impl DefinitionValue {
    pub(super) fn owned(self) -> PlatformSystemSettingValue {
        match self {
            Self::Missing => PlatformSystemSettingValue::Missing,
            Self::Boolean(value) => PlatformSystemSettingValue::Boolean(value),
            Self::Integer(value) => PlatformSystemSettingValue::Integer(value),
            Self::Text(value) => PlatformSystemSettingValue::Text(value.to_string()),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct SettingDefinition {
    pub id: &'static str,
    pub category: SystemSettingCategory,
    pub selection_kind: SystemSettingSelectionKind,
    pub risk_level: SystemSettingRiskLevel,
    pub default_value: DefinitionValue,
    /// Optional explicit value used when an already optimized setting is switched off.
    /// This is needed when the operating-system default is itself the recommended value.
    pub disabled_value: Option<DefinitionValue>,
    pub recommended_value: DefinitionValue,
    pub requires_restart: bool,
    pub requires_elevation: bool,
}

const MACOS_SETTINGS: &[SettingDefinition] = &[
    one_click(
        "macos.finder.show-file-extensions",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.finder.show-path-bar",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.finder.show-status-bar",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.finder.show-hidden-files",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.finder.show-posix-path",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.finder.disable-animations",
        SystemSettingCategory::Performance,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.finder.keep-folders-on-top",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.finder.search-current-folder",
        SystemSettingCategory::Productivity,
        DefinitionValue::Text("SCev"),
        DefinitionValue::Text("SCcf"),
        true,
    ),
    custom(
        "macos.finder.disable-extension-warning",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        true,
    ),
    custom(
        "macos.finder.show-hard-drives-on-desktop",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.finder.show-external-drives-on-desktop",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.finder.show-removable-media-on-desktop",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.finder.default-list-view",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Text("Nlsv"),
        true,
    ),
    custom(
        "macos.finder.folders-first-on-desktop",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.finder.enable-quit-menu",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        true,
    ),
    with_risk(
        custom(
            "macos.finder.remove-old-trash-items",
            SystemSettingCategory::Storage,
            DefinitionValue::Missing,
            DefinitionValue::Boolean(true),
            true,
        ),
        SystemSettingRiskLevel::Caution,
    ),
    one_click(
        "macos.panels.expand-save",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        false,
    ),
    one_click(
        "macos.panels.expand-print",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        false,
    ),
    one_click(
        "macos.desktop.prevent-network-ds-store",
        SystemSettingCategory::Storage,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.desktop.prevent-usb-ds-store",
        SystemSettingCategory::Storage,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.dock.auto-hide",
        SystemSettingCategory::Appearance,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.dock.minimize-to-application",
        SystemSettingCategory::Appearance,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.dock.use-scale-effect",
        SystemSettingCategory::Performance,
        DefinitionValue::Text("genie"),
        DefinitionValue::Text("scale"),
        true,
    ),
    one_click(
        "macos.mission-control.keep-space-order",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        true,
    ),
    one_click(
        "macos.dock.hide-recent-apps",
        SystemSettingCategory::Privacy,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        true,
    ),
    one_click(
        "macos.dock.disable-launch-animation",
        SystemSettingCategory::Performance,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        true,
    ),
    custom(
        "macos.dock.show-only-open-apps",
        SystemSettingCategory::Appearance,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.dock.dim-hidden-apps",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.dock.remove-auto-hide-delay",
        SystemSettingCategory::Performance,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "macos.dock.enable-magnification",
        SystemSettingCategory::Appearance,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.mission-control.group-windows-by-app",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.window.disable-animations",
        SystemSettingCategory::Performance,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(false),
        true,
    ),
    custom(
        "macos.window.close-app-windows",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(false),
        false,
    ),
    with_disabled_value(
        custom(
            "macos.window.double-click-titlebar-minimize",
            SystemSettingCategory::Appearance,
            DefinitionValue::Text("Maximize"),
            DefinitionValue::Text("Minimize"),
            false,
        ),
        DefinitionValue::Text("Maximize"),
    ),
    one_click(
        "macos.screenshots.disable-shadow",
        SystemSettingCategory::Appearance,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        false,
    ),
    with_disabled_value(
        custom(
            "macos.screenshots.use-png",
            SystemSettingCategory::Appearance,
            DefinitionValue::Text("png"),
            DefinitionValue::Text("png"),
            false,
        ),
        DefinitionValue::Text("jpg"),
    ),
    one_click(
        "macos.screenshots.disable-thumbnail",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.keyboard.full-navigation",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(2),
        false,
    ),
    custom(
        "macos.keyboard.fast-key-repeat",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(1),
        false,
    ),
    custom(
        "macos.keyboard.short-repeat-delay",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(15),
        DefinitionValue::Integer(10),
        false,
    ),
    custom(
        "macos.keyboard.disable-press-and-hold",
        SystemSettingCategory::Performance,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.keyboard.use-standard-function-keys",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        false,
    ),
    custom(
        "macos.documents.save-locally",
        SystemSettingCategory::Storage,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.printing.quit-after-finish",
        SystemSettingCategory::Performance,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        false,
    ),
    one_click(
        "macos.safari.show-full-url",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.safari.disable-safe-download-auto-open",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(false),
        true,
    ),
    custom(
        "macos.safari.show-status-bar",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.safari.enable-develop-menu",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.textedit.plain-text-default",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(false),
        true,
    ),
    one_click(
        "macos.photos.disable-auto-open",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        false,
    ),
    one_click(
        "macos.privacy.disable-personalized-ads",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.sharing.disable-airdrop",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.activity-monitor.show-all-processes",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(100),
        DefinitionValue::Integer(0),
        true,
    ),
    one_click(
        "macos.app-store.enable-auto-updates",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        false,
    ),
    custom(
        "macos.text.disable-auto-correct",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.text.disable-smart-quotes",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.text.disable-smart-dashes",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.text.disable-auto-capitalization",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.text.disable-period-substitution",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    with_disabled_value(
        custom(
            "macos.finder.warn-before-empty-trash",
            SystemSettingCategory::Productivity,
            DefinitionValue::Boolean(true),
            DefinitionValue::Boolean(true),
            false,
        ),
        DefinitionValue::Boolean(false),
    ),
    custom(
        "macos.dock.enable-spring-loading",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.dock.scroll-to-expose",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Boolean(true),
        true,
    ),
    custom(
        "macos.dock.fast-auto-hide-animation",
        SystemSettingCategory::Performance,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        true,
    ),
    one_click(
        "macos.safari.disable-search-suggestions",
        SystemSettingCategory::Privacy,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        true,
    ),
    one_click(
        "macos.safari.disable-top-hit-preload",
        SystemSettingCategory::Privacy,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        true,
    ),
    custom(
        "macos.text.disable-text-completion",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.text.disable-inline-predictions",
        SystemSettingCategory::Privacy,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.sound.disable-volume-feedback",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(true),
        DefinitionValue::Boolean(false),
        false,
    ),
    custom(
        "macos.time-machine.hide-new-disk-prompts",
        SystemSettingCategory::Productivity,
        DefinitionValue::Boolean(false),
        DefinitionValue::Boolean(true),
        false,
    ),
    with_disabled_value(
        custom(
            "macos.security.require-password-after-sleep",
            SystemSettingCategory::Privacy,
            DefinitionValue::Boolean(true),
            DefinitionValue::Boolean(true),
            false,
        ),
        DefinitionValue::Boolean(false),
    ),
    with_disabled_value(
        custom(
            "macos.security.lock-immediately-after-sleep",
            SystemSettingCategory::Privacy,
            DefinitionValue::Integer(0),
            DefinitionValue::Integer(0),
            false,
        ),
        DefinitionValue::Integer(5),
    ),
];

const WINDOWS_SETTINGS: &[SettingDefinition] = &[
    one_click(
        "windows.explorer.show-file-extensions",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "windows.explorer.show-hidden-files",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.explorer.launch-this-pc",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.explorer.compact-mode",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.explorer.show-full-path",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.explorer.show-item-checkboxes",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    one_click(
        "windows.explorer.hide-recent-files",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "windows.explorer.hide-frequent-folders",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    one_click(
        "windows.explorer.enable-auto-suggest",
        SystemSettingCategory::Productivity,
        DefinitionValue::Text("no"),
        DefinitionValue::Text("yes"),
        true,
    ),
    custom(
        "windows.explorer.disable-aero-shake",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    one_click(
        "windows.explorer.show-operation-details",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    one_click(
        "windows.explorer.hide-sync-notifications",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    with_disabled_value(
        custom(
            "windows.explorer.show-status-bar",
            SystemSettingCategory::Productivity,
            DefinitionValue::Integer(1),
            DefinitionValue::Integer(1),
            true,
        ),
        DefinitionValue::Integer(0),
    ),
    custom(
        "windows.explorer.separate-process",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.explorer.restore-folders-at-login",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.explorer.disable-sharing-wizard",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "windows.explorer.confirm-file-delete",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.explorer.use-manual-default-printer",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        false,
    ),
    custom(
        "windows.explorer.classic-context-menu",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Text(""),
        true,
    ),
    elevated_custom(
        "windows.explorer.remove-cast-to-device",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Text("Play to Menu"),
        true,
    ),
    custom(
        "windows.taskbar.show-seconds",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.taskbar.hide-task-view",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    elevated_custom(
        "windows.taskbar.hide-widgets",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        true,
    ),
    elevated_custom(
        "windows.taskbar.disable-widgets-board",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.taskbar.enable-end-task",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.taskbar.hide-search",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(0),
        true,
    ),
    elevated_custom(
        "windows.taskbar.hide-search-policy",
        SystemSettingCategory::Appearance,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "windows.taskbar.align-left",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "windows.taskbar.show-all-tray-icons",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "windows.taskbar.hide-badges",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.taskbar.disable-flashing",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.taskbar.hide-window-sharing",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    with_disabled_value(
        custom(
            "windows.taskbar.show-desktop-corner",
            SystemSettingCategory::Productivity,
            DefinitionValue::Integer(1),
            DefinitionValue::Integer(1),
            false,
        ),
        DefinitionValue::Integer(0),
    ),
    elevated_custom(
        "windows.taskbar.hide-weather",
        SystemSettingCategory::Appearance,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "windows.taskbar.hide-chat",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "windows.taskbar.hide-copilot",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    one_click(
        "windows.taskbar.disable-animations",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    with_disabled_value(
        custom(
            "windows.taskbar.show-on-all-displays",
            SystemSettingCategory::Productivity,
            DefinitionValue::Integer(1),
            DefinitionValue::Integer(1),
            true,
        ),
        DefinitionValue::Integer(0),
    ),
    custom(
        "windows.desktop.prefer-performance-effects",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(2),
        true,
    ),
    custom(
        "windows.windowing.disable-snap-assist",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "windows.personalization.dark-apps",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.personalization.dark-system",
        SystemSettingCategory::Appearance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    one_click(
        "windows.personalization.disable-transparency",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.gaming.enable-game-mode",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        false,
    ),
    custom(
        "windows.gaming.disable-capture",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.gaming.disable-game-dvr",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.gaming.optimize-windowed-games",
        SystemSettingCategory::Gaming,
        DefinitionValue::Text(""),
        DefinitionValue::Text("SwapEffectUpgradeEnable=1;"),
        true,
    ),
    custom(
        "windows.gaming.disable-game-bar-controller",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.gaming.disable-game-bar-tips",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.gaming.disable-dynamic-lighting",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.gaming.disable-app-lighting-control",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.performance.disable-background-store-apps",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        false,
    ),
    one_click(
        "windows.privacy.disable-advertising-id",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.privacy.disable-tailored-experiences",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.privacy.disable-activity-publishing",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.privacy.disable-activity-upload",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.privacy.disable-input-personalization",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        false,
    ),
    one_click(
        "windows.privacy.disable-ink-personalization",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        false,
    ),
    one_click(
        "windows.privacy.disable-feedback-requests",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.privacy.disable-nearby-sharing",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.privacy.disable-cross-device-experiences",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.privacy.disable-language-list-sharing",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        false,
    ),
    with_disabled_value(
        custom(
            "windows.clipboard.disable-history",
            SystemSettingCategory::Privacy,
            DefinitionValue::Integer(0),
            DefinitionValue::Integer(0),
            false,
        ),
        DefinitionValue::Integer(1),
    ),
    one_click(
        "windows.content.disable-silent-app-installs",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.content.disable-suggestions",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.content.disable-lock-screen-tips",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.content.disable-welcome-experience",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.content.disable-usage-tips",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.content.disable-notification-suggestions",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.content.disable-device-setup-suggestions",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.content.disable-spotlight",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.content.disable-phone-link-suggestions",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.content.disable-service-suggestions",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.content.disable-preinstalled-app-suggestions",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    elevated_one_click(
        "windows.search.disable-highlights",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.start.disable-recommendations",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.start.disable-account-notifications",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.start.hide-recently-added-apps",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.start.hide-most-used-apps",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    one_click(
        "windows.start.hide-recent-items",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    elevated_custom(
        "windows.edge.disable-sidebar",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    elevated_one_click(
        "windows.edge.disable-personalization",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    elevated_one_click(
        "windows.edge.disable-recommendations",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    elevated_one_click(
        "windows.edge.limit-diagnostic-data",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(1),
        true,
    ),
    elevated_custom(
        "windows.edge.disable-web-widget",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        true,
    ),
    custom(
        "windows.office.disable-optional-telemetry",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Integer(3),
        true,
    ),
    elevated_custom(
        "windows.firefox.disable-telemetry",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Integer(1),
        true,
    ),
    elevated_one_click(
        "windows.ai.disable-recall-snapshots",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    elevated_custom(
        "windows.ai.disable-copilot",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.typing.disable-autocorrect",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.typing.disable-spellcheck",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.typing.disable-text-prediction",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.typing.disable-double-space-period",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        false,
    ),
    custom(
        "windows.accessibility.disable-sticky-keys-shortcut",
        SystemSettingCategory::Productivity,
        DefinitionValue::Text("510"),
        DefinitionValue::Text("506"),
        false,
    ),
    custom(
        "windows.accessibility.disable-filter-keys-shortcut",
        SystemSettingCategory::Productivity,
        DefinitionValue::Text("126"),
        DefinitionValue::Text("122"),
        false,
    ),
    custom(
        "windows.accessibility.disable-toggle-keys-shortcut",
        SystemSettingCategory::Productivity,
        DefinitionValue::Text("62"),
        DefinitionValue::Text("58"),
        false,
    ),
    elevated_custom(
        "windows.search.disable-web-suggestions",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    custom(
        "windows.storage.enable-storage-sense",
        SystemSettingCategory::Storage,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        false,
    ),
    one_click(
        "windows.desktop.reduce-menu-delay",
        SystemSettingCategory::Productivity,
        DefinitionValue::Text("400"),
        DefinitionValue::Text("200"),
        false,
    ),
    one_click(
        "windows.desktop.reduce-hover-delay",
        SystemSettingCategory::Productivity,
        DefinitionValue::Text("400"),
        DefinitionValue::Text("100"),
        false,
    ),
    elevated_custom(
        "windows.performance.reduce-crash-dump",
        SystemSettingCategory::Storage,
        DefinitionValue::Integer(7),
        DefinitionValue::Integer(3),
        true,
    ),
    elevated_custom(
        "windows.privacy.disable-remote-assistance",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    elevated_custom(
        "windows.performance.reduce-service-shutdown-timeout",
        SystemSettingCategory::Performance,
        DefinitionValue::Text("5000"),
        DefinitionValue::Text("2000"),
        true,
    ),
    elevated_custom(
        "windows.performance.disable-network-throttling",
        SystemSettingCategory::Gaming,
        DefinitionValue::Missing,
        DefinitionValue::Integer(4_294_967_295),
        true,
    ),
    elevated_custom(
        "windows.performance.remove-reserved-bandwidth",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(80),
        DefinitionValue::Integer(0),
        true,
    ),
    elevated_custom(
        "windows.performance.multimedia-responsiveness",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(14),
        DefinitionValue::Integer(1),
        true,
    ),
    elevated_custom(
        "windows.performance.multimedia-no-lazy-mode",
        SystemSettingCategory::Gaming,
        DefinitionValue::Missing,
        DefinitionValue::Integer(1),
        true,
    ),
    elevated_custom(
        "windows.performance.multimedia-always-on",
        SystemSettingCategory::Gaming,
        DefinitionValue::Missing,
        DefinitionValue::Integer(1),
        true,
    ),
    elevated_custom(
        "windows.gaming.enable-hardware-gpu-scheduling",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(2),
        true,
    ),
    elevated_custom(
        "windows.compatibility.disable-camera-frame-server",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    elevated_custom(
        "windows.services.print-spooler-manual",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(3),
        true,
    ),
    elevated_custom(
        "windows.services.disable-sysmain",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(4),
        true,
    ),
    elevated_custom(
        "windows.services.disable-compatibility-assistant",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(4),
        true,
    ),
    with_risk(
        elevated_custom(
            "windows.services.disable-search-indexing",
            SystemSettingCategory::Performance,
            DefinitionValue::Integer(2),
            DefinitionValue::Integer(4),
            true,
        ),
        SystemSettingRiskLevel::Caution,
    ),
    elevated_custom(
        "windows.services.disable-diagnostic-tracking",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(4),
        true,
    ),
    elevated_custom(
        "windows.services.disable-error-reporting",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(4),
        true,
    ),
    elevated_custom(
        "windows.services.disable-sensors",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(4),
        true,
    ),
    elevated_custom(
        "windows.services.disable-insider",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(3),
        DefinitionValue::Integer(4),
        true,
    ),
    elevated_custom(
        "windows.services.disable-xbox-auth",
        SystemSettingCategory::Gaming,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(4),
        true,
    ),
    elevated_custom(
        "windows.services.disable-fax",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(3),
        DefinitionValue::Integer(4),
        true,
    ),
    elevated_custom(
        "windows.services.disable-media-player-sharing",
        SystemSettingCategory::Performance,
        DefinitionValue::Integer(3),
        DefinitionValue::Integer(4),
        true,
    ),
    with_risk(
        elevated_custom(
            "windows.security.disable-system-restore",
            SystemSettingCategory::Storage,
            DefinitionValue::Missing,
            DefinitionValue::Integer(1),
            true,
        ),
        SystemSettingRiskLevel::High,
    ),
    with_risk(
        elevated_custom(
            "windows.security.disable-defender",
            SystemSettingCategory::Privacy,
            DefinitionValue::Missing,
            DefinitionValue::Integer(1),
            true,
        ),
        SystemSettingRiskLevel::High,
    ),
    with_risk(
        elevated_custom(
            "windows.security.disable-smartscreen",
            SystemSettingCategory::Privacy,
            DefinitionValue::Missing,
            DefinitionValue::Integer(0),
            true,
        ),
        SystemSettingRiskLevel::High,
    ),
    custom(
        "windows.security.disable-autorun",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(145),
        DefinitionValue::Integer(255),
        true,
    ),
    elevated_custom(
        "windows.network.disable-llmnr",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    elevated_custom(
        "windows.network.disable-smb1",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        true,
    ),
    with_risk(
        elevated_custom(
            "windows.network.disable-smb2",
            SystemSettingCategory::Privacy,
            DefinitionValue::Missing,
            DefinitionValue::Integer(0),
            true,
        ),
        SystemSettingRiskLevel::High,
    ),
    with_risk(
        elevated_custom(
            "windows.security.disable-vbs",
            SystemSettingCategory::Gaming,
            DefinitionValue::Integer(1),
            DefinitionValue::Integer(0),
            true,
        ),
        SystemSettingRiskLevel::High,
    ),
    elevated_custom(
        "windows.storage.disable-ntfs-last-access",
        SystemSettingCategory::Storage,
        DefinitionValue::Integer(2),
        DefinitionValue::Integer(1),
        true,
    ),
    with_risk(
        elevated_custom(
            "windows.update.notify-before-download",
            SystemSettingCategory::Productivity,
            DefinitionValue::Missing,
            DefinitionValue::Integer(2),
            true,
        ),
        SystemSettingRiskLevel::Caution,
    ),
    elevated_custom(
        "windows.update.disable-peer-sharing",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        false,
    ),
    elevated_one_click(
        "windows.update.disable-preview-builds",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        false,
    ),
    elevated_custom(
        "windows.update.prevent-restart-when-logged-on",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Integer(1),
        false,
    ),
    elevated_custom(
        "windows.update.enable-microsoft-product-updates",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        false,
    ),
    elevated_custom(
        "windows.update.enable-restart-notifications",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        false,
    ),
    with_risk(
        elevated_custom(
            "windows.update.disable-automatic-updates",
            SystemSettingCategory::Productivity,
            DefinitionValue::Missing,
            DefinitionValue::Integer(1),
            false,
        ),
        SystemSettingRiskLevel::High,
    ),
    elevated_one_click(
        "windows.privacy.limit-diagnostic-data",
        SystemSettingCategory::Privacy,
        DefinitionValue::Missing,
        DefinitionValue::Integer(1),
        true,
    ),
    with_risk(
        elevated_custom(
            "windows.update.exclude-driver-updates",
            SystemSettingCategory::Productivity,
            DefinitionValue::Integer(0),
            DefinitionValue::Integer(1),
            true,
        ),
        SystemSettingRiskLevel::Caution,
    ),
    with_risk(
        elevated_custom(
            "windows.update.disable-store-auto-updates",
            SystemSettingCategory::Productivity,
            DefinitionValue::Missing,
            DefinitionValue::Integer(2),
            true,
        ),
        SystemSettingRiskLevel::Caution,
    ),
    elevated_custom(
        "windows.cloud.disable-onedrive-sync",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    elevated_custom(
        "windows.search.disable-cortana-policy",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    elevated_custom(
        "windows.input.disable-windows-ink",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    elevated_custom(
        "windows.clipboard.disable-cross-device-policy",
        SystemSettingCategory::Privacy,
        DefinitionValue::Integer(1),
        DefinitionValue::Integer(0),
        true,
    ),
    elevated_custom(
        "windows.filesystem.enable-long-paths",
        SystemSettingCategory::Productivity,
        DefinitionValue::Integer(0),
        DefinitionValue::Integer(1),
        true,
    ),
    with_risk(
        elevated_custom(
            "windows.setup.bypass-hardware-checks",
            SystemSettingCategory::Productivity,
            DefinitionValue::Integer(0),
            DefinitionValue::Integer(1),
            true,
        ),
        SystemSettingRiskLevel::Caution,
    ),
    elevated_custom(
        "windows.time.use-utc-hardware-clock",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Integer(1),
        true,
    ),
    elevated_custom(
        "windows.power.disable-modern-standby",
        SystemSettingCategory::Performance,
        DefinitionValue::Missing,
        DefinitionValue::Integer(0),
        true,
    ),
    with_risk(
        elevated_custom(
            "windows.power.disable-hibernation",
            SystemSettingCategory::Storage,
            DefinitionValue::Integer(1),
            DefinitionValue::Integer(0),
            true,
        ),
        SystemSettingRiskLevel::Caution,
    ),
    elevated_custom(
        "windows.recovery.enable-registry-backups",
        SystemSettingCategory::Storage,
        DefinitionValue::Missing,
        DefinitionValue::Integer(1),
        true,
    ),
    elevated_custom(
        "windows.login.enable-verbose-status",
        SystemSettingCategory::Productivity,
        DefinitionValue::Missing,
        DefinitionValue::Integer(1),
        true,
    ),
];

const fn one_click(
    id: &'static str,
    category: SystemSettingCategory,
    default_value: DefinitionValue,
    recommended_value: DefinitionValue,
    requires_restart: bool,
) -> SettingDefinition {
    SettingDefinition {
        id,
        category,
        selection_kind: SystemSettingSelectionKind::OneClick,
        risk_level: SystemSettingRiskLevel::Standard,
        default_value,
        disabled_value: None,
        recommended_value,
        requires_restart,
        requires_elevation: false,
    }
}

const fn elevated_one_click(
    id: &'static str,
    category: SystemSettingCategory,
    default_value: DefinitionValue,
    recommended_value: DefinitionValue,
    requires_restart: bool,
) -> SettingDefinition {
    SettingDefinition {
        id,
        category,
        selection_kind: SystemSettingSelectionKind::OneClick,
        risk_level: SystemSettingRiskLevel::Standard,
        default_value,
        disabled_value: None,
        recommended_value,
        requires_restart,
        requires_elevation: true,
    }
}

const fn custom(
    id: &'static str,
    category: SystemSettingCategory,
    default_value: DefinitionValue,
    recommended_value: DefinitionValue,
    requires_restart: bool,
) -> SettingDefinition {
    SettingDefinition {
        id,
        category,
        selection_kind: SystemSettingSelectionKind::Custom,
        risk_level: SystemSettingRiskLevel::Standard,
        default_value,
        disabled_value: None,
        recommended_value,
        requires_restart,
        requires_elevation: false,
    }
}

const fn elevated_custom(
    id: &'static str,
    category: SystemSettingCategory,
    default_value: DefinitionValue,
    recommended_value: DefinitionValue,
    requires_restart: bool,
) -> SettingDefinition {
    SettingDefinition {
        id,
        category,
        selection_kind: SystemSettingSelectionKind::Custom,
        risk_level: SystemSettingRiskLevel::Standard,
        default_value,
        disabled_value: None,
        recommended_value,
        requires_restart,
        requires_elevation: true,
    }
}

const fn with_disabled_value(
    mut definition: SettingDefinition,
    disabled_value: DefinitionValue,
) -> SettingDefinition {
    definition.disabled_value = Some(disabled_value);
    definition
}

const fn with_risk(
    mut definition: SettingDefinition,
    risk_level: SystemSettingRiskLevel,
) -> SettingDefinition {
    definition.risk_level = risk_level;
    definition
}

pub(super) fn definitions(platform: SystemSettingsPlatform) -> &'static [SettingDefinition] {
    match platform {
        SystemSettingsPlatform::Macos => MACOS_SETTINGS,
        SystemSettingsPlatform::Windows => WINDOWS_SETTINGS,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn platform_catalogs_have_unique_namespaced_identifiers() {
        for (platform, prefix) in [
            (SystemSettingsPlatform::Macos, "macos."),
            (SystemSettingsPlatform::Windows, "windows."),
        ] {
            let definitions = definitions(platform);
            let ids = definitions
                .iter()
                .map(|definition| definition.id)
                .collect::<BTreeSet<_>>();
            assert_eq!(ids.len(), definitions.len());
            assert!(ids.iter().all(|id| id.starts_with(prefix)));
            assert!(definitions.iter().any(|definition| {
                definition.selection_kind == SystemSettingSelectionKind::OneClick
            }));
        }
    }

    #[test]
    fn macos_destructive_and_personal_preferences_remain_explicit() {
        let macos = definitions(SystemSettingsPlatform::Macos);
        let old_trash = macos
            .iter()
            .find(|definition| definition.id == "macos.finder.remove-old-trash-items")
            .expect("the automatic Trash cleanup setting should exist");
        let function_keys = macos
            .iter()
            .find(|definition| definition.id == "macos.keyboard.use-standard-function-keys")
            .expect("the standard function keys setting should exist");

        assert_eq!(old_trash.risk_level, SystemSettingRiskLevel::Caution);
        assert_eq!(old_trash.selection_kind, SystemSettingSelectionKind::Custom);
        assert_eq!(
            function_keys.selection_kind,
            SystemSettingSelectionKind::Custom
        );
        assert!(matches!(
            function_keys.default_value,
            DefinitionValue::Missing
        ));
    }

    #[test]
    fn protected_windows_policy_settings_advertise_elevation() {
        let policy_ids = [
            "windows.edge.disable-sidebar",
            "windows.edge.disable-personalization",
            "windows.edge.disable-recommendations",
            "windows.edge.limit-diagnostic-data",
            "windows.ai.disable-recall-snapshots",
            "windows.ai.disable-copilot",
            "windows.search.disable-web-suggestions",
            "windows.search.disable-highlights",
            "windows.taskbar.hide-weather",
            "windows.taskbar.hide-widgets",
            "windows.taskbar.disable-widgets-board",
            "windows.taskbar.hide-search-policy",
            "windows.explorer.remove-cast-to-device",
            "windows.edge.disable-web-widget",
            "windows.firefox.disable-telemetry",
            "windows.update.disable-peer-sharing",
            "windows.update.disable-preview-builds",
            "windows.update.prevent-restart-when-logged-on",
            "windows.update.disable-automatic-updates",
            "windows.update.enable-microsoft-product-updates",
            "windows.update.enable-restart-notifications",
            "windows.privacy.limit-diagnostic-data",
            "windows.network.disable-llmnr",
        ];
        let windows = definitions(SystemSettingsPlatform::Windows);

        for setting_id in policy_ids {
            let definition = windows
                .iter()
                .find(|definition| definition.id == setting_id)
                .expect("the protected policy setting should exist");
            assert!(definition.requires_elevation);
        }
    }

    #[test]
    fn security_and_recovery_tradeoffs_advertise_high_risk() {
        let high_risk_ids = [
            "windows.security.disable-system-restore",
            "windows.security.disable-defender",
            "windows.security.disable-smartscreen",
            "windows.security.disable-vbs",
            "windows.network.disable-smb2",
            "windows.update.disable-automatic-updates",
        ];
        let windows = definitions(SystemSettingsPlatform::Windows);

        for setting_id in high_risk_ids {
            let definition = windows
                .iter()
                .find(|definition| definition.id == setting_id)
                .expect("the high-risk setting should exist");
            assert_eq!(definition.risk_level, SystemSettingRiskLevel::High);
            assert_eq!(
                definition.selection_kind,
                SystemSettingSelectionKind::Custom
            );
        }
    }

    #[test]
    fn taskbar_weather_policy_restores_to_not_configured() {
        let definition = definitions(SystemSettingsPlatform::Windows)
            .iter()
            .find(|definition| definition.id == "windows.taskbar.hide-weather")
            .expect("the taskbar weather setting should exist");

        assert!(matches!(definition.default_value, DefinitionValue::Missing));
        assert!(matches!(
            definition.recommended_value,
            DefinitionValue::Integer(0)
        ));
        assert!(definition.requires_elevation);
    }

    #[test]
    fn settings_with_an_optimized_default_define_an_explicit_off_value() {
        for platform in [
            SystemSettingsPlatform::Macos,
            SystemSettingsPlatform::Windows,
        ] {
            for definition in definitions(platform) {
                if definition.default_value.owned() != definition.recommended_value.owned() {
                    continue;
                }

                let disabled_value = definition.disabled_value.unwrap_or_else(|| {
                    panic!(
                        "setting {} needs an explicit value for the off switch state",
                        definition.id
                    )
                });
                assert_ne!(disabled_value.owned(), definition.recommended_value.owned());
            }
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore = "temporarily changes each applicable Windows setting and restores its exact value"]
    fn actual_windows_catalog_values_roundtrip_and_restore() {
        use mangodisk_platform::{
            current_platform, PlatformCancellation, PlatformSystemSettingChangeRequest,
            PlatformSystemSettingDiagnosticCode, SystemSettingsPlatform as _,
        };

        let definitions = definitions(SystemSettingsPlatform::Windows);
        let ids = definitions
            .iter()
            .map(|definition| definition.id)
            .collect::<Vec<_>>();
        let platform = current_platform();
        let cancellation = PlatformCancellation::new(|| false);
        let states = platform
            .scan_system_settings(&ids, &cancellation)
            .expect("the Windows settings catalog should be readable");
        let mut failures = Vec::new();
        let mut tested_count = 0_usize;
        let mut unsupported_count = 0_usize;

        for (definition, state) in definitions.iter().zip(states) {
            if state.diagnostic == Some(PlatformSystemSettingDiagnosticCode::Unsupported) {
                unsupported_count += 1;
                continue;
            }
            if let Some(diagnostic) = state.diagnostic {
                failures.push(format!("{}: initial read {diagnostic:?}", definition.id));
                continue;
            }

            let original = state.value;
            let original_effective = state.effective_value;
            let recommended = definition.recommended_value.owned();
            let disabled = definition
                .disabled_value
                .unwrap_or(definition.default_value)
                .owned();
            let desired = if original_effective == recommended {
                disabled
            } else {
                recommended
            };
            if desired == original {
                failures.push(format!(
                    "{}: catalog has no distinct value for roundtrip validation",
                    definition.id
                ));
                continue;
            }

            let apply = platform.change_system_setting(&PlatformSystemSettingChangeRequest {
                setting_id: definition.id.to_string(),
                expected_value: original.clone(),
                desired_value: desired.clone(),
            });
            match apply {
                Ok(result) if result.verified => {}
                Ok(result) => failures.push(format!(
                    "{}: apply did not verify changed={} verified={}",
                    definition.id, result.changed, result.verified
                )),
                Err(error) => failures.push(format!(
                    "{}: apply failed with {:?}",
                    definition.id,
                    error.code()
                )),
            }

            // Always observe the registry again before restoring. This covers the uncommon case
            // where a write succeeds but the platform adapter reports a subsequent read failure.
            let after = platform
                .scan_system_settings(&[definition.id], &cancellation)
                .expect("a post-change Windows setting read should complete")
                .into_iter()
                .next()
                .expect("a post-change Windows setting state should exist");
            if after.diagnostic.is_none() && after.value != original {
                if after.effective_value != desired {
                    failures.push(format!("{}: applied value did not match", definition.id));
                }
                let restore = platform.change_system_setting(&PlatformSystemSettingChangeRequest {
                    setting_id: definition.id.to_string(),
                    expected_value: after.value.clone(),
                    desired_value: original.clone(),
                });
                match restore {
                    Ok(result) if result.verified && result.value == original => {}
                    Ok(result) => failures.push(format!(
                        "{}: restore did not verify changed={} verified={}",
                        definition.id, result.changed, result.verified
                    )),
                    Err(error) => failures.push(format!(
                        "{}: restore failed with {:?}",
                        definition.id,
                        error.code()
                    )),
                }
            } else if let Some(diagnostic) = after.diagnostic {
                failures.push(format!(
                    "{}: post-change read {diagnostic:?}",
                    definition.id
                ));
                // If the adapter failed after persisting the write, the desired value is the only
                // safe expected value available. A conflict leaves an untouched original value
                // alone, while a match restores the exact baseline captured before the test.
                let _ = platform.change_system_setting(&PlatformSystemSettingChangeRequest {
                    setting_id: definition.id.to_string(),
                    expected_value: desired.clone(),
                    desired_value: original.clone(),
                });
            }

            let restored = platform
                .scan_system_settings(&[definition.id], &cancellation)
                .expect("a restored Windows setting read should complete")
                .into_iter()
                .next()
                .expect("a restored Windows setting state should exist");
            if restored.diagnostic.is_some() || restored.value != original {
                failures.push(format!(
                    "{}: original value was not restored",
                    definition.id
                ));
            }
            tested_count += 1;
        }

        eprintln!(
            "windows_system_settings_roundtrip_finished tested_count={tested_count} unsupported_count={unsupported_count} failed_count={}",
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "Windows setting roundtrip failures:\n{}",
            failures.join("\n")
        );
    }
}
