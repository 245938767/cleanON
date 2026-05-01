use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileItem {
    pub id: Uuid,
    pub root: PathBuf,
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub file_name: String,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: Option<DateTime<Utc>>,
    pub accessed_at: Option<DateTime<Utc>>,
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub mime_type: Option<String>,
    pub path_hash: String,
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanOptions {
    pub root: PathBuf,
    pub recursive: bool,
    pub max_depth: Option<usize>,
    pub include_hidden: bool,
    pub follow_symlinks: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClassifierOptions {
    pub prefer_extension_rules: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassificationContext {
    pub root_path: PathBuf,
    pub existing_folders: Vec<PathBuf>,
    pub skills: Vec<Skill>,
    pub rules: Vec<ClassificationRule>,
    pub use_ai: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassificationRule {
    pub rule_id: String,
    pub name: String,
    pub priority: i32,
    pub enabled: bool,
    pub conditions: Vec<RuleCondition>,
    pub target_category: FileCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleCondition {
    pub field: RuleField,
    pub operator: RuleOperator,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleField {
    FileName,
    Extension,
    MimeType,
    RelativePath,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleOperator {
    Equals,
    Contains,
    StartsWith,
    EndsWith,
    In,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileCategory {
    Documents,
    Images,
    Videos,
    Audio,
    Archives,
    Installers,
    Code,
    Spreadsheets,
    Presentations,
    Pdf,
    Other,
}

impl Default for FileCategory {
    fn default() -> Self {
        Self::Other
    }
}

impl FileCategory {
    pub fn folder_name(&self) -> &'static str {
        match self {
            Self::Documents => "Documents",
            Self::Images => "Images",
            Self::Videos => "Videos",
            Self::Audio => "Audio",
            Self::Archives => "Archives",
            Self::Installers => "Installers",
            Self::Code => "Code",
            Self::Spreadsheets => "Spreadsheets",
            Self::Presentations => "Presentations",
            Self::Pdf => "PDF",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileRiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassificationResult {
    pub file: FileItem,
    pub category: FileCategory,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub risk: FileRiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrganizationMode {
    ByCategory,
    ByExtension,
    Desktop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildPlanInput {
    pub task_id: String,
    pub root_path: PathBuf,
    pub mode: OrganizationMode,
    pub classifications: Vec<ClassificationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratePlanRequestDto {
    pub task_id: String,
    pub root_path: String,
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classifications: Option<Vec<ClassificationInputDto>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationInputDto {
    pub file_id: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_key: Option<String>,
    pub confidence: f32,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrganizationPlan {
    pub plan_id: Uuid,
    pub task_id: String,
    pub root_path: PathBuf,
    pub mode: OrganizationMode,
    pub operations: Vec<FileOperationPlan>,
    pub summary: PlanSummary,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationPlanDto {
    pub plan_id: String,
    pub task_id: String,
    pub root_path: String,
    pub mode: String,
    pub rows: Vec<OperationRowDto>,
    pub summary: PlanSummaryDto,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_preview: Option<DesktopPreviewDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummaryDto {
    pub files_considered: usize,
    pub folders_to_create: usize,
    pub files_to_move: usize,
    pub files_to_rename: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationRowDto {
    pub operation_id: String,
    pub operation_type: String,
    pub title: String,
    pub source: Option<String>,
    pub target: String,
    pub reason: String,
    pub risk: String,
    pub selected: bool,
    pub editable_target: String,
    #[serde(default)]
    pub validation_issues: Vec<ValidationIssueDto>,
    pub conflict_status: String,
    pub file_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssueDto {
    pub operation_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanSummary {
    pub files_considered: usize,
    pub folders_to_create: usize,
    pub files_to_move: usize,
    pub files_to_rename: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileOperationPlan {
    pub operation_id: Uuid,
    pub kind: FileOperationKind,
    pub reason: String,
    pub file_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileOperationKind {
    CreateFolder {
        path: PathBuf,
    },
    MoveFile {
        source: PathBuf,
        destination: PathBuf,
    },
    RenameFile {
        source: PathBuf,
        destination: PathBuf,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPreviewDto {
    pub platform: DesktopPreviewPlatform,
    pub capabilities: DesktopCapabilityFlagsDto,
    pub canvas: DesktopPreviewCanvasDto,
    pub before_groups: Vec<DesktopPreviewGroupDto>,
    pub after_zones: Vec<DesktopPreviewZoneDto>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DesktopPreviewPlatform {
    Macos,
    Windows,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCapabilityFlagsDto {
    pub preview_only: bool,
    pub supports_file_archive_plan: bool,
    pub supports_desktop_canvas_preview: bool,
    pub supports_icon_coordinate_writeback: bool,
    pub supports_pixel_perfect_layout: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPreviewCanvasDto {
    pub width: u32,
    pub height: u32,
    pub columns: u32,
    pub rows: u32,
    pub coordinate_space: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPreviewGroupDto {
    pub group_id: String,
    pub title: String,
    pub category_key: String,
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub files: Vec<DesktopPreviewFileDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPreviewFileDto {
    pub file_id: String,
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPreviewZoneDto {
    pub zone_id: String,
    pub title: String,
    pub category_key: String,
    pub archive_folder: String,
    pub file_count: usize,
    pub canvas_rect: DesktopPreviewRectDto,
    pub file_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPreviewRectDto {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn current_desktop_preview_platform() -> DesktopPreviewPlatform {
    match std::env::consts::OS {
        "macos" => DesktopPreviewPlatform::Macos,
        "windows" => DesktopPreviewPlatform::Windows,
        _ => DesktopPreviewPlatform::Other,
    }
}

pub fn desktop_capability_flags(platform: DesktopPreviewPlatform) -> DesktopCapabilityFlagsDto {
    DesktopCapabilityFlagsDto {
        preview_only: true,
        supports_file_archive_plan: true,
        supports_desktop_canvas_preview: matches!(platform, DesktopPreviewPlatform::Windows),
        supports_icon_coordinate_writeback: false,
        supports_pixel_perfect_layout: false,
    }
}

pub fn desktop_archive_folder(root: &std::path::Path, category: &FileCategory) -> PathBuf {
    root.join("Desktop Archive").join(category.folder_name())
}

pub fn build_desktop_preview_dto(
    platform: DesktopPreviewPlatform,
    desktop_root: &std::path::Path,
    classifications: &[ClassificationResult],
) -> DesktopPreviewDto {
    let before_groups = desktop_preview_groups(classifications);
    let canvas = desktop_preview_canvas(platform);
    let after_zones = before_groups
        .iter()
        .enumerate()
        .map(|(index, group)| desktop_preview_zone(index, group, &canvas, desktop_root))
        .collect();

    DesktopPreviewDto {
        platform,
        capabilities: desktop_capability_flags(platform),
        canvas,
        before_groups,
        after_zones,
    }
}

fn desktop_preview_canvas(platform: DesktopPreviewPlatform) -> DesktopPreviewCanvasDto {
    match platform {
        DesktopPreviewPlatform::Windows => DesktopPreviewCanvasDto {
            width: 1200,
            height: 760,
            columns: 4,
            rows: 3,
            coordinate_space: "preview_canvas".to_string(),
        },
        DesktopPreviewPlatform::Macos | DesktopPreviewPlatform::Other => DesktopPreviewCanvasDto {
            width: 1000,
            height: 640,
            columns: 3,
            rows: 2,
            coordinate_space: "preview_canvas".to_string(),
        },
    }
}

fn desktop_preview_groups(classifications: &[ClassificationResult]) -> Vec<DesktopPreviewGroupDto> {
    let mut groups = Vec::<DesktopPreviewGroupDto>::new();

    for classification in classifications {
        let key = desktop_category_key(&classification.category);
        let file = DesktopPreviewFileDto {
            file_id: classification.file.id.to_string(),
            name: classification.file.file_name.clone(),
            path: classification.file.path.to_string_lossy().into_owned(),
            size_bytes: classification.file.size_bytes,
        };

        match groups.iter().position(|group| group.category_key == key) {
            Some(index) => {
                let group = &mut groups[index];
                group.file_count += 1;
                group.total_size_bytes += file.size_bytes;
                group.files.push(file);
            }
            None => groups.push(DesktopPreviewGroupDto {
                group_id: format!("before-{key}"),
                title: classification.category.folder_name().to_string(),
                category_key: key.to_string(),
                file_count: 1,
                total_size_bytes: file.size_bytes,
                files: vec![file],
            }),
        }
    }

    groups
}

fn desktop_preview_zone(
    index: usize,
    group: &DesktopPreviewGroupDto,
    canvas: &DesktopPreviewCanvasDto,
    desktop_root: &std::path::Path,
) -> DesktopPreviewZoneDto {
    let columns = canvas.columns.max(1);
    let rows = canvas.rows.max(1);
    let cell_width = canvas.width / columns;
    let cell_height = canvas.height / rows;
    let column = index as u32 % columns;
    let row = (index as u32 / columns).min(rows - 1);

    DesktopPreviewZoneDto {
        zone_id: format!("zone-{}", group.category_key),
        title: group.title.clone(),
        category_key: group.category_key.clone(),
        archive_folder: desktop_root
            .join("Desktop Archive")
            .join(&group.title)
            .to_string_lossy()
            .into_owned(),
        file_count: group.file_count,
        canvas_rect: DesktopPreviewRectDto {
            x: column * cell_width,
            y: row * cell_height,
            width: cell_width,
            height: cell_height,
        },
        file_ids: group
            .files
            .iter()
            .map(|file| file.file_id.clone())
            .collect(),
    }
}

fn desktop_category_key(category: &FileCategory) -> &'static str {
    match category {
        FileCategory::Documents => "documents",
        FileCategory::Images => "images",
        FileCategory::Videos => "videos",
        FileCategory::Audio => "audio",
        FileCategory::Archives => "archives",
        FileCategory::Installers => "installers",
        FileCategory::Code => "code",
        FileCategory::Spreadsheets => "spreadsheets",
        FileCategory::Presentations => "presentations",
        FileCategory::Pdf => "pdf",
        FileCategory::Other => "other",
    }
}

impl FileOperationKind {
    pub fn source(&self) -> Option<&PathBuf> {
        match self {
            Self::CreateFolder { .. } => None,
            Self::MoveFile { source, .. } | Self::RenameFile { source, .. } => Some(source),
        }
    }

    pub fn destination(&self) -> &PathBuf {
        match self {
            Self::CreateFolder { path } => path,
            Self::MoveFile { destination, .. } | Self::RenameFile { destination, .. } => {
                destination
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserApproval {
    pub approved: bool,
    pub approved_plan_id: Uuid,
    pub approved_at: DateTime<Utc>,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserApprovalDto {
    pub approved: bool,
    pub approved_plan_id: String,
    pub approved_at: String,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanValidation {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

impl PlanValidation {
    pub fn ok() -> Self {
        Self {
            valid: true,
            issues: Vec::new(),
        }
    }

    pub fn from_issues(issues: Vec<ValidationIssue>) -> Self {
        Self {
            valid: issues.is_empty(),
            issues,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationIssue {
    pub operation_id: Option<Uuid>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Completed,
    PartiallyFailed,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionBatch {
    pub batch_id: Uuid,
    pub plan_id: Uuid,
    pub status: ExecutionStatus,
    pub executed_operations: Vec<ExecutedOperation>,
    pub rollback_entries: Vec<RollbackEntry>,
    pub errors: Vec<ExecutionError>,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionBatchDto {
    pub batch_id: String,
    pub plan_id: String,
    pub status: String,
    pub executed_operations: Vec<ExecutedOperationDto>,
    pub rollback_entries: Vec<RollbackEntryDto>,
    pub errors: Vec<ExecutionErrorDto>,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutedOperationDto {
    pub operation_id: String,
    pub operation_type: String,
    pub source: Option<String>,
    pub target: String,
    pub completed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionErrorDto {
    pub operation_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutedOperation {
    pub operation_id: Uuid,
    pub kind: FileOperationKind,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionError {
    pub operation_id: Option<Uuid>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RollbackEntry {
    pub batch_id: Uuid,
    pub operation_id: Uuid,
    pub action: RollbackAction,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RollbackEntryDto {
    pub batch_id: String,
    pub operation_id: String,
    pub action: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistorySummaryDto {
    pub batch_id: String,
    pub plan_id: String,
    pub status: String,
    pub operation_count: usize,
    pub error_count: usize,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RollbackAction {
    RemoveCreatedFolder { path: PathBuf },
    MoveFileBack { from: PathBuf, to: PathBuf },
    RenameFileBack { from: PathBuf, to: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RollbackResult {
    pub batch_id: Uuid,
    pub rolled_back: Vec<Uuid>,
    pub errors: Vec<ExecutionError>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillQuery {
    pub root_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Skill {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub rule: SkillRule,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillRule {
    #[serde(default)]
    pub extension: Option<String>,
    #[serde(default, alias = "file_name_contains")]
    pub file_name_contains: Option<String>,
    #[serde(default, alias = "mime_prefix")]
    pub mime_prefix: Option<String>,
    #[serde(default)]
    pub category: FileCategory,
    #[serde(default, alias = "destination_hint")]
    pub destination_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillDto {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub rule: SkillRule,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillUpdateProposal {
    pub name: String,
    pub rule: SkillRule,
    pub enabled: bool,
    pub evidence: Vec<String>,
    pub source_event_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateProposalDto {
    pub name: String,
    pub rule: SkillRule,
    pub enabled: bool,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub source_event_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserDecisionEvent {
    pub event_id: Uuid,
    pub file_name: String,
    pub extension: Option<String>,
    pub mime_type: Option<String>,
    pub decision: UserDecision,
    pub original_category: Option<FileCategory>,
    pub final_category: Option<FileCategory>,
    pub original_destination: Option<PathBuf>,
    pub final_destination: Option<PathBuf>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserDecision {
    Accepted,
    Rejected,
    EditedDestination,
    RenamedFolder,
    RenamedCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettingsDto {
    pub provider: String,
    pub cloud_enabled: bool,
    pub model: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OrganizerError {
    #[error("approval is required before executing a plan")]
    ApprovalRequired,
    #[error("approval does not match plan id")]
    ApprovalPlanMismatch,
    #[error("plan validation failed: {0}")]
    ValidationFailed(String),
    #[error("{0}")]
    Message(String),
}
