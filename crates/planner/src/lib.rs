use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use smart_file_organizer_core::{
    BuildPlanInput, FileOperationKind, FileOperationPlan, OrganizationMode, OrganizationPlan,
    PlanSummary,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use uuid::Uuid;

#[async_trait]
pub trait PlanBuilder: Send + Sync {
    async fn build_plan(&self, input: BuildPlanInput) -> Result<OrganizationPlan>;
}

#[derive(Debug, Default, Clone)]
pub struct DefaultPlanBuilder;

#[async_trait]
impl PlanBuilder for DefaultPlanBuilder {
    async fn build_plan(&self, input: BuildPlanInput) -> Result<OrganizationPlan> {
        let mut operations = Vec::new();
        let mut folders = BTreeSet::new();
        let mut planned_moves = Vec::new();

        for classification in &input.classifications {
            let folder = target_folder(&input, classification);
            folders.insert(folder.clone());

            let source = classification.file.path.clone();
            let destination =
                unique_destination(&folder.join(&classification.file.file_name), &source);
            if source != destination {
                planned_moves.push(FileOperationPlan {
                    operation_id: Uuid::new_v4(),
                    kind: FileOperationKind::MoveFile {
                        source,
                        destination,
                    },
                    reason: format!(
                        "classified as {:?} with confidence {:.2}",
                        classification.category, classification.confidence
                    ),
                    file_id: Some(classification.file.id),
                });
            }
        }

        for folder in folders {
            operations.push(FileOperationPlan {
                operation_id: Uuid::new_v4(),
                kind: FileOperationKind::CreateFolder { path: folder },
                reason: "required by organization plan".to_string(),
                file_id: None,
            });
        }
        operations.extend(planned_moves);

        let summary = PlanSummary {
            files_considered: input.classifications.len(),
            folders_to_create: operations
                .iter()
                .filter(|operation| {
                    matches!(operation.kind, FileOperationKind::CreateFolder { .. })
                })
                .count(),
            files_to_move: operations
                .iter()
                .filter(|operation| matches!(operation.kind, FileOperationKind::MoveFile { .. }))
                .count(),
            files_to_rename: operations
                .iter()
                .filter(|operation| matches!(operation.kind, FileOperationKind::RenameFile { .. }))
                .count(),
        };

        Ok(OrganizationPlan {
            plan_id: Uuid::new_v4(),
            task_id: input.task_id,
            root_path: input.root_path,
            mode: input.mode,
            operations,
            summary,
            created_at: Utc::now(),
        })
    }
}

fn target_folder(
    input: &BuildPlanInput,
    classification: &smart_file_organizer_core::ClassificationResult,
) -> PathBuf {
    match input.mode {
        OrganizationMode::ByCategory => input.root_path.join(classification.category.folder_name()),
        OrganizationMode::Desktop => smart_file_organizer_platform::desktop_archive_folder(
            &input.root_path,
            &classification.category,
        ),
        OrganizationMode::ByExtension => input.root_path.join(
            classification
                .file
                .extension
                .as_deref()
                .filter(|extension| !extension.is_empty())
                .unwrap_or("no-extension"),
        ),
    }
}

fn unique_destination(destination: &std::path::Path, source: &std::path::Path) -> PathBuf {
    if destination == source || !destination.exists() {
        return destination.to_path_buf();
    }

    let stem = destination
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let extension = destination
        .extension()
        .map(|extension| extension.to_string_lossy().into_owned());

    for index in 1.. {
        let candidate_name = match &extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = destination.with_file_name(candidate_name);
        if candidate != source && !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded suffix loop should always return a candidate")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use smart_file_organizer_core::{ClassificationResult, FileCategory, FileItem, FileRiskLevel};
    use std::path::PathBuf;
    use uuid::Uuid;

    fn classified_pdf(root: PathBuf) -> ClassificationResult {
        let now = Utc::now();
        ClassificationResult {
            file: FileItem {
                id: Uuid::new_v4(),
                root: root.clone(),
                path: root.join("a.pdf"),
                relative_path: PathBuf::from("a.pdf"),
                file_name: "a.pdf".to_string(),
                extension: Some("pdf".to_string()),
                size_bytes: 1,
                created_at: None,
                modified_at: None,
                accessed_at: None,
                is_hidden: false,
                is_symlink: false,
                mime_type: None,
                path_hash: "hash-a-pdf".to_string(),
                indexed_at: now,
            },
            category: FileCategory::Pdf,
            confidence: 0.9,
            evidence: vec!["extension .pdf matched local rule".to_string()],
            risk: FileRiskLevel::Low,
        }
    }

    #[tokio::test]
    async fn builds_preview_only_plan() {
        let root = PathBuf::from("/tmp/example");
        let plan = DefaultPlanBuilder
            .build_plan(BuildPlanInput {
                task_id: "task-1".to_string(),
                root_path: root.clone(),
                mode: OrganizationMode::ByCategory,
                classifications: vec![classified_pdf(root.clone())],
            })
            .await
            .unwrap();

        assert_eq!(plan.summary.files_considered, 1);
        assert_eq!(plan.summary.folders_to_create, 1);
        assert_eq!(plan.summary.files_to_move, 1);
        assert!(plan.operations.iter().any(|operation| {
            matches!(&operation.kind, FileOperationKind::CreateFolder { path } if path == &root.join("PDF"))
        }));
    }

    #[tokio::test]
    async fn desktop_mode_builds_archive_plan_without_coordinate_operations() {
        let root = PathBuf::from("/tmp/Desktop");
        let plan = DefaultPlanBuilder
            .build_plan(BuildPlanInput {
                task_id: "desktop-task".to_string(),
                root_path: root.clone(),
                mode: OrganizationMode::Desktop,
                classifications: vec![classified_pdf(root.clone())],
            })
            .await
            .unwrap();

        assert_eq!(plan.mode, OrganizationMode::Desktop);
        assert!(plan.operations.iter().all(|operation| matches!(
            operation.kind,
            FileOperationKind::CreateFolder { .. } | FileOperationKind::MoveFile { .. }
        )));
        assert!(plan.operations.iter().any(|operation| {
            matches!(&operation.kind, FileOperationKind::CreateFolder { path } if path == &root.join("Desktop Archive").join("PDF"))
        }));
        assert!(plan.operations.iter().any(|operation| {
            matches!(&operation.kind, FileOperationKind::MoveFile { destination, .. } if destination == &root.join("Desktop Archive").join("PDF").join("a.pdf"))
        }));
    }
}
