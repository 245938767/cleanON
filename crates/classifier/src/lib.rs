use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use smart_file_organizer_core::{
    ClassificationContext, ClassificationResult, ClassificationRule, FileCategory, FileItem,
    FileRiskLevel, RuleCondition, RuleField, RuleOperator, Skill,
};

#[async_trait]
pub trait Classifier: Send + Sync {
    async fn classify(
        &self,
        file: &FileItem,
        context: &ClassificationContext,
    ) -> Result<ClassificationResult>;
}

#[derive(Debug, Default, Clone)]
pub struct BasicClassifier;

#[derive(Debug, Clone)]
struct ClassificationCandidate {
    category: FileCategory,
    confidence: f32,
    evidence: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct SkillRule {
    extension: Option<String>,
    file_name_contains: Option<String>,
    mime_prefix: Option<String>,
    category: String,
}

#[async_trait]
impl Classifier for BasicClassifier {
    async fn classify(
        &self,
        file: &FileItem,
        context: &ClassificationContext,
    ) -> Result<ClassificationResult> {
        let candidate = skill_candidate(file, &context.skills)
            .or_else(|| rule_candidate(file, &context.rules))
            .or_else(|| keyword_candidate(file))
            .unwrap_or_else(|| basic_candidate(file));

        Ok(ClassificationResult {
            file: file.clone(),
            category: candidate.category,
            confidence: candidate.confidence,
            evidence: candidate.evidence,
            risk: if file.is_symlink {
                FileRiskLevel::Medium
            } else {
                FileRiskLevel::Low
            },
        })
    }
}

fn skill_candidate(file: &FileItem, skills: &[Skill]) -> Option<ClassificationCandidate> {
    skills
        .iter()
        .filter(|skill| skill.enabled)
        .filter_map(|skill| skill_match(file, skill).map(|category| (skill, category)))
        .next()
        .map(|(skill, category)| ClassificationCandidate {
            category,
            confidence: 0.96,
            evidence: vec![
                format!("命中已启用 Skill：{}", skill.name),
                "Skill 建议优先于内置规则".to_string(),
            ],
        })
}

fn skill_match(file: &FileItem, skill: &Skill) -> Option<FileCategory> {
    let rule: SkillRule = serde_json::from_str(&skill.rule).ok()?;
    let has_condition =
        rule.extension.is_some() || rule.file_name_contains.is_some() || rule.mime_prefix.is_some();
    if !has_condition {
        return None;
    }

    if let Some(expected) = rule.extension {
        if !file
            .extension
            .as_deref()
            .is_some_and(|actual| actual.eq_ignore_ascii_case(expected.trim_start_matches('.')))
        {
            return None;
        }
    }

    if let Some(needle) = rule.file_name_contains {
        if !contains_case_insensitive(&file.file_name, &needle) {
            return None;
        }
    }

    if let Some(prefix) = rule.mime_prefix {
        if !file
            .mime_type
            .as_deref()
            .is_some_and(|mime| mime.starts_with(&prefix))
        {
            return None;
        }
    }

    parse_category(&rule.category)
}

fn rule_candidate(
    file: &FileItem,
    rules: &[ClassificationRule],
) -> Option<ClassificationCandidate> {
    let mut enabled_rules = rules
        .iter()
        .filter(|rule| rule.enabled && !rule.conditions.is_empty())
        .collect::<Vec<_>>();
    enabled_rules.sort_by(|left, right| right.priority.cmp(&left.priority));

    enabled_rules
        .into_iter()
        .find(|rule| {
            rule.conditions
                .iter()
                .all(|condition| matches_condition(file, condition))
        })
        .map(|rule| ClassificationCandidate {
            category: rule.target_category.clone(),
            confidence: 0.92,
            evidence: vec![format!(
                "命中分类规则：{}（优先级 {}）",
                rule.name, rule.priority
            )],
        })
}

fn matches_condition(file: &FileItem, condition: &RuleCondition) -> bool {
    let actual = match condition.field {
        RuleField::FileName => file.file_name.as_str(),
        RuleField::Extension => file.extension.as_deref().unwrap_or_default(),
        RuleField::MimeType => file.mime_type.as_deref().unwrap_or_default(),
        RuleField::RelativePath => file.relative_path.to_str().unwrap_or_default(),
    };

    match condition.operator {
        RuleOperator::Equals => value_as_str(&condition.value)
            .is_some_and(|expected| actual.eq_ignore_ascii_case(expected.trim_start_matches('.'))),
        RuleOperator::Contains => value_as_str(&condition.value)
            .is_some_and(|expected| contains_case_insensitive(actual, expected)),
        RuleOperator::StartsWith => value_as_str(&condition.value)
            .is_some_and(|expected| actual.to_lowercase().starts_with(&expected.to_lowercase())),
        RuleOperator::EndsWith => value_as_str(&condition.value)
            .is_some_and(|expected| actual.to_lowercase().ends_with(&expected.to_lowercase())),
        RuleOperator::In => condition.value.as_array().is_some_and(|values| {
            values
                .iter()
                .filter_map(value_as_str)
                .any(|expected| actual.eq_ignore_ascii_case(expected.trim_start_matches('.')))
        }),
    }
}

fn keyword_candidate(file: &FileItem) -> Option<ClassificationCandidate> {
    let file_name = file.file_name.to_lowercase();
    let rules: &[(&[&str], FileCategory, &str)] = &[
        (
            &["screenshot", "截屏", "截图"],
            FileCategory::Images,
            "文件名包含截图关键词",
        ),
        (
            &["invoice", "receipt", "发票", "票据", "收据"],
            FileCategory::Documents,
            "文件名包含票据关键词",
        ),
        (
            &["contract", "agreement", "合同", "协议"],
            FileCategory::Documents,
            "文件名包含合同关键词",
        ),
        (
            &["meeting", "minutes", "会议纪要"],
            FileCategory::Documents,
            "文件名包含会议纪要关键词",
        ),
    ];

    rules
        .iter()
        .find(|(needles, _, _)| needles.iter().any(|needle| file_name.contains(needle)))
        .map(|(_, category, evidence)| ClassificationCandidate {
            category: category.clone(),
            confidence: 0.88,
            evidence: vec![evidence.to_string()],
        })
}

fn basic_candidate(file: &FileItem) -> ClassificationCandidate {
    let extension = file.extension.as_deref().unwrap_or_default();
    let mime = file.mime_type.as_deref().unwrap_or_default();
    let (category, confidence, evidence) = classify_extension_or_mime(extension, mime);

    ClassificationCandidate {
        category,
        confidence,
        evidence,
    }
}

fn classify_extension_or_mime(extension: &str, mime: &str) -> (FileCategory, f32, Vec<String>) {
    let normalized = extension.trim_start_matches('.').to_lowercase();
    let category = match normalized.as_str() {
        "pdf" => FileCategory::Pdf,
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "tiff" | "bmp" | "svg" => {
            FileCategory::Images
        }
        "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" => FileCategory::Videos,
        "mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg" => FileCategory::Audio,
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => FileCategory::Archives,
        "dmg" | "pkg" | "exe" | "msi" | "appimage" | "deb" | "rpm" => FileCategory::Installers,
        "rs" | "js" | "ts" | "tsx" | "jsx" | "py" | "go" | "java" | "kt" | "swift" | "c"
        | "cpp" | "h" | "hpp" | "html" | "css" | "json" | "toml" | "yaml" | "yml" | "sql"
        | "sh" => FileCategory::Code,
        "xls" | "xlsx" | "csv" | "tsv" | "numbers" => FileCategory::Spreadsheets,
        "ppt" | "pptx" | "key" => FileCategory::Presentations,
        "txt" | "md" | "doc" | "docx" | "rtf" | "pages" => FileCategory::Documents,
        _ => mime_category(mime).unwrap_or(FileCategory::Other),
    };

    if !normalized.is_empty() && category != FileCategory::Other {
        return (
            category,
            0.85,
            vec![format!("扩展名 .{normalized} 命中本地基础分类规则")],
        );
    }

    if !mime.is_empty() && category != FileCategory::Other {
        return (
            category,
            0.82,
            vec![format!("MIME 类型 {mime} 命中本地基础分类规则")],
        );
    }

    (
        FileCategory::Other,
        0.2,
        vec!["未命中扩展名、MIME、关键词或 Skill，归入其他".to_string()],
    )
}

fn mime_category(mime: &str) -> Option<FileCategory> {
    if mime == "application/pdf" {
        return Some(FileCategory::Pdf);
    }
    if mime.starts_with("image/") {
        return Some(FileCategory::Images);
    }
    if mime.starts_with("video/") {
        return Some(FileCategory::Videos);
    }
    if mime.starts_with("audio/") {
        return Some(FileCategory::Audio);
    }
    if mime.starts_with("text/") {
        return Some(FileCategory::Documents);
    }
    None
}

fn parse_category(category: &str) -> Option<FileCategory> {
    match category.trim().to_lowercase().as_str() {
        "documents" | "document" | "docs" | "文档" => Some(FileCategory::Documents),
        "images" | "image" | "图片" | "图像" => Some(FileCategory::Images),
        "videos" | "video" | "视频" => Some(FileCategory::Videos),
        "audio" | "音频" => Some(FileCategory::Audio),
        "archives" | "archive" | "压缩包" | "归档" => Some(FileCategory::Archives),
        "installers" | "installer" | "安装包" => Some(FileCategory::Installers),
        "code" | "代码" => Some(FileCategory::Code),
        "spreadsheets" | "spreadsheet" | "表格" => Some(FileCategory::Spreadsheets),
        "presentations" | "presentation" | "演示文稿" => Some(FileCategory::Presentations),
        "pdf" => Some(FileCategory::Pdf),
        "other" | "其他" => Some(FileCategory::Other),
        _ => None,
    }
}

fn contains_case_insensitive(value: &str, needle: &str) -> bool {
    value.to_lowercase().contains(&needle.to_lowercase())
}

fn value_as_str(value: &Value) -> Option<&str> {
    value.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;
    use std::path::{Path, PathBuf};
    use uuid::Uuid;

    fn file(name: &str) -> FileItem {
        let now = Utc::now();
        let path = PathBuf::from("/tmp").join(name);
        let extension = Path::new(name)
            .extension()
            .map(|extension| extension.to_string_lossy().into_owned());
        FileItem {
            id: Uuid::new_v4(),
            root: PathBuf::from("/tmp"),
            path,
            relative_path: PathBuf::from(name),
            file_name: name.to_string(),
            extension,
            size_bytes: 1,
            created_at: Some(now),
            modified_at: Some(now),
            accessed_at: Some(now),
            is_hidden: false,
            is_symlink: false,
            mime_type: None,
            path_hash: format!("hash-{name}"),
            indexed_at: now,
        }
    }

    fn context() -> ClassificationContext {
        ClassificationContext {
            root_path: PathBuf::from("/tmp"),
            ..ClassificationContext::default()
        }
    }

    #[tokio::test]
    async fn classifies_core_extensions_with_chinese_evidence() {
        let cases = [
            ("photo.jpg", FileCategory::Images),
            ("report.pdf", FileCategory::Pdf),
            ("backup.zip", FileCategory::Archives),
            ("installer.dmg", FileCategory::Installers),
            ("main.rs", FileCategory::Code),
        ];

        for (name, expected) in cases {
            let result = BasicClassifier
                .classify(&file(name), &context())
                .await
                .unwrap();

            assert_eq!(result.category, expected);
            assert!(result.confidence > 0.8);
            assert!(result.evidence[0].contains("扩展名"));
        }
    }

    #[tokio::test]
    async fn classifies_file_name_keywords_before_extension() {
        let result = BasicClassifier
            .classify(&file("发票-三月.pdf"), &context())
            .await
            .unwrap();

        assert_eq!(result.category, FileCategory::Documents);
        assert!(result
            .evidence
            .iter()
            .any(|evidence| evidence.contains("票据")));

        let screenshot = BasicClassifier
            .classify(&file("Screenshot 2026-05-01.png"), &context())
            .await
            .unwrap();
        assert_eq!(screenshot.category, FileCategory::Images);
        assert!(screenshot
            .evidence
            .iter()
            .any(|evidence| evidence.contains("截图")));
    }

    #[tokio::test]
    async fn enabled_classification_rule_uses_highest_priority() {
        let mut context = context();
        context.rules = vec![
            ClassificationRule {
                rule_id: "disabled".to_string(),
                name: "禁用规则".to_string(),
                priority: 200,
                enabled: false,
                conditions: vec![RuleCondition {
                    field: RuleField::Extension,
                    operator: RuleOperator::Equals,
                    value: json!("pdf"),
                }],
                target_category: FileCategory::Archives,
            },
            ClassificationRule {
                rule_id: "contract".to_string(),
                name: "合同归档".to_string(),
                priority: 100,
                enabled: true,
                conditions: vec![RuleCondition {
                    field: RuleField::FileName,
                    operator: RuleOperator::Contains,
                    value: json!("合同"),
                }],
                target_category: FileCategory::Documents,
            },
        ];

        let result = BasicClassifier
            .classify(&file("客户合同.pdf"), &context)
            .await
            .unwrap();

        assert_eq!(result.category, FileCategory::Documents);
        assert!(result.evidence[0].contains("合同归档"));
    }

    #[tokio::test]
    async fn enabled_skill_overrides_rules_and_empty_matcher_is_ignored() {
        let mut context = context();
        context.rules = vec![ClassificationRule {
            rule_id: "pdf".to_string(),
            name: "PDF 规则".to_string(),
            priority: 100,
            enabled: true,
            conditions: vec![RuleCondition {
                field: RuleField::Extension,
                operator: RuleOperator::Equals,
                value: json!("pdf"),
            }],
            target_category: FileCategory::Pdf,
        }];
        context.skills = vec![
            Skill {
                id: Uuid::new_v4(),
                name: "空 Skill".to_string(),
                enabled: true,
                rule: json!({"category": "Archives"}).to_string(),
                created_at: Utc::now(),
            },
            Skill {
                id: Uuid::new_v4(),
                name: "发票进入文档".to_string(),
                enabled: true,
                rule: json!({
                    "extension": "pdf",
                    "file_name_contains": "发票",
                    "category": "Documents"
                })
                .to_string(),
                created_at: Utc::now(),
            },
        ];

        let result = BasicClassifier
            .classify(&file("发票.pdf"), &context)
            .await
            .unwrap();

        assert_eq!(result.category, FileCategory::Documents);
        assert!(result.confidence > 0.9);
        assert!(result.evidence[0].contains("Skill"));
    }
}
