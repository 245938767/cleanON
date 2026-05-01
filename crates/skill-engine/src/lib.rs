use smart_file_organizer_core::{
    FileCategory, Skill, SkillRule, SkillUpdateProposal, UserDecision, UserDecisionEvent,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    pub file_name: String,
    pub extension: Option<String>,
    pub mime: Option<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSuggestion {
    pub skill_id: String,
    pub skill_name: String,
    pub category: FileCategory,
    pub destination_hint: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillEngine {
    skills: Vec<Skill>,
}

impl SkillEngine {
    pub fn new(skills: Vec<Skill>) -> Self {
        Self { skills }
    }

    pub fn suggest(&self, file: &FileFacts) -> Vec<SkillSuggestion> {
        self.skills
            .iter()
            .filter(|skill| skill.enabled)
            .filter(|skill| rule_matches(&skill.rule, file))
            .map(|skill| SkillSuggestion {
                skill_id: skill.id.to_string(),
                skill_name: skill.name.clone(),
                category: skill.rule.category.clone(),
                destination_hint: skill.rule.destination_hint.clone(),
                evidence: skill_evidence(skill),
            })
            .collect()
    }
}

pub fn propose_skill_updates(events: &[UserDecisionEvent]) -> Vec<SkillUpdateProposal> {
    events
        .iter()
        .filter_map(proposal_from_event)
        .collect::<Vec<_>>()
}

fn proposal_from_event(event: &UserDecisionEvent) -> Option<SkillUpdateProposal> {
    if event.decision == UserDecision::Rejected {
        return None;
    }

    let category = event.final_category.clone()?;
    let destination_hint = destination_hint(event);

    if let Some(extension) = normalized_extension(event.extension.as_deref()) {
        return Some(SkillUpdateProposal {
            name: format!(".{extension} 文件进入 {}", category.folder_name()),
            rule: SkillRule {
                extension: Some(extension.clone()),
                file_name_contains: None,
                mime_prefix: event.mime_type.as_deref().and_then(mime_prefix),
                category,
                destination_hint,
            },
            enabled: true,
            evidence: vec![format!(
                "用户决策 {:?}：同扩展名 .{extension}",
                event.decision
            )],
            source_event_ids: vec![event.event_id],
        });
    }

    let keyword = file_name_keyword(&event.file_name)?;
    Some(SkillUpdateProposal {
        name: format!("文件名包含 {keyword} 进入 {}", category.folder_name()),
        rule: SkillRule {
            extension: None,
            file_name_contains: Some(keyword.clone()),
            mime_prefix: event.mime_type.as_deref().and_then(mime_prefix),
            category,
            destination_hint,
        },
        enabled: true,
        evidence: vec![format!(
            "用户决策 {:?}：文件名关键词 {keyword}",
            event.decision
        )],
        source_event_ids: vec![event.event_id],
    })
}

pub fn rule_matches(rule: &SkillRule, file: &FileFacts) -> bool {
    let mut has_condition = false;

    if let Some(expected) = normalized_extension(rule.extension.as_deref()) {
        has_condition = true;
        if !file
            .extension
            .as_deref()
            .and_then(|extension| normalized_extension(Some(extension)))
            .is_some_and(|actual| actual.eq_ignore_ascii_case(&expected))
        {
            return false;
        }
    }

    if let Some(needle) = rule
        .file_name_contains
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        has_condition = true;
        if !contains_case_insensitive(&file.file_name, needle) {
            return false;
        }
    }

    if let Some(prefix) = rule
        .mime_prefix
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        has_condition = true;
        if !file
            .mime
            .as_deref()
            .is_some_and(|mime| mime.starts_with(prefix))
        {
            return false;
        }
    }

    has_condition
}

fn skill_evidence(skill: &Skill) -> Vec<String> {
    let mut evidence = vec![format!("命中已启用 Skill：{}", skill.name)];
    if let Some(extension) = normalized_extension(skill.rule.extension.as_deref()) {
        evidence.push(format!("Skill 条件：扩展名 .{extension}"));
    }
    if let Some(needle) = skill.rule.file_name_contains.as_deref() {
        evidence.push(format!("Skill 条件：文件名包含 {needle}"));
    }
    if let Some(prefix) = skill.rule.mime_prefix.as_deref() {
        evidence.push(format!("Skill 条件：MIME 前缀 {prefix}"));
    }
    if let Some(destination_hint) = skill.rule.destination_hint.as_deref() {
        evidence.push(format!("Skill 目标文件夹提示：{destination_hint}"));
    }
    evidence
}

