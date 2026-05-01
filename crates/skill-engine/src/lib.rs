use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub matcher: SkillMatcher,
    pub suggestion: SkillSuggestionTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SkillMatcher {
    pub extension: Option<String>,
    pub file_name_contains: Option<String>,
    pub mime_prefix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSuggestionTemplate {
    pub category: String,
    pub destination_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFacts {
    pub file_name: String,
    pub extension: Option<String>,
    pub mime: Option<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSuggestion {
    pub skill_id: String,
    pub skill_name: String,
    pub category: String,
    pub destination_hint: Option<String>,
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
            .filter(|skill| skill.enabled && skill.matcher.matches(file))
            .map(|skill| SkillSuggestion {
                skill_id: skill.id.clone(),
                skill_name: skill.name.clone(),
                category: skill.suggestion.category.clone(),
                destination_hint: skill.suggestion.destination_hint.clone(),
            })
            .collect()
    }
}

impl SkillMatcher {
    fn matches(&self, file: &FileFacts) -> bool {
        let mut has_condition = false;

        if let Some(expected) = &self.extension {
            has_condition = true;
            if !file
                .extension
                .as_deref()
                .is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
            {
                return false;
            }
        }

        if let Some(needle) = &self.file_name_contains {
            has_condition = true;
            if !file
                .file_name
                .to_lowercase()
                .contains(&needle.to_lowercase())
            {
                return false;
            }
        }

        if let Some(prefix) = &self.mime_prefix {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_structured_suggestions_for_matching_enabled_skills() {
        let engine = SkillEngine::new(vec![
            Skill {
                id: "skill-pdf".to_string(),
                name: "PDF 文件".to_string(),
                enabled: true,
                matcher: SkillMatcher {
                    extension: Some("pdf".to_string()),
                    ..SkillMatcher::default()
                },
                suggestion: SkillSuggestionTemplate {
                    category: "Documents".to_string(),
                    destination_hint: Some("文档/PDF".to_string()),
                },
            },
            Skill {
                id: "skill-disabled".to_string(),
                name: "Disabled".to_string(),
                enabled: false,
                matcher: SkillMatcher {
                    extension: Some("pdf".to_string()),
                    ..SkillMatcher::default()
                },
                suggestion: SkillSuggestionTemplate {
                    category: "Ignored".to_string(),
                    destination_hint: None,
                },
            },
        ]);

        let suggestions = engine.suggest(&FileFacts {
            file_name: "invoice.pdf".to_string(),
            extension: Some("PDF".to_string()),
            mime: Some("application/pdf".to_string()),
            size_bytes: 1024,
        });

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].skill_id, "skill-pdf");
        assert_eq!(suggestions[0].category, "Documents");
    }

    #[test]
    fn empty_matcher_does_not_match_everything() {
        let engine = SkillEngine::new(vec![Skill {
            id: "empty".to_string(),
            name: "Empty".to_string(),
            enabled: true,
            matcher: SkillMatcher::default(),
            suggestion: SkillSuggestionTemplate {
                category: "Other".to_string(),
                destination_hint: None,
            },
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
}
