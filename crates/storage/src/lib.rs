use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use smart_file_organizer_core::{FileItem, HistorySummaryDto};
use uuid::Uuid;

const MIGRATION_0001_VERSION: &str = "0001_storage_skill_ai";
const MIGRATION_0001_SQL: &str = include_str!("../../../migrations/0001_storage_skill_ai.sql");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredSkill {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub rule_json: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiProviderSettings {
    pub provider: String,
    pub base_url: Option<String>,
    pub cloud_enabled: bool,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanTaskRecord {
    pub task_id: String,
    pub root_path: PathBuf,
    pub mode: String,
    pub os: String,
    pub status: String,
}

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).context("open sqlite database")?;
        let storage = Self { conn };
        storage.apply_migrations()?;
        Ok(storage)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().context("open in-memory sqlite database")?;
        let storage = Self { conn };
        storage.apply_migrations()?;
        Ok(storage)
    }

    pub fn apply_migrations(&self) -> Result<()> {
        self.conn
            .execute_batch(MIGRATION_0001_SQL)
            .context("apply base migration")?;
        self.ensure_ai_provider_settings_base_url()?;
        self.conn.execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)",
            params![MIGRATION_0001_VERSION],
        )?;
        Ok(())
    }

    pub fn create_scan_task(
        &self,
        task_id: &str,
        root_path: impl AsRef<Path>,
        mode: &str,
        os: &str,
        status: &str,
    ) -> Result<()> {
        let now = Utc::now().timestamp();
        let root_path = root_path.as_ref();
        let root_path_text = path_to_string(root_path);
        let root_path_hash = hash_path(root_path);
        self.conn.execute(
            "INSERT INTO organization_task(
               task_id, root_path, root_path_hash, mode, os, status, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(task_id) DO UPDATE SET
               root_path = excluded.root_path,
               root_path_hash = excluded.root_path_hash,
               mode = excluded.mode,
               os = excluded.os,
               status = excluded.status,
               updated_at = excluded.updated_at",
            params![
                task_id,
                root_path_text,
                root_path_hash,
                mode,
                os,
                status,
                now
            ],
        )?;
        Ok(())
    }

    pub fn update_scan_task_status(&self, task_id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE organization_task
             SET status = ?2, updated_at = ?3
             WHERE task_id = ?1",
            params![task_id, status, Utc::now().timestamp()],
        )?;
        Ok(())
    }

    pub fn upsert_file_items(&self, files: &[FileItem]) -> Result<()> {
        for file in files {
            self.conn.execute(
                "INSERT INTO file_item(
                   id, root_path, relative_path, path, path_hash, file_name, extension,
                   mime_type, size_bytes, created_at, modified_at, accessed_at, is_directory,
                   is_hidden, is_symlink, content_hash, indexed_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, ?13, ?14, NULL, ?15)
                 ON CONFLICT(path_hash) DO UPDATE SET
                   root_path = excluded.root_path,
                   relative_path = excluded.relative_path,
                   path = excluded.path,
                   file_name = excluded.file_name,
                   extension = excluded.extension,
                   mime_type = excluded.mime_type,
                   size_bytes = excluded.size_bytes,
                   created_at = excluded.created_at,
                   modified_at = excluded.modified_at,
                   accessed_at = excluded.accessed_at,
                   is_hidden = excluded.is_hidden,
                   is_symlink = excluded.is_symlink,
                   indexed_at = excluded.indexed_at",
                params![
                    file.id.to_string(),
                    path_to_string(&file.root),
                    path_to_string(&file.relative_path),
                    path_to_string(&file.path),
                    file.path_hash,
                    file.file_name,
                    file.extension,
                    file.mime_type,
                    size_to_i64(file.size_bytes),
                    timestamp(file.created_at),
                    timestamp(file.modified_at),
                    timestamp(file.accessed_at),
                    file.is_hidden as i64,
                    file.is_symlink as i64,
                    file.indexed_at.timestamp()
                ],
            )?;
        }
        Ok(())
    }

    pub fn list_files_for_root(&self, root_path: impl AsRef<Path>) -> Result<Vec<FileItem>> {
        let root_path = path_to_string(root_path.as_ref());
        let mut stmt = self.conn.prepare(
            "SELECT
               id, root_path, relative_path, path, file_name, extension, size_bytes,
               created_at, modified_at, accessed_at, is_hidden, is_symlink, mime_type,
               path_hash, indexed_at
             FROM file_item
             WHERE root_path = ?1
             ORDER BY path",
        )?;
        let files = stmt
            .query_map(params![root_path], row_to_file_item)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(files)
    }

    pub fn list_files_for_task(&self, task_id: &str) -> Result<Vec<FileItem>> {
        let root_path: Option<String> = self
            .conn
            .query_row(
                "SELECT root_path FROM organization_task WHERE task_id = ?1",
                params![task_id],
                |row| row.get(0),
            )
            .optional()?;

        match root_path {
            Some(root_path) => self.list_files_for_root(root_path),
            None => Ok(Vec::new()),
        }
    }

    pub fn count_file_items(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM file_item", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn save_plan(&self, id: &str, plan_json: &serde_json::Value) -> Result<()> {
        let plan_json = serde_json::to_string(plan_json)?;
        self.conn.execute(
            "INSERT INTO organization_plans(id, plan_json, status)
             VALUES (?1, ?2, 'draft')
             ON CONFLICT(id) DO UPDATE SET
               plan_json = excluded.plan_json,
               updated_at = CURRENT_TIMESTAMP",
            params![id, plan_json],
        )?;
        Ok(())
    }

    pub fn load_plan(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT plan_json FROM organization_plans WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;

        raw.map(|value| serde_json::from_str(&value).context("parse stored plan json"))
            .transpose()
    }

    pub fn record_execution_batch(
        &self,
        id: &str,
        plan_id: &str,
        status: &str,
        rollback_json: &serde_json::Value,
    ) -> Result<()> {
        let rollback_json = serde_json::to_string(rollback_json)?;
        self.conn.execute(
            "INSERT INTO execution_batches(id, plan_id, status, rollback_json)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
               plan_id = excluded.plan_id,
               status = excluded.status,
               rollback_json = excluded.rollback_json",
            params![id, plan_id, status, rollback_json],
        )?;
        Ok(())
    }

    pub fn list_execution_batches(&self) -> Result<Vec<HistorySummaryDto>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, plan_id, status, rollback_json, created_at
             FROM execution_batches
             ORDER BY created_at DESC, id DESC",
        )?;
        let batches = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let plan_id: String = row.get(1)?;
                let status: String = row.get(2)?;
                let rollback_json: String = row.get(3)?;
                let created_at: String = row.get(4)?;
                row_to_history_summary(id, plan_id, status, rollback_json, created_at)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(batches)
    }

    pub fn load_execution_batch(&self, id: &str) -> Result<Option<serde_json::Value>> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT rollback_json FROM execution_batches WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;

        raw.map(|value| serde_json::from_str(&value).context("parse stored execution batch json"))
            .transpose()
    }

    pub fn upsert_skill(&self, skill: &StoredSkill) -> Result<()> {
        let rule_json = serde_json::to_string(&skill.rule_json)?;
        self.conn.execute(
            "INSERT INTO skills(id, name, enabled, rule_json)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               enabled = excluded.enabled,
               rule_json = excluded.rule_json,
               updated_at = CURRENT_TIMESTAMP",
            params![skill.id, skill.name, skill.enabled as i64, rule_json],
        )?;
        Ok(())
    }

    pub fn list_enabled_skills(&self) -> Result<Vec<StoredSkill>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, enabled, rule_json FROM skills WHERE enabled = 1 ORDER BY name",
        )?;
        let skills = stmt
            .query_map([], |row| {
                let rule_json: String = row.get(3)?;
                let rule_json = serde_json::from_str(&rule_json).map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?;
                Ok(StoredSkill {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    enabled: row.get::<_, i64>(2)? != 0,
                    rule_json,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(skills)
    }

    pub fn save_ai_provider_settings(&self, settings: &AiProviderSettings) -> Result<()> {
        self.conn.execute(
            "INSERT INTO ai_provider_settings(provider, base_url, cloud_enabled, model)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(provider) DO UPDATE SET
               base_url = excluded.base_url,
               cloud_enabled = excluded.cloud_enabled,
               model = excluded.model,
               updated_at = CURRENT_TIMESTAMP",
            params![
                settings.provider,
                settings.base_url,
                settings.cloud_enabled as i64,
                settings.model
            ],
        )?;
        Ok(())
    }

    pub fn get_ai_provider_settings(&self, provider: &str) -> Result<Option<AiProviderSettings>> {
        self.conn
            .query_row(
                "SELECT provider, base_url, cloud_enabled, model
                 FROM ai_provider_settings
                 WHERE provider = ?1",
                params![provider],
                row_to_ai_provider_settings,
            )
            .optional()
            .context("load ai provider settings")
    }

    pub fn list_ai_provider_settings(&self) -> Result<Vec<AiProviderSettings>> {
        let mut stmt = self.conn.prepare(
            "SELECT provider, base_url, cloud_enabled, model
             FROM ai_provider_settings
             ORDER BY provider",
        )?;
        let settings = stmt
            .query_map([], row_to_ai_provider_settings)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(settings)
    }

    pub fn table_columns(&self, table: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(&format!("PRAGMA table_info({})", table.replace('"', "")))?;
        let columns = stmt
            .query_map([], |row| row.get(1))?
            .collect::<std::result::Result<Vec<String>, _>>()?;
        Ok(columns)
    }

    fn ensure_ai_provider_settings_base_url(&self) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare("PRAGMA table_info(ai_provider_settings)")?;
        let columns = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if !columns.iter().any(|column| column == "base_url") {
            self.conn.execute(
                "ALTER TABLE ai_provider_settings ADD COLUMN base_url TEXT",
                [],
            )?;
        }
        Ok(())
    }
}

fn row_to_ai_provider_settings(row: &rusqlite::Row<'_>) -> rusqlite::Result<AiProviderSettings> {
    Ok(AiProviderSettings {
        provider: row.get(0)?,
        base_url: row.get(1)?,
        cloud_enabled: row.get::<_, i64>(2)? != 0,
        model: row.get(3)?,
    })
}

fn row_to_file_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileItem> {
    let id: String = row.get(0)?;
    let root_path: String = row.get(1)?;
    let relative_path: String = row.get(2)?;
    let path: String = row.get(3)?;
    let size_bytes: i64 = row.get(6)?;
    let indexed_at: i64 = row.get(14)?;

    Ok(FileItem {
        id: Uuid::parse_str(&id).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
        })?,
        root: PathBuf::from(root_path),
        path: PathBuf::from(path),
        relative_path: PathBuf::from(relative_path),
        file_name: row.get(4)?,
        extension: row.get(5)?,
        size_bytes: size_bytes.max(0) as u64,
        created_at: datetime(row.get(7)?),
        modified_at: datetime(row.get(8)?),
        accessed_at: datetime(row.get(9)?),
        is_hidden: row.get::<_, i64>(10)? != 0,
        is_symlink: row.get::<_, i64>(11)? != 0,
        mime_type: row.get(12)?,
        path_hash: row.get(13)?,
        indexed_at: datetime(Some(indexed_at)).unwrap_or_else(Utc::now),
    })
}

