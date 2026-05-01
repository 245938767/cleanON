use serde::{Deserialize, Serialize};
use smart_file_organizer_core::{
    ClassificationContext, ClassificationInputDto, ExecutedOperationDto, ExecutionBatch,
    ExecutionBatchDto, FileCategory, FileOperationKind, FileOperationPlan, FileRiskLevel,
    GeneratePlanRequestDto, HistorySummaryDto, OperationRowDto, OrganizationMode, OrganizationPlan,
    OrganizationPlanDto, PlanSummary, PlanSummaryDto, RollbackEntryDto, RollbackResult,
    ScanOptions, Skill, SkillDto, SkillUpdateProposal, SkillUpdateProposalDto, UserApproval,
    UserApprovalDto, UserDecisionEvent, ValidationIssueDto,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelProviderDto {
    pub provider: String,
    pub label: String,
    pub requires_base_url: bool,
    pub requires_api_key: bool,
    pub cloud: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettingsDto {
    pub provider: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub cloud_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettingsListDto {
    pub providers: Vec<ModelProviderDto>,
    pub saved_settings: Vec<ModelSettingsDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TestModelRequest {
    pub settings: ModelSettingsDto,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TestModelConnectionResponse {
    pub provider: String,
    pub request_valid: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TestModelJsonOutputResponse {
    pub provider: String,
    pub valid: bool,
    pub summary: String,
    pub categories_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationResultDto {
    pub file_id: String,
    pub category: String,
    pub category_key: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlanPatchDto {
    #[serde(default)]
    pub operations: Vec<OperationPatchDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationPatchDto {
    pub operation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editable_target: Option<String>,
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
    app: AppHandle,
    request: GeneratePlanRequestDto,
) -> Result<OrganizationPlanDto, String> {
    let prepared = {
        let storage = open_storage(&app)?;
        prepare_generate_plan(&storage, request)?
    };
    let dto = build_prepared_plan(prepared).await?;
    {
        let storage = open_storage(&app)?;
        storage
            .save_plan(
                &dto.plan_id,
                &serde_json::to_value(&dto).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(dto)
}

#[tauri::command]
async fn review_plan(plan: OrganizationPlanDto) -> Result<OrganizationPlanDto, String> {
    Ok(plan)
}

#[tauri::command]
async fn patch_plan(
    app: AppHandle,
    plan: OrganizationPlanDto,
    patch: PlanPatchDto,
) -> Result<OrganizationPlanDto, String> {
    let patched = apply_plan_patch_with_validation(plan, patch).await?;
    let storage = open_storage(&app)?;
    storage
        .save_plan(
            &patched.plan_id,
            &serde_json::to_value(&patched).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    Ok(patched)
}

#[tauri::command]
async fn execute_confirmed_plan(
    app: AppHandle,
    plan: OrganizationPlanDto,
    approval: UserApprovalDto,
) -> Result<ExecutionBatchDto, String> {
    use smart_file_organizer_executor::{DefaultPlanExecutor, PlanExecutor};

    let plan = plan_dto_to_core(plan)?;
    let approval = approval_dto_to_core(approval)?;
    let executor = DefaultPlanExecutor;
    let batch = executor
        .execute_confirmed(&plan, &approval)
        .await
        .map_err(|error| error.to_string())?;
    let dto = execution_batch_to_dto(&batch);
    let storage = open_storage(&app)?;
    storage
        .record_execution_batch(
            &dto.batch_id,
            &dto.plan_id,
            &dto.status,
            &serde_json::to_value(&dto).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    Ok(dto)
}

#[tauri::command]
async fn rollback_batch(batch: ExecutionBatch) -> Result<RollbackResult, String> {
    use smart_file_organizer_executor::{DefaultPlanExecutor, PlanExecutor};

    DefaultPlanExecutor
        .rollback_batch(&batch)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_execution_batches(app: AppHandle) -> Result<Vec<HistorySummaryDto>, String> {
    let storage = open_storage(&app)?;
    storage
        .list_execution_batches()
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn load_plan(app: AppHandle, plan_id: String) -> Result<Option<OrganizationPlanDto>, String> {
    let storage = open_storage(&app)?;
    storage
        .load_plan(&plan_id)
        .map_err(|error| error.to_string())?
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn load_execution_batch(
    app: AppHandle,
    batch_id: String,
) -> Result<Option<ExecutionBatchDto>, String> {
    let storage = open_storage(&app)?;
    storage
        .load_execution_batch(&batch_id)
        .map_err(|error| error.to_string())?
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn rollback_batch_by_id(app: AppHandle, batch_id: String) -> Result<RollbackResult, String> {
    use smart_file_organizer_executor::{DefaultPlanExecutor, PlanExecutor};

    let batch = load_execution_batch(app, batch_id)
        .await?
        .ok_or_else(|| "execution batch not found".to_string())
        .and_then(execution_batch_dto_to_core)?;
    DefaultPlanExecutor
        .rollback_batch(&batch)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn list_skills(app: AppHandle) -> Result<Vec<SkillDto>, String> {
    let storage = open_storage(&app)?;
    Ok(storage
        .list_skills()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(stored_skill_to_core)
        .map(|skill| skill_to_dto(&skill))
        .collect())
}

#[tauri::command]
async fn save_skill(app: AppHandle, proposal: SkillUpdateProposalDto) -> Result<SkillDto, String> {
    let storage = open_storage(&app)?;
    let id = Uuid::new_v4();
    storage
        .upsert_skill(&smart_file_organizer_storage::StoredSkill {
            id: id.to_string(),
            name: proposal.name.clone(),
            enabled: proposal.enabled,
            rule: proposal.rule.clone(),
        })
        .map_err(|error| error.to_string())?;

    let skill = Skill {
        id,
        name: proposal.name,
        enabled: proposal.enabled,
        rule: proposal.rule,
        created_at: chrono::Utc::now(),
    };
    Ok(skill_to_dto(&skill))
}

#[tauri::command]
async fn list_model_settings(app: AppHandle) -> Result<ModelSettingsListDto, String> {
    let storage = open_storage(&app)?;
    list_model_settings_with_storage(&storage)
}

#[tauri::command]
async fn save_model_settings(
    app: AppHandle,
    settings: ModelSettingsDto,
) -> Result<ModelSettingsDto, String> {
    let storage = open_storage(&app)?;
    save_model_settings_with_storage(&storage, settings)
}

#[tauri::command]
async fn test_model_connection(
    request: TestModelRequest,
) -> Result<TestModelConnectionResponse, String> {
    let config = model_settings_to_provider_config(&request.settings);
    let credentials = request
        .api_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
        .map(smart_file_organizer_ai_gateway::ProviderCredentials::new);
    let result =
        smart_file_organizer_ai_gateway::test_provider_connection(&config, credentials.as_ref())
            .map_err(|error| error.to_string())?;
    Ok(TestModelConnectionResponse {
        provider: result.provider,
        request_valid: result.request_valid,
        message: result.message,
    })
}

#[tauri::command]
async fn test_model_json_output(
    request: TestModelRequest,
) -> Result<TestModelJsonOutputResponse, String> {
    let config = model_settings_to_provider_config(&request.settings);
    let credentials = request
        .api_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
        .map(smart_file_organizer_ai_gateway::ProviderCredentials::new);
    let sanitized = sample_sanitized_ai_request();
    smart_file_organizer_ai_gateway::build_provider_request(
        &config,
        &sanitized,
        credentials.as_ref(),
    )
    .map_err(|error| error.to_string())?;
    let raw = sample_provider_json_response(&request.settings.provider);
    let suggestion = smart_file_organizer_ai_gateway::parse_provider_response(
        &request.settings.provider,
        &raw,
        &sanitized,
    )
    .map_err(|error| error.to_string())?;
    Ok(TestModelJsonOutputResponse {
        provider: suggestion.provider,
        valid: true,
        summary: suggestion.summary,
        categories_count: suggestion.categories.len(),
    })
}

#[tauri::command]
async fn disable_skill(app: AppHandle, id: String) -> Result<bool, String> {
    let storage = open_storage(&app)?;
    storage
        .disable_skill(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn delete_skill(app: AppHandle, id: String) -> Result<bool, String> {
    let storage = open_storage(&app)?;
    storage.delete_skill(&id).map_err(|error| error.to_string())
}

#[tauri::command]
async fn propose_skill_updates(
    events: Vec<UserDecisionEvent>,
) -> Result<Vec<SkillUpdateProposalDto>, String> {
    Ok(
        smart_file_organizer_skill_engine::propose_skill_updates(&events)
            .into_iter()
            .map(skill_proposal_to_dto)
            .collect(),
    )
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
            patch_plan,
            execute_confirmed_plan,
            rollback_batch,
            list_execution_batches,
            load_plan,
            load_execution_batch,
            rollback_batch_by_id,
            list_skills,
            save_skill,
            list_model_settings,
            save_model_settings,
            test_model_connection,
            test_model_json_output,
            disable_skill,
            delete_skill,
            propose_skill_updates
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
    Ok(classify_loaded_core(files, context)
        .await?
        .into_iter()
        .map(classification_to_dto)
        .collect())
}

async fn classify_loaded_core(
    files: Vec<smart_file_organizer_core::FileItem>,
    context: ClassificationContext,
) -> Result<Vec<smart_file_organizer_core::ClassificationResult>, String> {
    use smart_file_organizer_classifier::{BasicClassifier, Classifier};

    let classifier = BasicClassifier;
    let mut results = Vec::with_capacity(files.len());
    for file in files {
        let result = classifier
            .classify(&file, &context)
            .await
            .map_err(|error| error.to_string())?;
        results.push(result);
    }
    Ok(results)
}

enum PreparedClassifications {
    Ready(Vec<smart_file_organizer_core::ClassificationResult>),
    NeedsClassification(
        Vec<smart_file_organizer_core::FileItem>,
        ClassificationContext,
    ),
}

struct PreparedGeneratePlan {
    task_id: String,
    root_path: PathBuf,
    mode: OrganizationMode,
    classifications: PreparedClassifications,
}

fn prepare_generate_plan(
    storage: &smart_file_organizer_storage::Storage,
    request: GeneratePlanRequestDto,
) -> Result<PreparedGeneratePlan, String> {
    let mode = parse_mode(&request.mode)?;
    let root_path = PathBuf::from(&request.root_path);
    let classifications = match request.classifications {
        Some(classifications) => PreparedClassifications::Ready(classifications_from_dto(
            storage,
            &request.task_id,
            &root_path,
            classifications,
        )?),
        None => {
            let (files, context) = load_classification_input(
                storage,
                ClassifyFilesRequest {
                    task_id: request.task_id.clone(),
                    root_path: request.root_path.clone(),
                },
            )?;
            PreparedClassifications::NeedsClassification(files, context)
        }
    };

    Ok(PreparedGeneratePlan {
        task_id: request.task_id,
        root_path,
        mode,
        classifications,
    })
}

async fn build_prepared_plan(
    prepared: PreparedGeneratePlan,
) -> Result<OrganizationPlanDto, String> {
    use smart_file_organizer_executor::{DefaultPlanExecutor, PlanExecutor};
    use smart_file_organizer_planner::{DefaultPlanBuilder, PlanBuilder};

    let classifications = match prepared.classifications {
        PreparedClassifications::Ready(classifications) => classifications,
        PreparedClassifications::NeedsClassification(files, context) => {
            classify_loaded_core(files, context).await?
        }
    };
    let risk_by_file_id = classifications
        .iter()
        .map(|classification| {
            (
                classification.file.id,
                risk_label(&classification.risk).to_string(),
            )
        })
        .collect::<HashMap<_, _>>();

    let plan = DefaultPlanBuilder
        .build_plan(smart_file_organizer_core::BuildPlanInput {
            task_id: prepared.task_id,
            root_path: prepared.root_path,
            mode: prepared.mode,
            classifications,
        })
        .await
        .map_err(|error| error.to_string())?;

    let validation = DefaultPlanExecutor
        .validate_plan(&plan)
        .await
        .map_err(|error| error.to_string())?;
    Ok(plan_to_dto(&plan, &validation, &risk_by_file_id))
}

async fn apply_plan_patch_with_validation(
    mut plan: OrganizationPlanDto,
    patch: PlanPatchDto,
) -> Result<OrganizationPlanDto, String> {
    use smart_file_organizer_executor::{DefaultPlanExecutor, PlanExecutor};

    validate_supported_plan_rows(&plan)?;
    for operation_patch in patch.operations {
        let row = plan
            .rows
            .iter_mut()
            .find(|row| row.operation_id == operation_patch.operation_id)
            .ok_or_else(|| format!("operation not found: {}", operation_patch.operation_id))?;
        if let Some(selected) = operation_patch.selected {
            row.selected = selected;
        }
        if let Some(editable_target) = operation_patch.editable_target {
            row.editable_target = editable_target;
        }
    }

    let core_plan = plan_dto_to_core(plan.clone())?;
    let validation = DefaultPlanExecutor
        .validate_plan(&core_plan)
        .await
        .map_err(|error| error.to_string())?;
    apply_validation_to_plan_dto(&mut plan, &validation);
    plan.summary = summarize_plan_dto(&plan)?;
    Ok(plan)
}

fn validate_supported_plan_rows(plan: &OrganizationPlanDto) -> Result<(), String> {
    for row in &plan.rows {
        match row.operation_type.as_str() {
            "create_folder" | "move_file" | "rename_file" => {}
            other => return Err(format!("unsupported operation_type: {other}")),
        }
    }
    Ok(())
}

fn apply_validation_to_plan_dto(
    plan: &mut OrganizationPlanDto,
    validation: &smart_file_organizer_core::PlanValidation,
) {
    let mut issues_by_operation = HashMap::<String, Vec<ValidationIssueDto>>::new();
    for issue in &validation.issues {
        if let Some(operation_id) = issue.operation_id {
            issues_by_operation
                .entry(operation_id.to_string())
                .or_default()
                .push(validation_issue_to_dto(issue));
        }
    }

    for row in &mut plan.rows {
        let issues = if row.selected {
            issues_by_operation
                .remove(&row.operation_id)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        row.validation_issues = issues;
        row.conflict_status = if row.selected && !row.validation_issues.is_empty() {
            "blocked".to_string()
        } else {
            "none".to_string()
        };
        if row.selected && !row.validation_issues.is_empty() {
            row.risk = "high".to_string();
        }
    }
}

fn summarize_plan_dto(plan: &OrganizationPlanDto) -> Result<PlanSummaryDto, String> {
    let operations = plan
        .rows
        .iter()
        .filter(|row| row.selected)
        .cloned()
        .map(row_dto_to_operation)
        .collect::<Result<Vec<_>, _>>()?;
    let summary = summarize_operations(plan.summary.files_considered, &operations);
    Ok(PlanSummaryDto {
        files_considered: summary.files_considered,
        folders_to_create: summary.folders_to_create,
        files_to_move: summary.files_to_move,
        files_to_rename: summary.files_to_rename,
    })
}

fn classifications_from_dto(
    storage: &smart_file_organizer_storage::Storage,
    task_id: &str,
    root_path: &Path,
    dtos: Vec<ClassificationInputDto>,
) -> Result<Vec<smart_file_organizer_core::ClassificationResult>, String> {
    let mut files = storage
        .list_files_for_task(task_id)
        .map_err(|error| error.to_string())?;
    if files.is_empty() {
        files = storage
            .list_files_for_root(root_path)
            .map_err(|error| error.to_string())?;
    }
    let files_by_id = files
        .into_iter()
        .map(|file| (file.id.to_string(), file))
        .collect::<HashMap<_, _>>();

    dtos.into_iter()
        .map(|dto| {
            let file = files_by_id
                .get(&dto.file_id)
                .cloned()
                .ok_or_else(|| format!("classification file not found: {}", dto.file_id))?;
            Ok(smart_file_organizer_core::ClassificationResult {
                file,
                category: parse_category(dto.category_key.as_deref().unwrap_or(&dto.category))?,
                confidence: dto.confidence,
                evidence: dto.evidence,
                risk: parse_risk(&dto.risk)?,
            })
        })
        .collect()
}

fn stored_skill_to_core(skill: smart_file_organizer_storage::StoredSkill) -> Skill {
    Skill {
        id: Uuid::parse_str(&skill.id).unwrap_or_else(|_| Uuid::new_v4()),
        name: skill.name,
        enabled: skill.enabled,
        rule: skill.rule,
        created_at: chrono::Utc::now(),
    }
}

fn list_model_settings_with_storage(
    storage: &smart_file_organizer_storage::Storage,
) -> Result<ModelSettingsListDto, String> {
    let providers = smart_file_organizer_ai_gateway::provider_registry()
        .into_iter()
        .map(|provider| ModelProviderDto {
            provider: provider.provider,
            label: provider.label,
            requires_base_url: provider.requires_base_url,
            requires_api_key: provider.requires_api_key,
            cloud: provider.cloud,
        })
        .collect();
    let saved_settings = storage
        .list_ai_provider_settings()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(storage_model_settings_to_dto)
        .collect();
    Ok(ModelSettingsListDto {
        providers,
        saved_settings,
    })
}

fn save_model_settings_with_storage(
    storage: &smart_file_organizer_storage::Storage,
    settings: ModelSettingsDto,
) -> Result<ModelSettingsDto, String> {
    let config = model_settings_to_provider_config(&settings);
    smart_file_organizer_ai_gateway::validate_provider_config(&config)
        .map_err(|error| error.to_string())?;
    storage
        .save_ai_provider_settings(&smart_file_organizer_storage::AiProviderSettings {
            provider: settings.provider.clone(),
            base_url: settings.base_url.clone(),
            cloud_enabled: settings.cloud_enabled,
            model: settings.model.clone(),
        })
        .map_err(|error| error.to_string())?;
    Ok(settings)
}

fn storage_model_settings_to_dto(
    settings: smart_file_organizer_storage::AiProviderSettings,
) -> ModelSettingsDto {
    ModelSettingsDto {
        provider: settings.provider,
        base_url: settings.base_url,
        model: settings.model,
        cloud_enabled: settings.cloud_enabled,
    }
}

fn model_settings_to_provider_config(
    settings: &ModelSettingsDto,
) -> smart_file_organizer_ai_gateway::ProviderConfig {
    smart_file_organizer_ai_gateway::ProviderConfig {
        provider: settings.provider.clone(),
        base_url: settings.base_url.clone(),
        model: settings.model.clone(),
        cloud_enabled: settings.cloud_enabled,
    }
}

fn sample_sanitized_ai_request() -> smart_file_organizer_ai_gateway::SanitizedAiRequest {
    smart_file_organizer_ai_gateway::SanitizedAiRequest {
        prompt: "Return a JSON suggestion for this sanitized file list.".to_string(),
        files: vec![smart_file_organizer_ai_gateway::SanitizedFileInput {
            token: "file_test".to_string(),
            extension: Some("txt".to_string()),
            mime: Some("text/plain".to_string()),
            size_bucket: smart_file_organizer_ai_gateway::SizeBucket::Small,
            path_depth: 1,
        }],
    }
}

fn sample_provider_json_response(provider: &str) -> String {
    let content = serde_json::json!({
        "summary": "JSON output validation passed.",
        "categories": [
            {
                "file_token": "file_test",
                "category": "Documents",
                "confidence": 90
            }
        ]
    })
    .to_string();

    match smart_file_organizer_ai_gateway::parse_provider_kind(provider) {
        Ok(smart_file_organizer_ai_gateway::ProviderKind::OpenAiCompatible) => serde_json::json!({
            "choices": [{"message": {"content": content}}]
        })
        .to_string(),
        Ok(smart_file_organizer_ai_gateway::ProviderKind::Ollama) => {
            serde_json::json!({ "response": content }).to_string()
        }
        _ => content,
    }
}

fn classification_to_dto(
    result: smart_file_organizer_core::ClassificationResult,
) -> ClassificationResultDto {
    ClassificationResultDto {
        file_id: result.file.id.to_string(),
        category: category_label(&result.category).to_string(),
        category_key: category_key(&result.category).to_string(),
        confidence: result.confidence,
        evidence: result.evidence,
        risk: risk_label(&result.risk).to_string(),
    }
}

fn plan_to_dto(
    plan: &OrganizationPlan,
    validation: &smart_file_organizer_core::PlanValidation,
    risk_by_file_id: &HashMap<Uuid, String>,
) -> OrganizationPlanDto {
    OrganizationPlanDto {
        plan_id: plan.plan_id.to_string(),
        task_id: plan.task_id.clone(),
        root_path: path_to_string(&plan.root_path),
        mode: mode_key(&plan.mode).to_string(),
        rows: plan
            .operations
            .iter()
            .map(|operation| operation_to_row_dto(operation, validation, risk_by_file_id))
            .collect(),
        summary: PlanSummaryDto {
            files_considered: plan.summary.files_considered,
            folders_to_create: plan.summary.folders_to_create,
            files_to_move: plan.summary.files_to_move,
            files_to_rename: plan.summary.files_to_rename,
        },
        created_at: plan.created_at.to_rfc3339(),
    }
}

fn operation_to_row_dto(
    operation: &FileOperationPlan,
    validation: &smart_file_organizer_core::PlanValidation,
    risk_by_file_id: &HashMap<Uuid, String>,
) -> OperationRowDto {
    let operation_issues = validation
        .issues
        .iter()
        .filter(|issue| issue.operation_id == Some(operation.operation_id))
        .map(validation_issue_to_dto)
        .collect::<Vec<_>>();
    let (operation_type, title, source, target) = match &operation.kind {
        FileOperationKind::CreateFolder { path } => (
            "create_folder",
            format!("创建文件夹 {}", display_name(path)),
            None,
            path_to_string(path),
        ),
        FileOperationKind::MoveFile {
            source,
            destination,
        } => (
            "move_file",
            format!("移动 {}", display_name(source)),
            Some(path_to_string(source)),
            path_to_string(destination),
        ),
        FileOperationKind::RenameFile {
            source,
            destination,
        } => (
            "rename_file",
            format!("重命名 {}", display_name(source)),
            Some(path_to_string(source)),
            path_to_string(destination),
        ),
    };
    let conflict_status = if operation_issues.is_empty() {
        "none"
    } else {
        "blocked"
    };

    OperationRowDto {
        operation_id: operation.operation_id.to_string(),
        operation_type: operation_type.to_string(),
        title,
        source,
        target: target.clone(),
        reason: operation.reason.clone(),
        risk: if operation_issues.is_empty() {
            operation
                .file_id
                .and_then(|file_id| risk_by_file_id.get(&file_id).cloned())
                .unwrap_or_else(|| "low".to_string())
        } else {
            "high".to_string()
        },
        selected: true,
        editable_target: target,
        validation_issues: operation_issues,
        conflict_status: conflict_status.to_string(),
        file_id: operation.file_id.map(|id| id.to_string()),
    }
}

fn validation_issue_to_dto(
    issue: &smart_file_organizer_core::ValidationIssue,
) -> ValidationIssueDto {
    ValidationIssueDto {
        operation_id: issue.operation_id.map(|id| id.to_string()),
        message: issue.message.clone(),
    }
}

fn plan_dto_to_core(plan: OrganizationPlanDto) -> Result<OrganizationPlan, String> {
    let plan_id = parse_uuid(&plan.plan_id, "plan_id")?;
    let root_path = PathBuf::from(&plan.root_path);
    let mode = parse_mode(&plan.mode)?;
    let operations = plan
        .rows
        .into_iter()
        .filter(|row| row.selected)
        .map(row_dto_to_operation)
        .collect::<Result<Vec<_>, _>>()?;
    let summary = summarize_operations(plan.summary.files_considered, &operations);
    let created_at = chrono::DateTime::parse_from_rfc3339(&plan.created_at)
        .map_err(|error| format!("invalid plan created_at: {error}"))?
        .with_timezone(&chrono::Utc);

    Ok(OrganizationPlan {
        plan_id,
        task_id: plan.task_id,
        root_path,
        mode,
        operations,
        summary,
        created_at,
    })
}

fn row_dto_to_operation(row: OperationRowDto) -> Result<FileOperationPlan, String> {
    let operation_id = parse_uuid(&row.operation_id, "operation_id")?;
    let file_id = row
        .file_id
        .as_deref()
        .map(|id| parse_uuid(id, "file_id"))
        .transpose()?;
    let target = PathBuf::from(if row.editable_target.is_empty() {
        row.target
    } else {
        row.editable_target
    });
    let kind = match row.operation_type.as_str() {
        "create_folder" => FileOperationKind::CreateFolder { path: target },
        "move_file" => FileOperationKind::MoveFile {
            source: row
                .source
                .map(PathBuf::from)
                .ok_or_else(|| "move_file row missing source".to_string())?,
            destination: target,
        },
        "rename_file" => FileOperationKind::RenameFile {
            source: row
                .source
                .map(PathBuf::from)
                .ok_or_else(|| "rename_file row missing source".to_string())?,
            destination: target,
        },
        other => return Err(format!("unsupported operation_type: {other}")),
    };

    Ok(FileOperationPlan {
        operation_id,
        kind,
        reason: row.reason,
        file_id,
    })
}

fn summarize_operations(files_considered: usize, operations: &[FileOperationPlan]) -> PlanSummary {
    PlanSummary {
        files_considered,
        folders_to_create: operations
            .iter()
            .filter(|operation| matches!(operation.kind, FileOperationKind::CreateFolder { .. }))
            .count(),
        files_to_move: operations
            .iter()
            .filter(|operation| matches!(operation.kind, FileOperationKind::MoveFile { .. }))
            .count(),
        files_to_rename: operations
            .iter()
            .filter(|operation| matches!(operation.kind, FileOperationKind::RenameFile { .. }))
            .count(),
    }
}

fn approval_dto_to_core(approval: UserApprovalDto) -> Result<UserApproval, String> {
    Ok(UserApproval {
        approved: approval.approved,
        approved_plan_id: parse_uuid(&approval.approved_plan_id, "approved_plan_id")?,
        approved_at: chrono::DateTime::parse_from_rfc3339(&approval.approved_at)
            .map_err(|error| format!("invalid approved_at: {error}"))?
            .with_timezone(&chrono::Utc),
        actor: approval.actor,
    })
}

fn execution_batch_to_dto(batch: &ExecutionBatch) -> ExecutionBatchDto {
    ExecutionBatchDto {
        batch_id: batch.batch_id.to_string(),
        plan_id: batch.plan_id.to_string(),
        status: execution_status_key(&batch.status).to_string(),
        executed_operations: batch
            .executed_operations
            .iter()
            .map(|operation| {
                let (operation_type, source, target) = operation_kind_parts(&operation.kind);
                ExecutedOperationDto {
                    operation_id: operation.operation_id.to_string(),
                    operation_type: operation_type.to_string(),
                    source,
                    target,
                    completed_at: operation.completed_at.to_rfc3339(),
                }
            })
            .collect(),
        rollback_entries: batch.rollback_entries.iter().map(rollback_to_dto).collect(),
        errors: batch
            .errors
            .iter()
            .map(|error| smart_file_organizer_core::ExecutionErrorDto {
                operation_id: error.operation_id.map(|id| id.to_string()),
                message: error.message.clone(),
            })
            .collect(),
        started_at: batch.started_at.to_rfc3339(),
        finished_at: batch.finished_at.to_rfc3339(),
    }
}

fn execution_batch_dto_to_core(batch: ExecutionBatchDto) -> Result<ExecutionBatch, String> {
    Ok(ExecutionBatch {
        batch_id: parse_uuid(&batch.batch_id, "batch_id")?,
        plan_id: parse_uuid(&batch.plan_id, "plan_id")?,
        status: parse_execution_status(&batch.status)?,
        executed_operations: batch
            .executed_operations
            .into_iter()
            .map(executed_operation_dto_to_core)
            .collect::<Result<Vec<_>, _>>()?,
        rollback_entries: batch
            .rollback_entries
            .into_iter()
            .map(rollback_entry_dto_to_core)
            .collect::<Result<Vec<_>, _>>()?,
        errors: batch
            .errors
            .into_iter()
            .map(|error| {
                Ok(smart_file_organizer_core::ExecutionError {
                    operation_id: error
                        .operation_id
                        .as_deref()
                        .map(|id| parse_uuid(id, "operation_id"))
                        .transpose()?,
                    message: error.message,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        started_at: parse_datetime(&batch.started_at, "started_at")?,
        finished_at: parse_datetime(&batch.finished_at, "finished_at")?,
    })
}

fn executed_operation_dto_to_core(
    operation: ExecutedOperationDto,
) -> Result<smart_file_organizer_core::ExecutedOperation, String> {
    let kind = operation_kind_from_parts(
        &operation.operation_type,
        operation.source.as_deref(),
        &operation.target,
    )?;
    Ok(smart_file_organizer_core::ExecutedOperation {
        operation_id: parse_uuid(&operation.operation_id, "operation_id")?,
        kind,
        completed_at: parse_datetime(&operation.completed_at, "completed_at")?,
    })
}

fn rollback_entry_dto_to_core(
    entry: RollbackEntryDto,
) -> Result<smart_file_organizer_core::RollbackEntry, String> {
    let action = match entry.action.as_str() {
        "remove_created_folder" => smart_file_organizer_core::RollbackAction::RemoveCreatedFolder {
            path: entry
                .from
                .map(PathBuf::from)
                .ok_or_else(|| "remove_created_folder rollback missing path".to_string())?,
        },
        "move_file_back" => smart_file_organizer_core::RollbackAction::MoveFileBack {
            from: entry
                .from
                .map(PathBuf::from)
                .ok_or_else(|| "move_file_back rollback missing from".to_string())?,
            to: entry
                .to
                .map(PathBuf::from)
                .ok_or_else(|| "move_file_back rollback missing to".to_string())?,
        },
        "rename_file_back" => smart_file_organizer_core::RollbackAction::RenameFileBack {
            from: entry
                .from
                .map(PathBuf::from)
                .ok_or_else(|| "rename_file_back rollback missing from".to_string())?,
            to: entry
                .to
                .map(PathBuf::from)
                .ok_or_else(|| "rename_file_back rollback missing to".to_string())?,
        },
        other => return Err(format!("unsupported rollback action: {other}")),
    };

    Ok(smart_file_organizer_core::RollbackEntry {
        batch_id: parse_uuid(&entry.batch_id, "batch_id")?,
        operation_id: parse_uuid(&entry.operation_id, "operation_id")?,
        action,
        created_at: parse_datetime(&entry.created_at, "rollback created_at")?,
    })
}

fn rollback_to_dto(entry: &smart_file_organizer_core::RollbackEntry) -> RollbackEntryDto {
    let (action, from, to) = match &entry.action {
        smart_file_organizer_core::RollbackAction::RemoveCreatedFolder { path } => {
            ("remove_created_folder", Some(path_to_string(path)), None)
        }
        smart_file_organizer_core::RollbackAction::MoveFileBack { from, to } => (
            "move_file_back",
            Some(path_to_string(from)),
            Some(path_to_string(to)),
        ),
        smart_file_organizer_core::RollbackAction::RenameFileBack { from, to } => (
            "rename_file_back",
            Some(path_to_string(from)),
            Some(path_to_string(to)),
        ),
    };
    RollbackEntryDto {
        batch_id: entry.batch_id.to_string(),
        operation_id: entry.operation_id.to_string(),
        action: action.to_string(),
        from,
        to,
        created_at: entry.created_at.to_rfc3339(),
    }
}

fn skill_to_dto(skill: &Skill) -> SkillDto {
    SkillDto {
        id: skill.id.to_string(),
        name: skill.name.clone(),
        enabled: skill.enabled,
        rule: skill.rule.clone(),
        created_at: skill.created_at.to_rfc3339(),
    }
}

fn skill_proposal_to_dto(proposal: SkillUpdateProposal) -> SkillUpdateProposalDto {
    SkillUpdateProposalDto {
        name: proposal.name,
        rule: proposal.rule,
        enabled: proposal.enabled,
        evidence: proposal.evidence,
        source_event_ids: proposal
            .source_event_ids
            .into_iter()
            .map(|event_id| event_id.to_string())
            .collect(),
    }
}

fn parse_uuid(value: &str, field: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|error| format!("invalid {field}: {error}"))
}

fn parse_datetime(value: &str, field: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|error| format!("invalid {field}: {error}"))
        .map(|value| value.with_timezone(&chrono::Utc))
}

fn operation_kind_parts(kind: &FileOperationKind) -> (&'static str, Option<String>, String) {
    match kind {
        FileOperationKind::CreateFolder { path } => ("create_folder", None, path_to_string(path)),
        FileOperationKind::MoveFile {
            source,
            destination,
        } => (
            "move_file",
            Some(path_to_string(source)),
            path_to_string(destination),
        ),
        FileOperationKind::RenameFile {
            source,
            destination,
        } => (
            "rename_file",
            Some(path_to_string(source)),
            path_to_string(destination),
        ),
    }
}

fn operation_kind_from_parts(
    operation_type: &str,
    source: Option<&str>,
    target: &str,
) -> Result<FileOperationKind, String> {
    match operation_type {
        "create_folder" => Ok(FileOperationKind::CreateFolder {
            path: PathBuf::from(target),
        }),
        "move_file" => Ok(FileOperationKind::MoveFile {
            source: source
                .map(PathBuf::from)
                .ok_or_else(|| "move_file operation missing source".to_string())?,
            destination: PathBuf::from(target),
        }),
        "rename_file" => Ok(FileOperationKind::RenameFile {
            source: source
                .map(PathBuf::from)
                .ok_or_else(|| "rename_file operation missing source".to_string())?,
            destination: PathBuf::from(target),
        }),
        other => Err(format!("unsupported operation_type: {other}")),
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

fn category_key(category: &FileCategory) -> &'static str {
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

fn parse_category(value: &str) -> Result<FileCategory, String> {
    match value {
        "documents" | "Documents" | "文档" => Ok(FileCategory::Documents),
        "images" | "Images" | "图片" => Ok(FileCategory::Images),
        "videos" | "Videos" | "视频" => Ok(FileCategory::Videos),
        "audio" | "Audio" | "音频" => Ok(FileCategory::Audio),
        "archives" | "Archives" | "压缩包" => Ok(FileCategory::Archives),
        "installers" | "Installers" | "安装包" => Ok(FileCategory::Installers),
        "code" | "Code" | "代码" => Ok(FileCategory::Code),
        "spreadsheets" | "Spreadsheets" | "表格" => Ok(FileCategory::Spreadsheets),
        "presentations" | "Presentations" | "演示文稿" => Ok(FileCategory::Presentations),
        "pdf" | "Pdf" | "PDF" => Ok(FileCategory::Pdf),
        "other" | "Other" | "其他" => Ok(FileCategory::Other),
        other => Err(format!("unsupported category: {other}")),
    }
}

fn risk_label(risk: &FileRiskLevel) -> &'static str {
    match risk {
        FileRiskLevel::Low => "low",
        FileRiskLevel::Medium => "medium",
        FileRiskLevel::High => "high",
    }
}

fn parse_risk(value: &str) -> Result<FileRiskLevel, String> {
    match value {
        "" | "low" | "Low" => Ok(FileRiskLevel::Low),
        "medium" | "Medium" => Ok(FileRiskLevel::Medium),
        "high" | "High" => Ok(FileRiskLevel::High),
        other => Err(format!("unsupported risk: {other}")),
    }
}

fn mode_key(mode: &OrganizationMode) -> &'static str {
    match mode {
        OrganizationMode::ByCategory => "by_category",
        OrganizationMode::ByExtension => "by_extension",
        OrganizationMode::Desktop => "desktop",
    }
}

fn parse_mode(value: &str) -> Result<OrganizationMode, String> {
    match value {
        "by_category" | "category" | "ByCategory" => Ok(OrganizationMode::ByCategory),
        "by_extension" | "extension" | "ByExtension" => Ok(OrganizationMode::ByExtension),
        "desktop" | "Desktop" => Ok(OrganizationMode::Desktop),
        other => Err(format!("unsupported organization mode: {other}")),
    }
}

fn execution_status_key(status: &smart_file_organizer_core::ExecutionStatus) -> &'static str {
    match status {
        smart_file_organizer_core::ExecutionStatus::Completed => "completed",
        smart_file_organizer_core::ExecutionStatus::PartiallyFailed => "partially_failed",
        smart_file_organizer_core::ExecutionStatus::Rejected => "rejected",
    }
}

fn parse_execution_status(
    status: &str,
) -> Result<smart_file_organizer_core::ExecutionStatus, String> {
    match status {
        "completed" | "Completed" => Ok(smart_file_organizer_core::ExecutionStatus::Completed),
        "partially_failed" | "PartiallyFailed" => {
            Ok(smart_file_organizer_core::ExecutionStatus::PartiallyFailed)
        }
        "rejected" | "Rejected" => Ok(smart_file_organizer_core::ExecutionStatus::Rejected),
        other => Err(format!("unsupported execution status: {other}")),
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

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path_to_string(path))
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
                rule: smart_file_organizer_core::SkillRule {
                    extension: Some("jpg".to_string()),
                    category: FileCategory::Documents,
                    ..smart_file_organizer_core::SkillRule::default()
                },
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

    #[tokio::test]
    async fn generate_plan_reclassifies_from_scan_storage_without_core_payload() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("a.pdf"), b"pdf").unwrap();
        let storage = smart_file_organizer_storage::Storage::in_memory().unwrap();
        let registry = ScanRegistry::default();

        scan_folder_with_storage(
            &storage,
            &registry,
            ScanFolderRequest {
                task_id: Some("task-plan".to_string()),
                root_path: path_to_string(temp.path()),
                recursive: true,
                max_depth: None,
                include_hidden: false,
                follow_symlinks: false,
            },
        )
        .unwrap();

        let prepared = prepare_generate_plan(
            &storage,
            GeneratePlanRequestDto {
                task_id: "task-plan".to_string(),
                root_path: path_to_string(temp.path()),
                mode: "by_category".to_string(),
                classifications: None,
            },
        )
        .unwrap();
        let plan = build_prepared_plan(prepared).await.unwrap();

        assert_eq!(plan.task_id, "task-plan");
        assert_eq!(plan.mode, "by_category");
        assert!(plan.rows.iter().any(|row| row.operation_type == "move_file"
            && row.title.contains("a.pdf")
            && row.selected
            && row.conflict_status == "none"));
        assert!(plan.rows.iter().all(|row| !row.editable_target.is_empty()));
    }

    #[test]
    fn plan_and_approval_dtos_convert_to_core_execution_contract() {
        let plan_id = Uuid::new_v4();
        let operation_id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("a.txt");
        let target = temp.path().join("Docs").join("a.txt");

        let plan = OrganizationPlanDto {
            plan_id: plan_id.to_string(),
            task_id: "task-dto".to_string(),
            root_path: path_to_string(temp.path()),
            mode: "by_category".to_string(),
            rows: vec![OperationRowDto {
                operation_id: operation_id.to_string(),
                operation_type: "move_file".to_string(),
                title: "移动 a.txt".to_string(),
                source: Some(path_to_string(&source)),
                target: path_to_string(&target),
                reason: "test".to_string(),
                risk: "low".to_string(),
                selected: true,
                editable_target: path_to_string(&target),
                validation_issues: Vec::new(),
                conflict_status: "none".to_string(),
                file_id: None,
            }],
            summary: PlanSummaryDto {
                files_considered: 1,
                folders_to_create: 0,
                files_to_move: 1,
                files_to_rename: 0,
            },
            created_at: now.clone(),
        };
        let approval = UserApprovalDto {
            approved: true,
            approved_plan_id: plan_id.to_string(),
            approved_at: now,
            actor: Some("tester".to_string()),
        };

        let core_plan = plan_dto_to_core(plan).unwrap();
        let core_approval = approval_dto_to_core(approval).unwrap();

        assert_eq!(core_plan.plan_id, plan_id);
        assert_eq!(core_approval.approved_plan_id, plan_id);
        assert!(matches!(
            &core_plan.operations[0].kind,
            FileOperationKind::MoveFile { source: actual_source, destination }
                if actual_source == &source && destination == &target
        ));
    }

    #[tokio::test]
    async fn plan_patch_rejects_rows_edits_targets_and_revalidates() {
        let plan_id = Uuid::new_v4();
        let first_operation_id = Uuid::new_v4();
        let second_operation_id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        let temp = tempfile::tempdir().unwrap();
        let first_source = temp.path().join("first.txt");
        let second_source = temp.path().join("second.txt");
        let existing_target = temp.path().join("Done").join("second.txt");
        fs::write(&first_source, b"first").unwrap();
        fs::write(&second_source, b"second").unwrap();
        fs::create_dir_all(existing_target.parent().unwrap()).unwrap();
        fs::write(&existing_target, b"collision").unwrap();

        let plan = OrganizationPlanDto {
            plan_id: plan_id.to_string(),
            task_id: "task-patch".to_string(),
            root_path: path_to_string(temp.path()),
            mode: "by_category".to_string(),
            rows: vec![
                OperationRowDto {
                    operation_id: first_operation_id.to_string(),
                    operation_type: "move_file".to_string(),
                    title: "移动 first.txt".to_string(),
                    source: Some(path_to_string(&first_source)),
                    target: path_to_string(&temp.path().join("Done").join("first.txt")),
                    reason: "test".to_string(),
                    risk: "low".to_string(),
                    selected: true,
                    editable_target: path_to_string(&temp.path().join("Done").join("first.txt")),
                    validation_issues: Vec::new(),
                    conflict_status: "none".to_string(),
                    file_id: None,
                },
                OperationRowDto {
                    operation_id: second_operation_id.to_string(),
                    operation_type: "move_file".to_string(),
                    title: "移动 second.txt".to_string(),
                    source: Some(path_to_string(&second_source)),
                    target: path_to_string(&temp.path().join("Done").join("new-second.txt")),
                    reason: "test".to_string(),
                    risk: "low".to_string(),
                    selected: true,
                    editable_target: path_to_string(
                        &temp.path().join("Done").join("new-second.txt"),
                    ),
                    validation_issues: Vec::new(),
                    conflict_status: "none".to_string(),
                    file_id: None,
                },
            ],
            summary: PlanSummaryDto {
                files_considered: 2,
                folders_to_create: 0,
                files_to_move: 2,
                files_to_rename: 0,
            },
            created_at: now,
        };

        let patched = apply_plan_patch_with_validation(
            plan,
            PlanPatchDto {
                operations: vec![
                    OperationPatchDto {
                        operation_id: first_operation_id.to_string(),
                        selected: Some(false),
                        editable_target: None,
                    },
                    OperationPatchDto {
                        operation_id: second_operation_id.to_string(),
                        selected: None,
                        editable_target: Some(path_to_string(&existing_target)),
                    },
                ],
            },
        )
        .await
        .unwrap();

        let first_row = patched
            .rows
            .iter()
            .find(|row| row.operation_id == first_operation_id.to_string())
            .unwrap();
        let second_row = patched
            .rows
            .iter()
            .find(|row| row.operation_id == second_operation_id.to_string())
            .unwrap();
        assert!(!first_row.selected);
        assert!(first_row.validation_issues.is_empty());
        assert_eq!(patched.summary.files_to_move, 1);
        assert_eq!(second_row.editable_target, path_to_string(&existing_target));
        assert_eq!(second_row.conflict_status, "blocked");
        assert!(second_row
            .validation_issues
            .iter()
            .any(|issue| issue.message.contains("destination already exists")));
    }

    #[tokio::test]
    async fn plan_patch_rejects_unsupported_delete_rows() {
        let temp = tempfile::tempdir().unwrap();
        let plan = OrganizationPlanDto {
            plan_id: Uuid::new_v4().to_string(),
            task_id: "task-delete".to_string(),
            root_path: path_to_string(temp.path()),
            mode: "by_category".to_string(),
            rows: vec![OperationRowDto {
                operation_id: Uuid::new_v4().to_string(),
                operation_type: "delete_file".to_string(),
                title: "删除 a.txt".to_string(),
                source: Some(path_to_string(&temp.path().join("a.txt"))),
                target: path_to_string(&temp.path().join("a.txt")),
                reason: "unsupported".to_string(),
                risk: "high".to_string(),
                selected: false,
                editable_target: path_to_string(&temp.path().join("a.txt")),
                validation_issues: Vec::new(),
                conflict_status: "none".to_string(),
                file_id: None,
            }],
            summary: PlanSummaryDto {
                files_considered: 1,
                folders_to_create: 0,
                files_to_move: 0,
                files_to_rename: 0,
            },
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let error = apply_plan_patch_with_validation(plan, PlanPatchDto { operations: vec![] })
            .await
            .unwrap_err();

        assert!(error.contains("unsupported operation_type: delete_file"));
    }

    #[test]
    fn execution_batch_dto_converts_to_core_for_history_rollback() {
        let batch_id = Uuid::new_v4();
        let plan_id = Uuid::new_v4();
        let operation_id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        let temp = tempfile::tempdir().unwrap();
        let moved = temp.path().join("Done").join("a.txt");
        let original = temp.path().join("a.txt");

        let batch = execution_batch_dto_to_core(ExecutionBatchDto {
            batch_id: batch_id.to_string(),
            plan_id: plan_id.to_string(),
            status: "completed".to_string(),
            executed_operations: vec![ExecutedOperationDto {
                operation_id: operation_id.to_string(),
                operation_type: "move_file".to_string(),
                source: Some(path_to_string(&original)),
                target: path_to_string(&moved),
                completed_at: now.clone(),
            }],
            rollback_entries: vec![RollbackEntryDto {
                batch_id: batch_id.to_string(),
                operation_id: operation_id.to_string(),
                action: "move_file_back".to_string(),
                from: Some(path_to_string(&moved)),
                to: Some(path_to_string(&original)),
                created_at: now.clone(),
            }],
            errors: Vec::new(),
            started_at: now.clone(),
            finished_at: now,
        })
        .unwrap();

        assert_eq!(batch.batch_id, batch_id);
        assert_eq!(batch.rollback_entries.len(), 1);
        assert!(matches!(
            &batch.rollback_entries[0].action,
            smart_file_organizer_core::RollbackAction::MoveFileBack { from, to }
                if from == &moved && to == &original
        ));
    }

    #[test]
    fn model_settings_commands_save_without_api_key() {
        let storage = smart_file_organizer_storage::Storage::in_memory().unwrap();
        let saved = save_model_settings_with_storage(
            &storage,
            ModelSettingsDto {
                provider: "openai-compatible".to_string(),
                base_url: Some("https://api.deepseek.example/v1".to_string()),
                model: Some("deepseek-chat".to_string()),
                cloud_enabled: true,
            },
        )
        .unwrap();
        let listed = list_model_settings_with_storage(&storage).unwrap();
        let serialized = serde_json::to_string(&listed).unwrap();

        assert_eq!(saved.provider, "openai-compatible");
        assert!(listed
            .providers
            .iter()
            .any(|provider| provider.provider == "ollama"));
        assert_eq!(listed.saved_settings.len(), 1);
        assert!(!serialized.contains("sk-runtime-only"));
        assert!(!serialized.to_ascii_lowercase().contains("api_key"));
    }

    #[tokio::test]
    async fn model_test_commands_validate_request_shape_and_json_output() {
        let request = TestModelRequest {
            settings: ModelSettingsDto {
                provider: "openai-compatible".to_string(),
                base_url: Some("https://api.kimi.example/v1".to_string()),
                model: Some("kimi-k2".to_string()),
                cloud_enabled: true,
            },
            api_key: Some("sk-runtime-only".to_string()),
        };

        let connection = test_model_connection(request.clone()).await.unwrap();
        let json_output = test_model_json_output(request).await.unwrap();

        assert!(connection.request_valid);
        assert!(connection.message.contains("/chat/completions"));
        assert!(json_output.valid);
        assert_eq!(json_output.categories_count, 1);
    }
}
