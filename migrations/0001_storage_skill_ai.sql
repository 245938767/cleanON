CREATE TABLE IF NOT EXISTS schema_migrations (
  version TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS file_item (
  id TEXT PRIMARY KEY,
  root_path TEXT NOT NULL,
  relative_path TEXT NOT NULL,
  path TEXT NOT NULL,
  path_hash TEXT NOT NULL UNIQUE,
  file_name TEXT NOT NULL,
  extension TEXT,
  mime_type TEXT,
  size_bytes INTEGER NOT NULL,
  created_at INTEGER,
  modified_at INTEGER,
  accessed_at INTEGER,
  is_directory INTEGER NOT NULL DEFAULT 0,
  is_hidden INTEGER NOT NULL DEFAULT 0,
  is_symlink INTEGER NOT NULL DEFAULT 0,
  content_hash TEXT,
  indexed_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_file_item_path_hash ON file_item(path_hash);
CREATE INDEX IF NOT EXISTS idx_file_item_root_path ON file_item(root_path);
CREATE INDEX IF NOT EXISTS idx_file_item_extension ON file_item(extension);
CREATE INDEX IF NOT EXISTS idx_file_item_modified_at ON file_item(modified_at);

CREATE VIRTUAL TABLE IF NOT EXISTS file_fts USING fts5(
  file_id UNINDEXED,
  file_name,
  extracted_text,
  metadata_text
);

CREATE TABLE IF NOT EXISTS organization_task (
  task_id TEXT PRIMARY KEY,
  root_path TEXT NOT NULL,
  root_path_hash TEXT NOT NULL,
  mode TEXT NOT NULL,
  os TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS organization_plans (
  id TEXT PRIMARY KEY,
  plan_json TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'draft',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS execution_batches (
  id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL,
  status TEXT NOT NULL,
  rollback_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY(plan_id) REFERENCES organization_plans(id)
);

CREATE TABLE IF NOT EXISTS skills (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  rule_json TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS ai_provider_settings (
  provider TEXT PRIMARY KEY,
  cloud_enabled INTEGER NOT NULL DEFAULT 0,
  model TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