fn normalized_extension(value: Option<&str>) -> Option<String> {
    value
        .map(|value| value.trim().trim_start_matches('.').to_lowercase())
        .filter(|value| !value.is_empty())
}

fn mime_prefix(value: &str) -> Option<String> {
    value
        .split_once('/')
        .map(|(prefix, _)| format!("{prefix}/"))
        .filter(|prefix| prefix != "/")
}

fn destination_hint(event: &UserDecisionEvent) -> Option<String> {
    let destination = event.final_destination.as_ref()?;
    let final_name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())?;

    if final_name == event.file_name {
        return destination
            .parent()
            .and_then(|parent| parent.file_name())
            .map(|name| name.to_string_lossy().into_owned());
    }

    Some(final_name)
}

fn file_name_keyword(file_name: &str) -> Option<String> {
    let stem = file_name.split('.').next().unwrap_or(file_name).trim();
    stem.split(|character: char| !character.is_alphanumeric())
        .find(|part| part.chars().count() >= 2)
        .map(str::to_lowercase)
}

fn contains_case_insensitive(value: &str, needle: &str) -> bool {
    value.to_lowercase().contains(&needle.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn returns_structured_suggestions_for_matching_enabled_skills() {
        let engine = SkillEngine::new(vec![
            Skill {
                id: Uuid::new_v4(),
                name: "PDF 文件".to_string(),
                enabled: true,
                rule: SkillRule {
                    extension: Some("pdf".to_string()),
                    category: FileCategory::Documents,
                    destination_hint: Some("文档/PDF".to_string()),
                    ..SkillRule::default()
                },
                created_at: Utc::now(),
            },
            Skill {
                id: Uuid::new_v4(),
                name: "Disabled".to_string(),
                enabled: false,
                rule: SkillRule {
                    extension: Some("pdf".to_string()),
                    category: FileCategory::Archives,
                    ..SkillRule::default()
                },
                created_at: Utc::now(),
            },
        ]);

        let suggestions = engine.suggest(&FileFacts {
            file_name: "invoice.pdf".to_string(),
            extension: Some("PDF".to_string()),
            mime: Some("application/pdf".to_string()),
            size_bytes: 1024,
        });

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].skill_name, "PDF 文件");
        assert_eq!(suggestions[0].category, FileCategory::Documents);
        assert!(suggestions[0]
            .evidence
            .iter()
            .any(|evidence| evidence.contains("扩展名 .pdf")));
    }

    #[test]
    fn empty_matcher_does_not_match_everything() {
        let engine = SkillEngine::new(vec![Skill {
            id: Uuid::new_v4(),
            name: "Empty".to_string(),
            enabled: true,
            rule: SkillRule {
                category: FileCategory::Other,
                ..SkillRule::default()
            },
            created_at: Utc::now(),
        }]);

        assert!(engine
            .suggest(&FileFacts {
                file_name: "a.txt".to_string(),
                extension: Some("txt".to_string()),
                mime: Some("text/plain".to_string()),
                size_bytes: 1,
            })
            .is_empty());
    }

    #[test]
    fn proposes_extension_skill_from_edited_destination_event() {
        let event_id = Uuid::new_v4();
        let proposals = propose_skill_updates(&[UserDecisionEvent {
            event_id,
            file_name: "invoice.pdf".to_string(),
            extension: Some("PDF".to_string()),
            mime_type: Some("application/pdf".to_string()),
            decision: UserDecision::EditedDestination,
            original_category: Some(FileCategory::Pdf),
            final_category: Some(FileCategory::Documents),
            original_destination: Some(PathBuf::from("/tmp/PDF/invoice.pdf")),
            final_destination: Some(PathBuf::from("/tmp/Invoices/invoice.pdf")),
            occurred_at: Utc::now(),
        }]);

        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].rule.extension, Some("pdf".to_string()));
        assert_eq!(proposals[0].rule.category, FileCategory::Documents);
        assert_eq!(
            proposals[0].rule.destination_hint,
            Some("Invoices".to_string())
        );
        assert_eq!(proposals[0].source_event_ids, vec![event_id]);
    }

    #[test]
    fn rejected_events_do_not_create_positive_rules() {
        let proposals = propose_skill_updates(&[UserDecisionEvent {
            event_id: Uuid::new_v4(),
            file_name: "draft".to_string(),
            extension: None,
            mime_type: None,
            decision: UserDecision::Rejected,
            original_category: Some(FileCategory::Documents),
            final_category: None,
            original_destination: None,
            final_destination: None,
            occurred_at: Utc::now(),
        }]);

        assert!(proposals.is_empty());
    }
}
