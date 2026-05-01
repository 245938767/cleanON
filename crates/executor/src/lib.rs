use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use smart_file_organizer_core::{
    ExecutedOperation, ExecutionBatch, ExecutionError, ExecutionStatus, FileOperationKind,
    OrganizationPlan, OrganizerError, PlanValidation, RollbackAction, RollbackResult, UserApproval,
    ValidationIssue,
};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[async_trait]
pub trait PlanExecutor: Send + Sync {
    async fn validate_plan(&self, plan: &OrganizationPlan) -> Result<PlanValidation>;
    async fn execute_confirmed(
        &self,
        plan: &OrganizationPlan,
        approval: &UserApproval,
    ) -> Result<ExecutionBatch>;
    async fn rollback_batch(&self, batch: &ExecutionBatch) -> Result<RollbackResult>;
}

#[derive(Debug, Default, Clone)]
pub struct DefaultPlanExecutor;

#[async_trait]
impl PlanExecutor for DefaultPlanExecutor {
    async fn validate_plan(&self, plan: &OrganizationPlan) -> Result<PlanValidation> {
        Ok(validate_plan_sync(plan))
    }

    async fn execute_confirmed(
        &self,
        plan: &OrganizationPlan,
        approval: &UserApproval,
    ) -> Result<ExecutionBatch> {
        ensure_approval(plan, approval)?;
        let validation = validate_plan_sync(plan);
        if !validation.valid {
            let message = validation
                .issues
                .iter()
                .map(|issue| issue.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(OrganizerError::ValidationFailed(message).into());
        }

        execute_plan_sync(plan)
    }

    async fn rollback_batch(&self, batch: &ExecutionBatch) -> Result<RollbackResult> {
        Ok(smart_file_organizer_rollback::rollback_batch(batch))
    }
}

fn ensure_approval(plan: &OrganizationPlan, approval: &UserApproval) -> Result<()> {
    if !approval.approved {
        return Err(OrganizerError::ApprovalRequired.into());
    }
    if approval.approved_plan_id != plan.plan_id {
        return Err(OrganizerError::ApprovalPlanMismatch.into());
    }
    Ok(())
}

fn validate_plan_sync(plan: &OrganizationPlan) -> PlanValidation {
    let mut issues = Vec::new();
    if !plan.root_path.is_dir() {
        issues.push(ValidationIssue {
            operation_id: None,
            message: format!("plan root is not a directory: {}", plan.root_path.display()),
        });
    }

    for operation in &plan.operations {
        match &operation.kind {
            FileOperationKind::CreateFolder { path } => {
                validate_inside_root(path, plan, operation.operation_id, &mut issues);
                if path.exists() && !path.is_dir() {
                    issues.push(ValidationIssue {
                        operation_id: Some(operation.operation_id),
                        message: format!(
                            "create-folder target is not a directory: {}",
                            path.display()
                        ),
                    });
                }
            }
            FileOperationKind::MoveFile {
                source,
                destination,
            }
            | FileOperationKind::RenameFile {
                source,
                destination,
            } => {
                validate_inside_root(source, plan, operation.operation_id, &mut issues);
                validate_inside_root(destination, plan, operation.operation_id, &mut issues);
                if !source.is_file() {
                    issues.push(ValidationIssue {
                        operation_id: Some(operation.operation_id),
                        message: format!("source file does not exist: {}", source.display()),
                    });
                }
                if destination.exists() {
                    issues.push(ValidationIssue {
                        operation_id: Some(operation.operation_id),
                        message: format!("destination already exists: {}", destination.display()),
                    });
                }
                if source == destination {
                    issues.push(ValidationIssue {
                        operation_id: Some(operation.operation_id),
                        message: "source and destination are identical".to_string(),
                    });
                }
            }
        }
    }

    PlanValidation::from_issues(issues)
}

fn validate_inside_root(
    path: &Path,
    plan: &OrganizationPlan,
    operation_id: Uuid,
    issues: &mut Vec<ValidationIssue>,
) {
    if !path.starts_with(&plan.root_path) {
        issues.push(ValidationIssue {
            operation_id: Some(operation_id),
            message: format!("operation path escapes plan root: {}", path.display()),
        });
    }
}

fn execute_plan_sync(plan: &OrganizationPlan) -> Result<ExecutionBatch> {
    let batch_id = Uuid::new_v4();
    let started_at = Utc::now();
    let mut executed_operations = Vec::new();
    let mut rollback_entries = Vec::new();
    let mut errors = Vec::new();

    for operation in &plan.operations {
        let result = match &operation.kind {
            FileOperationKind::CreateFolder { path } => {
                execute_create_folder(batch_id, operation.operation_id, path)
            }
            FileOperationKind::MoveFile {
                source,
                destination,
            } => execute_move_file(batch_id, operation.operation_id, source, destination, false),
            FileOperationKind::RenameFile {
                source,
                destination,
            } => execute_move_file(batch_id, operation.operation_id, source, destination, true),
        };

        match result {
            Ok(Some(rollback_entry)) => {
                rollback_entries.push(rollback_entry);
                executed_operations.push(ExecutedOperation {
                    operation_id: operation.operation_id,
                    kind: operation.kind.clone(),
                    completed_at: Utc::now(),
                });
            }
            Ok(None) => {
                executed_operations.push(ExecutedOperation {
                    operation_id: operation.operation_id,
                    kind: operation.kind.clone(),
                    completed_at: Utc::now(),
                });
            }
            Err(error) => {
                errors.push(ExecutionError {
                    operation_id: Some(operation.operation_id),
                    message: error.to_string(),
                });
                break;
            }
        }
    }

    let status = if errors.is_empty() {
        ExecutionStatus::Completed
    } else {
        ExecutionStatus::PartiallyFailed
    };

    Ok(ExecutionBatch {
        batch_id,
        plan_id: plan.plan_id,
        status,
        executed_operations,
        rollback_entries,
        errors,
        started_at,
        finished_at: Utc::now(),
    })
}

fn execute_create_folder(
    batch_id: Uuid,
    operation_id: Uuid,
    path: &Path,
) -> Result<Option<smart_file_organizer_core::RollbackEntry>> {
    if path.exists() {
        return Ok(None);
    }
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;
    Ok(Some(smart_file_organizer_rollback::rollback_entry(
        batch_id,
        operation_id,
        RollbackAction::RemoveCreatedFolder {
            path: path.to_path_buf(),
        },
    )))
}

fn execute_move_file(
    batch_id: Uuid,
    operation_id: Uuid,
    source: &Path,
    destination: &Path,
    is_rename: bool,
) -> Result<Option<smart_file_organizer_core::RollbackEntry>> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    } else {
        bail!("destination has no parent: {}", destination.display());
    }

    fs::rename(source, destination).with_context(|| {
        format!(
            "failed to move {} to {}",
            source.display(),
            destination.display()
        )
    })?;

    let action = if is_rename {
        RollbackAction::RenameFileBack {
            from: destination.to_path_buf(),
            to: source.to_path_buf(),
        }
    } else {
        RollbackAction::MoveFileBack {
            from: destination.to_path_buf(),
            to: source.to_path_buf(),
        }
    };

    Ok(Some(smart_file_organizer_rollback::rollback_entry(
        batch_id,
        operation_id,
        action,
    )))
}

