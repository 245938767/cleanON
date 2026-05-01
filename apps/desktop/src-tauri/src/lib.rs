use serde::{Deserialize, Serialize};
use smart_file_organizer_core::{
    ClassificationContext, ExecutionBatch, FileCategory, FileRiskLevel, OrganizationMode,
    OrganizationPlan, ScanOptions, Skill, SkillUpdateProposal, UserApproval,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanFolderRequest {
    pub task_id: Option<String>,
    pub root_path: String,
    pub recursive: bool,
    pub max_depth: Option<usize>,
    pub include_hidden: bool,
    pub follow_symlinks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScanFolderStatus {
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileItemDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub size_label: String,
    pub modified_at: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanFolderResponse {
    pub task_id: String,
    pub root_path: String,
    pub files: Vec<FileItemDto>,
    pub status: ScanFolderStatus,
    pub skipped_count: usize,
    pub error_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelScanResponse {
    pub task_id: String,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClassifyFilesRequest {
    pub task_id: String,
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationResultDto {
    pub file_id: String,
    pub category: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub risk: String,
}

#[derive(Default)]
pub struct ScanRegistry {
    tasks: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl ScanRegistry {
    fn register(&self, task_id: &str) -> Result<Arc<AtomicBool>, String> {
        let mut tasks = self
            .tasks
            .lock()
            .map_err(|_| "scan registry lock poisoned".to_string())?;
        let flag = tasks
            .entry(task_id.to_string())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone();
        Ok(flag)
    }

    fn cancel(&self, task_id: &str) -> Result<bool, String> {
        let cancelled = self
            .tasks
            .lock()
            .map_err(|_| "scan registry lock poisoned".to_string())?
            .get(task_id)
            .map(|flag| {
                flag.store(true, Ordering::SeqCst);
                true
            })
            .unwrap_or(false);
        Ok(cancelled)
    }

    fn finish(&self, task_id: &str) -> Result<(), String> {
        self.tasks
            .lock()
            .map_err(|_| "scan registry lock poisoned".to_string())?
            .remove(task_id);
        Ok(())
    }
}

#[tauri::command]
fn select_scan_folder(app: AppHandle) -> Result<Option<String>, String> {
    Ok(app
        .dialog()
        .file()
        .blocking_pick_folder()
        .map(|path| path.to_string()))
}

#[tauri::command]
async fn scan_folder(
    app: AppHandle,
    registry: State<'_, ScanRegistry>,
    request: ScanFolderRequest,
) -> Result<ScanFolderResponse, String> {
    let storage = open_storage(&app)?;
    scan_folder_with_storage(&storage, &registry, request)
}

#[tauri::command]
fn cancel_scan(
    registry: State<'_, ScanRegistry>,
    task_id: String,
) -> Result<CancelScanResponse, String> {
    let cancelled = registry.cancel(&task_id)?;
    Ok(CancelScanResponse { task_id, cancelled })
}

#[tauri::command]
async fn classify_files(
    app: AppHandle,
    request: ClassifyFilesRequest,
) -> Result<Vec<ClassificationResultDto>, String> {
    let (files, context) = {
        let storage = open_storage(&app)?;
        load_classification_input(&storage, request)?
    };
    classify_loaded_files(files, context).await
}

#[tauri::command]
async fn generate_plan(
    task_id: String,
    root_path: String,
    classifications: Vec<smart_file_organizer_core::ClassificationResult>,
    mode: OrganizationMode,
) -> Result<OrganizationPlan, String> {
    use smart_file_organizer_planner::{DefaultPlanBuilder, PlanBuilder};

    DefaultPlanBuilder
        .build_plan(smart_file_organizer_core::BuildPlanInput {
            task_id,
            root_path: root_path.into(),
            mode,
            classifications,
        })
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn review_plan(plan: OrganizationPlan) -> Result<OrganizationPlan, String> {
    Ok(plan)
}

#[tauri::command]
async fn execute_confirmed_plan(
    plan: OrganizationPlan,
    approval: UserApproval,
) -> Result<ExecutionBatch, String> {
    use smart_file_organizer_executor::{DefaultPlanExecutor, PlanExecutor};

    let executor = DefaultPlanExecutor;
    executor
        .execute_confirmed(&plan, &approval)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn rollback_batch(
    batch: ExecutionBatch,
) -> Result<smart_file_organizer_core::RollbackResult, String> {
    use smart_file_organizer_executor::{DefaultPlanExecutor, PlanExecutor};

    DefaultPlanExecutor
        .rollback_batch(&batch)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_skills(app: AppHandle) -> Result<Vec<Skill>, String> {
    let storage = open_storage(&app)?;
    Ok(storage
        .list_enabled_skills()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(stored_skill_to_core)
        .collect())
}

#[tauri::command]
async fn save_skill(app: AppHandle, proposal: SkillUpdateProposal) -> Result<Skill, String> {
    let storage = open_storage(&app)?;
    let id = Uuid::new_v4();
    let rule_json: serde_json::Value =
        serde_json::from_str(&proposal.rule).map_err(|error| error.to_string())?;
    storage
        .upsert_skill(&smart_file_organizer_storage::StoredSkill {
            id: id.to_string(),
            name: proposal.name.clone(),
            enabled: proposal.enabled,
            rule_json,
        })
        .map_err(|error| error.to_string())?;

    Ok(Skill {
        id,
        name: proposal.name,
        enabled: proposal.enabled,
        rule: proposal.rule,
        created_at: chrono::Utc::now(),
    })
}

pub fn run() {
    tauri::Builder::default()
        .manage(ScanRegistry::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            select_scan_folder,
            scan_folder,
            cancel_scan,
            classify_files,
            generate_plan,
            review_plan,
            execute_confirmed_plan,
            rollback_batch,
            list_skills,
            save_skill
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

fn scan_folder_with_storage(
    storage: &smart_file_organizer_storage::Storage,
    registry: &ScanRegistry,
    request: ScanFolderRequest,
) -> Result<ScanFolderResponse, String> {
    let task_id = request
        .task_id
        .unwrap_or_else(|| format!("scan-{}", Uuid::new_v4()));
    let root = PathBuf::from(&request.root_path);
    let cancel_flag = registry.register(&task_id)?;

    storage
        .create_scan_task(&task_id, &root, "files", std::env::consts::OS, "running")
        .map_err(|error| error.to_string())?;

    let scan_result = smart_file_organizer_scanner::scan_with_cancellation(
        ScanOptions {
            root: root.clone(),
            recursive: request.recursive,
            max_depth: request.max_depth,
            include_hidden: request.include_hidden,
            follow_symlinks: request.follow_symlinks,
        },
        || cancel_flag.load(Ordering::SeqCst),
    )
    .map_err(|error| error.to_string());

    let response = match scan_result {
        Ok(report) => {
            let status = match report.status {
                smart_file_organizer_scanner::ScanStatus::Completed => {
                    storage
                        .upsert_file_items(&report.files)
                        .map_err(|error| error.to_string())?;
                    storage
                        .update_scan_task_status(&task_id, "completed")
                        .map_err(|error| error.to_string())?;
                    ScanFolderStatus::Completed
                }
                smart_file_organizer_scanner::ScanStatus::Cancelled => {
                    storage
                        .update_scan_task_status(&task_id, "cancelled")
                        .map_err(|error| error.to_string())?;
                    ScanFolderStatus::Cancelled
                }
            };

            Ok(ScanFolderResponse {
                task_id: task_id.clone(),
                root_path: path_to_string(&root),
                files: report.files.iter().map(file_to_dto).collect(),
                status,
                skipped_count: report.skipped_count,
                error_count: report.error_count,
            })
        }
        Err(error) => {
            let _ = storage.update_scan_task_status(&task_id, "failed");
            Err(error)
        }
    };

    let _ = registry.finish(&task_id);
    response
}

fn load_classification_input(
    storage: &smart_file_organizer_storage::Storage,
    request: ClassifyFilesRequest,
) -> Result<
    (
        Vec<smart_file_organizer_core::FileItem>,
        ClassificationContext,
    ),
    String,
> {
    let root_path = PathBuf::from(&request.root_path);
    let mut files = storage
        .list_files_for_task(&request.task_id)
        .map_err(|error| error.to_string())?;
    if files.is_empty() {
        files = storage
            .list_files_for_root(&root_path)
            .map_err(|error| error.to_string())?;
    }

    let skills = storage
        .list_enabled_skills()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(stored_skill_to_core)
        .collect();
    let context = ClassificationContext {
        root_path,
        existing_folders: Vec::new(),
        skills,
        rules: Vec::new(),
        use_ai: false,
    };
    Ok((files, context))
}

async fn classify_loaded_files(
    files: Vec<smart_file_organizer_core::FileItem>,
    context: ClassificationContext,
) -> Result<Vec<ClassificationResultDto>, String> {
    use smart_file_organizer_classifier::{BasicClassifier, Classifier};

    let classifier = BasicClassifier;
    let mut results = Vec::with_capacity(files.len());
    for file in files {
        let result = classifier
            .classify(&file, &context)
            .await
            .map_err(|error| error.to_string())?;
        results.push(classification_to_dto(result));
    }
    Ok(results)
}

fn stored_skill_to_core(skill: smart_file_organizer_storage::StoredSkill) -> Skill {
    Skill {
        id: Uuid::parse_str(&skill.id).unwrap_or_else(|_| Uuid::new_v4()),
        name: skill.name,
        enabled: skill.enabled,
        rule: skill.rule_json.to_string(),
        created_at: chrono::Utc::now(),
    }
}

fn classification_to_dto(
    result: smart_file_organizer_core::ClassificationResult,
) -> ClassificationResultDto {
    ClassificationResultDto {
        file_id: result.file.id.to_string(),
        category: category_label(&result.category).to_string(),
        confidence: result.confidence,
        evidence: result.evidence,
        risk: risk_label(&result.risk).to_string(),
    }
}

fn category_label(category: &FileCategory) -> &'static str {
    match category {
        FileCategory::Documents => "文档",
        FileCategory::Images => "图片",
        FileCategory::Videos => "视频",
        FileCategory::Audio => "音频",
        FileCategory::Archives => "压缩包",
        FileCategory::Installers => "安装包",
        FileCategory::Code => "代码",
        FileCategory::Spreadsheets => "表格",
        FileCategory::Presentations => "演示文稿",
        FileCategory::Pdf => "PDF",
        FileCategory::Other => "其他",
    }
}

fn risk_label(risk: &FileRiskLevel) -> &'static str {
    match risk {
        FileRiskLevel::Low => "low",
        FileRiskLevel::Medium => "medium",
        FileRiskLevel::High => "high",
    }
}

fn open_storage(app: &AppHandle) -> Result<smart_file_organizer_storage::Storage, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|error| error.to_string())?;
    smart_file_organizer_storage::Storage::open(data_dir.join("smart-file-organizer.sqlite3"))
        .map_err(|error| error.to_string())
}

fn file_to_dto(file: &smart_file_organizer_core::FileItem) -> FileItemDto {
    FileItemDto {
        id: file.id.to_string(),
        name: file.file_name.clone(),
        path: path_to_string(&file.path),
        extension: file.extension.clone(),
        size_bytes: file.size_bytes,
        size_label: format_size(file.size_bytes),
        modified_at: file.modified_at.map(|value| value.to_rfc3339()),
        kind: file
            .extension
            .as_ref()
            .map(|extension| format!(".{extension}"))
            .unwrap_or_else(|| "无扩展名".to_string()),
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn format_size(size_bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size = size_bytes as f64;
    if size >= GB {
        format!("{:.1} GB", size / GB)
    } else if size >= MB {
        format!("{:.1} MB", size / MB)
    } else if size >= KB {
        format!("{:.1} KB", size / KB)
    } else {
        format!("{size_bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_command_persists_files_and_returns_dto() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("a.pdf"), b"pdf").unwrap();
        let storage = smart_file_organizer_storage::Storage::in_memory().unwrap();
        let registry = ScanRegistry::default();

        let response = scan_folder_with_storage(
            &storage,
            &registry,
            ScanFolderRequest {
                task_id: Some("task-1".to_string()),
                root_path: path_to_string(temp.path()),
                recursive: true,
                max_depth: None,
                include_hidden: false,
                follow_symlinks: false,
            },
        )
        .unwrap();

        assert_eq!(response.status, ScanFolderStatus::Completed);
        assert_eq!(response.files.len(), 1);
        assert_eq!(response.files[0].extension, Some("pdf".to_string()));
        assert_eq!(storage.count_file_items().unwrap(), 1);
    }

    #[test]
    fn cancelled_scan_does_not_persist_partial_files() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("a.pdf"), b"pdf").unwrap();
        let storage = smart_file_organizer_storage::Storage::in_memory().unwrap();
        let registry = ScanRegistry::default();
        let task_id = "task-cancelled";
        let cancel_flag = registry.register(task_id).unwrap();
        cancel_flag.store(true, Ordering::SeqCst);

        let response = scan_folder_with_storage(
            &storage,
            &registry,
            ScanFolderRequest {
                task_id: Some(task_id.to_string()),
                root_path: path_to_string(temp.path()),
                recursive: true,
                max_depth: None,
                include_hidden: false,
                follow_symlinks: false,
            },
        )
        .unwrap();

        assert_eq!(response.status, ScanFolderStatus::Cancelled);
        assert!(response.files.is_empty());
        assert_eq!(storage.count_file_items().unwrap(), 0);
    }

    #[tokio::test]
    async fn classify_command_reads_scanned_files_from_storage() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("安装包.dmg"), b"dmg").unwrap();
        fs::write(temp.path().join("legacy.jpg"), b"jpg").unwrap();
        let storage = smart_file_organizer_storage::Storage::in_memory().unwrap();
        let registry = ScanRegistry::default();
        storage
            .upsert_skill(&smart_file_organizer_storage::StoredSkill {
                id: "legacy-skill-id".to_string(),
                name: "旧版图片归文档".to_string(),
                enabled: true,
                rule_json: serde_json::json!({
                    "extension": "jpg",
                    "category": "Documents"
                }),
            })
            .unwrap();

        scan_folder_with_storage(
            &storage,
            &registry,
            ScanFolderRequest {
                task_id: Some("task-classify".to_string()),
                root_path: path_to_string(temp.path()),
                recursive: true,
                max_depth: None,
                include_hidden: false,
                follow_symlinks: false,
            },
        )
        .unwrap();

        let (files, context) = load_classification_input(
            &storage,
            ClassifyFilesRequest {
                task_id: "task-classify".to_string(),
                root_path: path_to_string(temp.path()),
            },
        )
        .unwrap();
        let results = classify_loaded_files(files, context).await.unwrap();

        assert_eq!(results.len(), 2);
        let installer = results
            .iter()
            .find(|result| result.category == "安装包")
            .unwrap();
        assert!(installer
            .evidence
            .iter()
            .any(|evidence| evidence.contains("扩展名")));
        let skill_result = results
            .iter()
            .find(|result| result.category == "文档")
            .unwrap();
        assert!(skill_result
            .evidence
            .iter()
            .any(|evidence| evidence.contains("Skill")));
    }
}
