use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use smart_file_organizer_core::{FileItem, ScanOptions};
use std::fs;
use std::path::Path;
use uuid::Uuid;
use walkdir::WalkDir;

#[async_trait]
pub trait FileScanner: Send + Sync {
    async fn scan(&self, options: ScanOptions) -> Result<Vec<FileItem>>;
}

#[derive(Debug, Default, Clone)]
pub struct DefaultFileScanner;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanStatus {
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    pub files: Vec<FileItem>,
    pub status: ScanStatus,
    pub skipped_count: usize,
    pub error_count: usize,
}

#[async_trait]
impl FileScanner for DefaultFileScanner {
    async fn scan(&self, options: ScanOptions) -> Result<Vec<FileItem>> {
        Ok(scan_sync(options)?.files)
    }
}

pub fn scan_sync(options: ScanOptions) -> Result<ScanReport> {
    scan_with_cancellation(options, || false)
}

pub fn scan_with_cancellation(
    options: ScanOptions,
    is_cancelled: impl Fn() -> bool,
) -> Result<ScanReport> {
    if smart_file_organizer_platform::is_sensitive_path(&options.root) {
        bail!(
            "refusing to scan sensitive directory: {}",
            options.root.display()
        );
    }
    if !options.root.is_dir() {
        bail!("scan root is not a directory: {}", options.root.display());
    }

    let walker_depth = if options.recursive {
        options
            .max_depth
            .map(|depth| depth.saturating_add(1))
            .unwrap_or(usize::MAX)
    } else {
        1
    };

    let mut files = Vec::new();
    let mut skipped_count = 0;
    let mut error_count = 0;
    for entry in WalkDir::new(&options.root)
        .follow_links(options.follow_symlinks)
        .max_depth(walker_depth)
        .into_iter()
        .filter_entry(|entry| should_descend(entry.path(), &options))
    {
        if is_cancelled() {
            return Ok(ScanReport {
                files,
                status: ScanStatus::Cancelled,
                skipped_count,
                error_count,
            });
        }

        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                error_count += 1;
                continue;
            }
        };
        if entry.depth() == 0 || !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path().to_path_buf();
        if !options.include_hidden && is_hidden(&path) {
            skipped_count += 1;
            continue;
        }
        if smart_file_organizer_platform::is_sensitive_path(&path) {
            skipped_count += 1;
            continue;
        }

        match build_file_item(&options.root, &path) {
            Ok(file) => files.push(file),
            Err(_) => error_count += 1,
        }
    }

    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(ScanReport {
        files,
        status: ScanStatus::Completed,
        skipped_count,
        error_count,
    })
}

fn build_file_item(root: &Path, path: &Path) -> Result<FileItem> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    let relative_path = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase());

    Ok(FileItem {
        id: Uuid::new_v4(),
        root: root.to_path_buf(),
        path: path.to_path_buf(),
        relative_path,
        file_name,
        extension,
        size_bytes: metadata.len(),
        created_at: metadata.created().ok().map(DateTime::<Utc>::from),
        modified_at: metadata.modified().ok().map(DateTime::<Utc>::from),
        accessed_at: metadata.accessed().ok().map(DateTime::<Utc>::from),
        is_hidden: is_hidden(path),
        is_symlink: metadata.file_type().is_symlink(),
        mime_type: mime_guess::from_path(path).first_raw().map(str::to_owned),
        path_hash: hash_path(path),
        indexed_at: Utc::now(),
    })
}

fn hash_path(path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn should_descend(path: &Path, options: &ScanOptions) -> bool {
    if path == options.root {
        return true;
    }
    if smart_file_organizer_platform::is_sensitive_path(path) {
        return false;
    }
    options.include_hidden || !is_hidden(path)
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .map(|name| name.to_string_lossy().starts_with('.'))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn scans_temp_directory_without_file_contents() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("a.pdf"), b"secret body").unwrap();
        fs::create_dir(temp.path().join("nested")).unwrap();
        fs::write(temp.path().join("nested").join("b.jpg"), b"image").unwrap();

        let files = DefaultFileScanner
            .scan(ScanOptions {
                root: temp.path().to_path_buf(),
                recursive: true,
                max_depth: None,
                include_hidden: false,
                follow_symlinks: false,
            })
            .await
            .unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|file| file.file_name == "a.pdf"));
        assert!(files.iter().all(|file| file.size_bytes > 0));
        assert!(files.iter().all(|file| file.path_hash.len() == 64));
    }

    #[tokio::test]
    async fn skips_hidden_files_by_default() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join(".hidden.txt"), b"hidden").unwrap();

        let files = DefaultFileScanner
            .scan(ScanOptions {
                root: temp.path().to_path_buf(),
                recursive: false,
                max_depth: None,
                include_hidden: false,
                follow_symlinks: false,
            })
            .await
            .unwrap();

        assert!(files.is_empty());
    }

    #[test]
    fn non_recursive_scan_ignores_nested_files() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("root.txt"), b"root").unwrap();
        fs::create_dir(temp.path().join("nested")).unwrap();
        fs::write(temp.path().join("nested").join("child.txt"), b"child").unwrap();

        let report = scan_sync(ScanOptions {
            root: temp.path().to_path_buf(),
            recursive: false,
            max_depth: None,
            include_hidden: false,
            follow_symlinks: false,
        })
        .unwrap();

        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].file_name, "root.txt");
    }

    #[test]
    fn max_depth_limits_recursive_scan() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("root.txt"), b"root").unwrap();
        fs::create_dir(temp.path().join("nested")).unwrap();
        fs::write(temp.path().join("nested").join("child.txt"), b"child").unwrap();

        let report = scan_sync(ScanOptions {
            root: temp.path().to_path_buf(),
            recursive: true,
            max_depth: Some(0),
            include_hidden: false,
            follow_symlinks: false,
        })
        .unwrap();

        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].file_name, "root.txt");
    }

    #[test]
    fn refuses_sensitive_scan_root() {
        let temp = tempfile::tempdir().unwrap();
        let sensitive = temp.path().join(".ssh");
        fs::create_dir(&sensitive).unwrap();

        let error = scan_sync(ScanOptions {
            root: sensitive,
            recursive: true,
            max_depth: None,
            include_hidden: false,
            follow_symlinks: false,
        })
        .unwrap_err();

        assert!(error.to_string().contains("sensitive directory"));
    }

    #[test]
    fn can_cancel_before_collecting_files() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("a.txt"), b"a").unwrap();

        let report = scan_with_cancellation(
            ScanOptions {
                root: temp.path().to_path_buf(),
                recursive: true,
                max_depth: None,
                include_hidden: false,
                follow_symlinks: false,
            },
            || true,
        )
        .unwrap();

        assert_eq!(report.status, ScanStatus::Cancelled);
        assert!(report.files.is_empty());
    }
}