#[allow(dead_code)]
fn lexical_clean(path: &Path) -> PathBuf {
    path.components().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use smart_file_organizer_classifier::{BasicClassifier, Classifier};
    use smart_file_organizer_core::{BuildPlanInput, OrganizationMode, ScanOptions};
    use smart_file_organizer_planner::{DefaultPlanBuilder, PlanBuilder};
    use smart_file_organizer_scanner::{DefaultFileScanner, FileScanner};
    use std::fs;

    #[tokio::test]
    async fn refuses_execution_without_matching_approval() {
        let temp = tempfile::tempdir().unwrap();
        let plan = OrganizationPlan {
            plan_id: Uuid::new_v4(),
            task_id: "task".to_string(),
            root_path: temp.path().to_path_buf(),
            mode: OrganizationMode::ByCategory,
            operations: Vec::new(),
            summary: smart_file_organizer_core::PlanSummary {
                files_considered: 0,
                folders_to_create: 0,
                files_to_move: 0,
                files_to_rename: 0,
            },
            created_at: Utc::now(),
        };
        let approval = UserApproval {
            approved: true,
            approved_plan_id: Uuid::new_v4(),
            approved_at: Utc::now(),
            actor: None,
        };

        let error = DefaultPlanExecutor
            .execute_confirmed(&plan, &approval)
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("approval does not match plan id"));
    }

    #[tokio::test]
    async fn executes_and_rolls_back_temp_directory_flow() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("report.pdf"), b"pdf").unwrap();
        fs::write(temp.path().join("photo.jpg"), b"jpg").unwrap();

        let scanner = DefaultFileScanner;
        let classifier = BasicClassifier;
        let planner = DefaultPlanBuilder;
        let executor = DefaultPlanExecutor;

        let files = scanner
            .scan(ScanOptions {
                root: temp.path().to_path_buf(),
                recursive: false,
                max_depth: None,
                include_hidden: false,
                follow_symlinks: false,
            })
            .await
            .unwrap();
        let mut classifications = Vec::new();
        for file in files {
            classifications.push(
                classifier
                    .classify(
                        &file,
                        &smart_file_organizer_core::ClassificationContext {
                            root_path: temp.path().to_path_buf(),
                            ..Default::default()
                        },
                    )
                    .await
                    .unwrap(),
            );
        }

        let plan = planner
            .build_plan(BuildPlanInput {
                task_id: "task".to_string(),
                root_path: temp.path().to_path_buf(),
                mode: OrganizationMode::ByCategory,
                classifications,
            })
            .await
            .unwrap();

        let validation = executor.validate_plan(&plan).await.unwrap();
        assert!(validation.valid, "{:?}", validation.issues);

        let approval = UserApproval {
            approved: true,
            approved_plan_id: plan.plan_id,
            approved_at: Utc::now(),
            actor: Some("test".to_string()),
        };
        let batch = executor.execute_confirmed(&plan, &approval).await.unwrap();

        assert_eq!(batch.status, ExecutionStatus::Completed);
        assert!(temp.path().join("PDF").join("report.pdf").exists());
        assert!(temp.path().join("Images").join("photo.jpg").exists());

        let rollback = executor.rollback_batch(&batch).await.unwrap();
        assert!(rollback.errors.is_empty(), "{:?}", rollback.errors);
        assert!(temp.path().join("report.pdf").exists());
        assert!(temp.path().join("photo.jpg").exists());
    }

    #[tokio::test]
    async fn validation_rejects_destination_collision() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("a.txt");
        let destination = temp.path().join("b.txt");
        fs::write(&source, b"a").unwrap();
        fs::write(&destination, b"b").unwrap();
        let plan = OrganizationPlan {
            plan_id: Uuid::new_v4(),
            task_id: "task".to_string(),
            root_path: temp.path().to_path_buf(),
            mode: OrganizationMode::ByCategory,
            operations: vec![smart_file_organizer_core::FileOperationPlan {
                operation_id: Uuid::new_v4(),
                kind: FileOperationKind::MoveFile {
                    source,
                    destination,
                },
                reason: "test".to_string(),
                file_id: None,
            }],
            summary: smart_file_organizer_core::PlanSummary {
                files_considered: 1,
                folders_to_create: 0,
                files_to_move: 1,
                files_to_rename: 0,
            },
            created_at: Utc::now(),
        };

        let validation = DefaultPlanExecutor.validate_plan(&plan).await.unwrap();

        assert!(!validation.valid);
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.message.contains("destination already exists")));
    }
}