fn row_to_history_summary(
    batch_id: String,
    plan_id: String,
    status: String,
    rollback_json: String,
    created_at: String,
) -> rusqlite::Result<HistorySummaryDto> {
    let value: serde_json::Value = serde_json::from_str(&rollback_json).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(err))
    })?;

    let operation_count = value
        .get("executedOperations")
        .and_then(|value| value.as_array())
        .map(Vec::len)
        .or_else(|| {
            value
                .get("executed_operations")
                .and_then(|value| value.as_array())
                .map(Vec::len)
        })
        .unwrap_or(0);
    let error_count = value
        .get("errors")
        .and_then(|value| value.as_array())
        .map(Vec::len)
        .unwrap_or(0);
    let started_at = value
        .get("startedAt")
        .or_else(|| value.get("started_at"))
        .and_then(|value| value.as_str())
        .unwrap_or(&created_at)
        .to_string();
    let finished_at = value
        .get("finishedAt")
        .or_else(|| value.get("finished_at"))
        .and_then(|value| value.as_str())
        .unwrap_or(&created_at)
        .to_string();

    Ok(HistorySummaryDto {
        batch_id,
        plan_id,
        status,
        operation_count,
        error_count,
        started_at,
        finished_at,
    })
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn hash_path(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path_to_string(path).as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn timestamp(value: Option<DateTime<Utc>>) -> Option<i64> {
    value.map(|value| value.timestamp())
}

fn datetime(value: Option<i64>) -> Option<DateTime<Utc>> {
    value.and_then(|value| DateTime::<Utc>::from_timestamp(value, 0))
}

fn size_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;
    use smart_file_organizer_core::FileItem;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn stores_plans_skills_and_rollback_records() {
        let storage = Storage::in_memory().unwrap();
        let plan = json!({
            "id": "plan-1",
            "operations": [
                {"kind": "MoveFile", "from": "/tmp/a.txt", "to": "/tmp/docs/a.txt"}
            ]
        });

        storage.save_plan("plan-1", &plan).unwrap();
        storage
            .record_execution_batch("batch-1", "plan-1", "done", &json!({"undo": []}))
            .unwrap();
        storage
            .upsert_skill(&StoredSkill {
                id: "skill-1".to_string(),
                name: "PDF docs".to_string(),
                enabled: true,
                rule_json: json!({"extension": "pdf", "category": "Documents"}),
            })
            .unwrap();

        assert_eq!(storage.load_plan("plan-1").unwrap(), Some(plan));
        assert_eq!(
            storage.load_execution_batch("batch-1").unwrap(),
            Some(json!({"undo": []}))
        );
        assert_eq!(storage.list_execution_batches().unwrap().len(), 1);
        assert_eq!(storage.list_enabled_skills().unwrap().len(), 1);
    }

    #[test]
    fn lists_execution_batches_from_stored_batch_json() {
        let storage = Storage::in_memory().unwrap();
        storage
            .save_plan("plan-1", &json!({"planId": "plan-1"}))
            .unwrap();
        storage
            .record_execution_batch(
                "batch-1",
                "plan-1",
                "partially_failed",
                &json!({
                    "batchId": "batch-1",
                    "planId": "plan-1",
                    "status": "partially_failed",
                    "executedOperations": [{"operationId": "op-1"}],
                    "rollbackEntries": [{"operationId": "op-1"}],
                    "errors": [{"message": "failed"}],
                    "startedAt": "2026-01-01T00:00:00Z",
                    "finishedAt": "2026-01-01T00:00:01Z"
                }),
            )
            .unwrap();

        let summaries = storage.list_execution_batches().unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].batch_id, "batch-1");
        assert_eq!(summaries[0].operation_count, 1);
        assert_eq!(summaries[0].error_count, 1);
        assert_eq!(summaries[0].started_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn ai_provider_settings_do_not_have_api_key_columns() {
        let storage = Storage::in_memory().unwrap();
        storage
            .save_ai_provider_settings(&AiProviderSettings {
                provider: "mock".to_string(),
                base_url: None,
                cloud_enabled: false,
                model: Some("local-test".to_string()),
            })
            .unwrap();

        let columns = storage.table_columns("ai_provider_settings").unwrap();
        assert!(columns.contains(&"provider".to_string()));
        assert!(columns.contains(&"base_url".to_string()));
        assert!(!columns.iter().any(|column| column.contains("key")));
        assert!(!columns.iter().any(|column| column.contains("secret")));
    }

    #[test]
    fn ai_provider_settings_roundtrip_without_credentials() {
        let storage = Storage::in_memory().unwrap();
        storage
            .save_ai_provider_settings(&AiProviderSettings {
                provider: "openai-compatible".to_string(),
                base_url: Some("https://api.deepseek.example/v1".to_string()),
                cloud_enabled: true,
                model: Some("deepseek-chat".to_string()),
            })
            .unwrap();

        let loaded = storage
            .get_ai_provider_settings("openai-compatible")
            .unwrap()
            .unwrap();
        let serialized = serde_json::to_string(&loaded).unwrap();

        assert_eq!(
            loaded.base_url.as_deref(),
            Some("https://api.deepseek.example/v1")
        );
        assert_eq!(storage.list_ai_provider_settings().unwrap().len(), 1);
        assert!(!serialized.to_ascii_lowercase().contains("api_key"));
        assert!(!serialized.to_ascii_lowercase().contains("authorization"));
    }

    #[test]
    fn migrations_create_scan_storage_tables() {
        let storage = Storage::in_memory().unwrap();

        let file_columns = storage.table_columns("file_item").unwrap();
        let task_columns = storage.table_columns("organization_task").unwrap();

        assert!(file_columns.contains(&"path_hash".to_string()));
        assert!(file_columns.contains(&"indexed_at".to_string()));
        assert!(task_columns.contains(&"root_path_hash".to_string()));
    }

    #[test]
    fn upserts_scanned_files_without_duplicates() {
        let storage = Storage::in_memory().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        let path = root.join("a.pdf");
        let now = Utc::now();
        let file = FileItem {
            id: Uuid::new_v4(),
            root: root.clone(),
            path: path.clone(),
            relative_path: PathBuf::from("a.pdf"),
            file_name: "a.pdf".to_string(),
            extension: Some("pdf".to_string()),
            size_bytes: 10,
            created_at: Some(now),
            modified_at: Some(now),
            accessed_at: Some(now),
            is_hidden: false,
            is_symlink: false,
            mime_type: Some("application/pdf".to_string()),
            path_hash: hash_path(&path),
            indexed_at: now,
        };

        storage
            .create_scan_task("task-1", &root, "files", "macos", "running")
            .unwrap();
        storage
            .upsert_file_items(std::slice::from_ref(&file))
            .unwrap();

        let mut updated = file;
        updated.id = Uuid::new_v4();
        updated.size_bytes = 20;
        storage.upsert_file_items(&[updated]).unwrap();
        storage
            .update_scan_task_status("task-1", "completed")
            .unwrap();

        let files = storage.list_files_for_task("task-1").unwrap();
        assert_eq!(storage.count_file_items().unwrap(), 1);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].size_bytes, 20);
    }
}
