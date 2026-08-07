use serde::{Deserialize, Serialize};

use crate::{
    applications::{
        leftovers::ApplicationLeftoverActionResult,
        uninstall::{
            ApplicationUninstallActionResult, ApplicationUninstallInstallerKind,
            ApplicationUninstallPlatform,
        },
    },
    cleanup::CleanupActionResult,
};

pub const OPERATION_RECORD_SCHEMA_VERSION: u32 = 2;

/// Identifies the product feature that initiated an operation.
///
/// History intentionally follows the four user-facing entry points instead of
/// exposing internal executors such as application-leftover cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationCategory {
    DeepCleanup,
    LargeFileCleanup,
    DuplicateFileCleanup,
    ApplicationUninstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationOutcome {
    Completed,
    CompletedWithWarnings,
    Cancelled,
}

impl OperationOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::CompletedWithWarnings => "completed_with_warnings",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOperationDetails {
    pub selected_rule_ids: Vec<String>,
    pub expected_bytes: u64,
    pub actions: Vec<CleanupActionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationLeftoverOperationDetails {
    pub candidate_ids: Vec<String>,
    pub expected_bytes: u64,
    pub actions: Vec<ApplicationLeftoverActionResult>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepCleanupOperationDetails {
    pub cleanup: Option<CleanupOperationDetails>,
    pub application_leftovers: Option<ApplicationLeftoverOperationDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCleanupOperationDetails {
    pub items: Vec<FileCleanupHistoryItem>,
    pub omitted_item_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileCleanupHistoryItemStatus {
    Deleted,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCleanupHistoryItem {
    pub path: String,
    pub status: FileCleanupHistoryItemStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationUninstallApplicationDetails {
    pub restart_required: bool,
    pub plan_id: String,
    pub application_id: String,
    pub application_name: String,
    pub application_identifier: String,
    pub application_version: Option<String>,
    pub application_publisher: Option<String>,
    pub application_platform: ApplicationUninstallPlatform,
    pub installer_kind: Option<ApplicationUninstallInstallerKind>,
    pub component_ids: Vec<String>,
    pub actions: Vec<ApplicationUninstallActionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationUninstallOperationDetails {
    pub batch_id: String,
    pub applications: Vec<ApplicationUninstallApplicationDetails>,
    pub restart_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum OperationDetails {
    DeepCleanup(DeepCleanupOperationDetails),
    LargeFileCleanup(FileCleanupOperationDetails),
    DuplicateFileCleanup(FileCleanupOperationDetails),
    ApplicationUninstall(ApplicationUninstallOperationDetails),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    pub schema_version: u32,
    pub operation_id: String,
    pub category: OperationCategory,
    pub started_at_ms: u64,
    pub finished_at_ms: u64,
    pub outcome: OperationOutcome,
    pub dry_run: bool,
    pub selected_item_count: u64,
    pub affected_item_count: u64,
    pub expected_bytes: u64,
    pub released_bytes: Option<u64>,
    pub released_bytes_is_estimate: bool,
    pub failed_item_count: u64,
    pub details: OperationDetails,
}
