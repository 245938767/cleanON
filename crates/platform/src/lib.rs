use std::path::{Path, PathBuf};

use smart_file_organizer_core::{
    ClassificationResult, DesktopCapabilityFlagsDto, DesktopPreviewCanvasDto, DesktopPreviewDto,
    DesktopPreviewFileDto, DesktopPreviewGroupDto, DesktopPreviewPlatform, DesktopPreviewRectDto,
    DesktopPreviewZoneDto, FileCategory,
};

const SENSITIVE_SUFFIXES: &[&str] = &[
    ".ssh",
    ".gnupg",
    "Library/Keychains",
    "Library/Application Support/1Password",
    "Library/Application Support/Google/Chrome",
    "Library/Application Support/Firefox",
    "AppData/Roaming/Microsoft/Credentials",
    "AppData/Roaming/1Password",
    "AppData/Local/Google/Chrome/User Data",
];

pub fn is_sensitive_path(path: impl AsRef<Path>) -> bool {
    let normalized = normalize(path.as_ref());
    SENSITIVE_SUFFIXES
        .iter()
        .any(|suffix| normalized.ends_with(&normalize(Path::new(suffix))))
}

pub fn default_sensitive_paths(home: impl AsRef<Path>) -> Vec<PathBuf> {
    let home = home.as_ref();
    SENSITIVE_SUFFIXES
        .iter()
        .map(|suffix| home.join(suffix))
        .collect()
}

pub fn current_desktop_platform() -> DesktopPreviewPlatform {
    match std::env::consts::OS {
        "macos" => DesktopPreviewPlatform::Macos,
        "windows" => DesktopPreviewPlatform::Windows,
        _ => DesktopPreviewPlatform::Other,
    }
}

pub fn desktop_capabilities(platform: DesktopPreviewPlatform) -> DesktopCapabilityFlagsDto {
    DesktopCapabilityFlagsDto {
        preview_only: true,
        supports_file_archive_plan: true,
        supports_desktop_canvas_preview: matches!(platform, DesktopPreviewPlatform::Windows),
        supports_icon_coordinate_writeback: false,
        supports_pixel_perfect_layout: false,
    }
}

pub fn build_desktop_preview(
    platform: DesktopPreviewPlatform,
    desktop_root: impl AsRef<Path>,
    classifications: &[ClassificationResult],
) -> DesktopPreviewDto {
    let desktop_root = desktop_root.as_ref();
    let groups = grouped_classifications(classifications);
    let canvas = desktop_canvas(platform);
    let after_zones = groups
        .iter()
        .enumerate()
        .map(|(index, group)| zone_for_group(index, group, &canvas, desktop_root))
        .collect();

    DesktopPreviewDto {
        platform,
        capabilities: desktop_capabilities(platform),
        canvas,
        before_groups: groups,
        after_zones,
    }
}

pub fn desktop_archive_folder(root: impl AsRef<Path>, category: &FileCategory) -> PathBuf {
    root.as_ref()
        .join("Desktop Archive")
        .join(category.folder_name())
}

fn normalize(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn desktop_canvas(platform: DesktopPreviewPlatform) -> DesktopPreviewCanvasDto {
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

fn grouped_classifications(
    classifications: &[ClassificationResult],
) -> Vec<DesktopPreviewGroupDto> {
    let mut groups = Vec::<DesktopPreviewGroupDto>::new();

    for classification in classifications {
        let category_key = category_key(&classification.category);
        let group_index = groups
            .iter()
            .position(|group| group.category_key == category_key);
        let file = DesktopPreviewFileDto {
            file_id: classification.file.id.to_string(),
            name: classification.file.file_name.clone(),
            path: classification.file.path.to_string_lossy().into_owned(),
            size_bytes: classification.file.size_bytes,
        };

        match group_index {
            Some(index) => {
                let group = &mut groups[index];
                group.file_count += 1;
                group.total_size_bytes += file.size_bytes;
                group.files.push(file);
            }
            None => groups.push(DesktopPreviewGroupDto {
                group_id: format!("before-{category_key}"),
                title: classification.category.folder_name().to_string(),
                category_key: category_key.to_string(),
                file_count: 1,
                total_size_bytes: file.size_bytes,
                files: vec![file],
            }),
        }
    }

    groups
}

fn zone_for_group(
    index: usize,
    group: &DesktopPreviewGroupDto,
    canvas: &DesktopPreviewCanvasDto,
    desktop_root: &Path,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sensitive_suffixes() {
        assert!(is_sensitive_path("/Users/me/.ssh"));
        assert!(is_sensitive_path("/Users/me/Library/Keychains"));
        assert!(!is_sensitive_path("/Users/me/Desktop"));
    }

    #[test]
    fn desktop_capabilities_do_not_allow_coordinate_writeback() {
        let macos = desktop_capabilities(DesktopPreviewPlatform::Macos);
        assert!(macos.preview_only);
        assert!(macos.supports_file_archive_plan);
        assert!(!macos.supports_desktop_canvas_preview);
        assert!(!macos.supports_icon_coordinate_writeback);
        assert!(!macos.supports_pixel_perfect_layout);

        let windows = desktop_capabilities(DesktopPreviewPlatform::Windows);
        assert!(windows.preview_only);
        assert!(windows.supports_desktop_canvas_preview);
        assert!(!windows.supports_icon_coordinate_writeback);
        assert!(!windows.supports_pixel_perfect_layout);
    }

    #[test]
    fn desktop_preview_groups_files_into_before_groups_and_after_zones() {
        let root = PathBuf::from("/tmp/Desktop");
        let preview = build_desktop_preview(
            DesktopPreviewPlatform::Windows,
            &root,
            &[
                classified_file(&root, "a.pdf", FileCategory::Pdf, 10),
                classified_file(&root, "b.pdf", FileCategory::Pdf, 20),
                classified_file(&root, "photo.jpg", FileCategory::Images, 30),
            ],
        );

        assert_eq!(preview.before_groups.len(), 2);
        let pdf_group = preview
            .before_groups
            .iter()
            .find(|group| group.category_key == "pdf")
            .unwrap();
        assert_eq!(pdf_group.file_count, 2);
        assert_eq!(pdf_group.total_size_bytes, 30);

        assert_eq!(preview.after_zones.len(), 2);
        assert!(preview
            .after_zones
            .iter()
            .all(|zone| zone.archive_folder.contains("Desktop Archive")));
        assert!(preview
            .after_zones
            .iter()
            .all(|zone| zone.canvas_rect.width > 0 && zone.canvas_rect.height > 0));
    }

    fn classified_file(
        root: &Path,
        name: &str,
        category: FileCategory,
        size_bytes: u64,
    ) -> ClassificationResult {
        let now = chrono::Utc::now();
        smart_file_organizer_core::ClassificationResult {
            file: smart_file_organizer_core::FileItem {
                id: uuid::Uuid::new_v4(),
                root: root.to_path_buf(),
                path: root.join(name),
                relative_path: PathBuf::from(name),
                file_name: name.to_string(),
                extension: Path::new(name)
                    .extension()
                    .map(|extension| extension.to_string_lossy().into_owned()),
                size_bytes,
                created_at: None,
                modified_at: None,
                accessed_at: None,
                is_hidden: false,
                is_symlink: false,
                mime_type: None,
                path_hash: format!("hash-{name}"),
                indexed_at: now,
            },
            category,
            confidence: 0.9,
            evidence: Vec::new(),
            risk: smart_file_organizer_core::FileRiskLevel::Low,
        }
    }
}
