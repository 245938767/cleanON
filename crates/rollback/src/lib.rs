use anyhow::{Context, Result};
use chrono::Utc;
use smart_file_organizer_core::{
    ExecutionBatch, ExecutionError, RollbackAction, RollbackEntry, RollbackResult,
};
use std::fs;

pub fn rollback_batch(batch: &ExecutionBatch) -> RollbackResult {
    let mut rolled_back = Vec::new();
    let mut errors = Vec::new();

    for entry in batch.rollback_entries.iter().rev() {
        match apply_entry(entry) {
            Ok(()) => rolled_back.push(entry.operation_id),
            Err(error) => errors.push(ExecutionError {
                operation_id: Some(entry.operation_id),
                message: error.to_string(),
            }),
        }
    }

    RollbackResult {
        batch_id: batch.batch_id,
        rolled_back,
        errors,
    }
}

fn apply_entry(entry: &RollbackEntry) -> Result<()> {
    match &entry.action {
        RollbackAction::RemoveCreatedFolder { path } => {
            if path.exists() {
                fs::remove_dir(path).with_context(|| {
                    format!("failed to remove created folder {}", path.display())
                })?;
            }
        }
        RollbackAction::MoveFileBack { from, to } | RollbackAction::RenameFileBack { from, to } => {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to recreate {}", parent.display()))?;
            }
            fs::rename(from, to).with_context(|| {
                format!("failed to move {} back to {}", from.display(), to.display())
            })?;
        }
    }
    Ok(())
}

pub fn rollback_entry(
    batch_id: uuid::Uuid,
    operation_id: uuid::Uuid,
    action: RollbackAction,
) -> RollbackEntry {
    RollbackEntry {
        batch_id,
        operation_id,
        action,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_file_organizer_core::{ExecutionBatch, ExecutionStatus};
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn rolls_file_move_back_in_reverse_order() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("a.txt");
        let destination = temp.path().join("Documents").join("a.txt");
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&destination, b"body").unwrap();

        let batch_id = Uuid::new_v4();
        let operation_id = Uuid::new_v4();
        let result = rollback_batch(&ExecutionBatch {
            batch_id,
            plan_id: Uuid::new_v4(),
            status: ExecutionStatus::Completed,
            executed_operations: Vec::new(),
            rollback_entries: vec![rollback_entry(
                batch_id,
                operation_id,
                RollbackAction::MoveFileBack {
                    from: destination.clone(),
                    to: source.clone(),
                },
            )],
            errors: Vec::new(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
        });

        assert!(result.errors.is_empty());
        assert!(source.exists());
        assert!(!destination.exists());
    }
}
