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
    pub rule: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillUpdateProposal {
    pub name: String,
    pub rule: String,
    pub enabled: bool,
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
