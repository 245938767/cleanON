use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

const DEFAULT_OPENAI_COMPATIBLE_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";

#[derive(Clone)]
pub struct ProviderCredentials {
    api_key: String,
}

impl ProviderCredentials {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }

    pub fn expose_for_provider_call(&self) -> &str {
        &self.api_key
    }
}

impl std::fmt::Debug for ProviderCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderCredentials")
            .field("api_key", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiGatewayRequest {
    pub prompt: String,
    pub files: Vec<AiFileInput>,
    pub cloud_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiFileInput {
    pub absolute_path: String,
    pub extension: Option<String>,
    pub mime: Option<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizedAiRequest {
    pub prompt: String,
    pub files: Vec<SanitizedFileInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizedFileInput {
    pub token: String,
    pub extension: Option<String>,
    pub mime: Option<String>,
    pub size_bucket: SizeBucket,
    pub path_depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeBucket {
    Empty,
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredPlanSuggestion {
    pub provider: String,
    pub summary: String,
    pub categories: Vec<CategorySuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategorySuggestion {
    pub file_token: String,
    pub category: String,
    pub confidence: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub cloud_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    pub provider: String,
    pub label: String,
    pub requires_base_url: bool,
    pub requires_api_key: bool,
    pub cloud: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Mock,
    Ollama,
    OpenAiCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConnectionTest {
    pub provider: String,
    pub request_valid: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderHttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<ProviderHttpHeader>,
    pub body: serde_json::Value,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProviderHttpHeader {
    pub name: String,
    value: String,
}

impl ProviderHttpHeader {
    fn public(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    fn secret(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn expose_for_provider_call(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Debug for ProviderHttpHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = if self.name.eq_ignore_ascii_case("authorization")
            || self.name.to_ascii_lowercase().contains("key")
        {
            "<redacted>"
        } else {
            self.value.as_str()
        };
        f.debug_struct("ProviderHttpHeader")
            .field("name", &self.name)
            .field("value", &value)
            .finish()
    }
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn suggest(
        &self,
        request: SanitizedAiRequest,
        credentials: Option<&ProviderCredentials>,
    ) -> anyhow::Result<StructuredPlanSuggestion>;
}

pub struct AiGateway<P> {
    provider: P,
}

impl<P> AiGateway<P>
where
    P: AiProvider,
{
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    pub async fn suggest(
        &self,
        request: AiGatewayRequest,
        credentials: Option<&ProviderCredentials>,
    ) -> anyhow::Result<StructuredPlanSuggestion> {
        let sanitized = desensitize_request(&request);
        self.provider.suggest(sanitized, credentials).await
    }
}

pub fn provider_registry() -> Vec<ProviderDescriptor> {
    vec![
        ProviderDescriptor {
            provider: "mock".to_string(),
            label: "Mock".to_string(),
            requires_base_url: false,
            requires_api_key: false,
            cloud: false,
        },
        ProviderDescriptor {
            provider: "ollama".to_string(),
            label: "Ollama".to_string(),
            requires_base_url: false,
            requires_api_key: false,
            cloud: false,
        },
        ProviderDescriptor {
            provider: "openai-compatible".to_string(),
            label: "OpenAI-compatible".to_string(),
            requires_base_url: true,
            requires_api_key: true,
            cloud: true,
        },
    ]
}

pub fn parse_provider_kind(provider: &str) -> anyhow::Result<ProviderKind> {
    match provider {
        "mock" => Ok(ProviderKind::Mock),
        "ollama" => Ok(ProviderKind::Ollama),
        "openai-compatible" | "openai_compatible" | "deepseek" | "kimi" => {
            Ok(ProviderKind::OpenAiCompatible)
        }
        other => anyhow::bail!("unsupported AI provider: {other}"),
    }
}

pub fn validate_provider_config(config: &ProviderConfig) -> anyhow::Result<ProviderKind> {
    let kind = parse_provider_kind(&config.provider)?;
    if matches!(kind, ProviderKind::OpenAiCompatible) && !config.cloud_enabled {
        anyhow::bail!("cloud provider requires explicit cloud_enabled=true");
    }
    if matches!(kind, ProviderKind::OpenAiCompatible)
        && config
            .base_url
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
    {
        anyhow::bail!("openai-compatible provider requires base_url");
    }
    Ok(kind)
}

pub fn build_provider_request(
    config: &ProviderConfig,
    request: &SanitizedAiRequest,
    credentials: Option<&ProviderCredentials>,
) -> anyhow::Result<Option<ProviderHttpRequest>> {
    match validate_provider_config(config)? {
        ProviderKind::Mock => Ok(None),
        ProviderKind::Ollama => build_ollama_request(config, request).map(Some),
        ProviderKind::OpenAiCompatible => {
            build_openai_compatible_request(config, request, credentials).map(Some)
        }
    }
}

pub fn test_provider_connection(
    config: &ProviderConfig,
    credentials: Option<&ProviderCredentials>,
) -> anyhow::Result<ProviderConnectionTest> {
    let request = SanitizedAiRequest {
        prompt: "Return a valid JSON suggestion.".to_string(),
        files: vec![SanitizedFileInput {
            token: "file_test".to_string(),
            extension: Some("txt".to_string()),
            mime: Some("text/plain".to_string()),
            size_bucket: SizeBucket::Small,
            path_depth: 1,
        }],
    };
    let request_shape = build_provider_request(config, &request, credentials)?;
    Ok(ProviderConnectionTest {
        provider: config.provider.clone(),
        request_valid: true,
        message: match request_shape {
            Some(shape) => format!("request shape valid: {} {}", shape.method, shape.url),
            None => "mock provider ready".to_string(),
        },
    })
}

pub fn parse_provider_response(
    provider: &str,
    raw: &str,
    request: &SanitizedAiRequest,
) -> anyhow::Result<StructuredPlanSuggestion> {
    let kind = parse_provider_kind(provider)?;
    let structured_raw = match kind {
        ProviderKind::Mock => raw.to_string(),
        ProviderKind::OpenAiCompatible => {
            let value: serde_json::Value = serde_json::from_str(raw)
                .map_err(|error| anyhow::anyhow!("provider response is invalid JSON: {error}"))?;
            value
                .pointer("/choices/0/message/content")
                .and_then(|content| content.as_str())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "provider response missing choices[0].message.content JSON string"
                    )
                })?
                .to_string()
        }
        ProviderKind::Ollama => {
            let value: serde_json::Value = serde_json::from_str(raw)
                .map_err(|error| anyhow::anyhow!("provider response is invalid JSON: {error}"))?;
            value
                .get("response")
                .and_then(|content| content.as_str())
                .ok_or_else(|| anyhow::anyhow!("provider response missing response JSON string"))?
                .to_string()
        }
    };

    validate_structured_suggestion(provider, &structured_raw, request)
}

pub fn validate_structured_suggestion(
    provider: &str,
    raw: &str,
    request: &SanitizedAiRequest,
) -> anyhow::Result<StructuredPlanSuggestion> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|error| anyhow::anyhow!("structured suggestion is invalid JSON: {error}"))?;
    let summary = value
        .get("summary")
        .and_then(|summary| summary.as_str())
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .ok_or_else(|| anyhow::anyhow!("structured suggestion missing non-empty summary"))?
        .to_string();
    let categories = value
        .get("categories")
        .and_then(|categories| categories.as_array())
        .ok_or_else(|| anyhow::anyhow!("structured suggestion missing categories array"))?;
    let known_tokens = request
        .files
        .iter()
        .map(|file| file.token.as_str())
        .collect::<std::collections::HashSet<_>>();

    let categories = categories
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let file_token = value
                .get("file_token")
                .and_then(|token| token.as_str())
                .map(str::trim)
                .filter(|token| !token.is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!("category suggestion at index {index} missing file_token")
                })?;
            if !known_tokens.contains(file_token) {
                anyhow::bail!("category suggestion at index {index} references unknown file_token");
            }
            let category = value
                .get("category")
                .and_then(|category| category.as_str())
                .map(str::trim)
                .filter(|category| !category.is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!("category suggestion at index {index} missing category")
                })?
                .to_string();
            let confidence = value
                .get("confidence")
                .and_then(|confidence| confidence.as_u64())
                .ok_or_else(|| {
                    anyhow::anyhow!("category suggestion at index {index} missing confidence")
                })?;
            if confidence > 100 {
                anyhow::bail!("category suggestion at index {index} confidence must be 0..=100");
            }
            Ok(CategorySuggestion {
                file_token: file_token.to_string(),
                category,
                confidence: confidence as u8,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(StructuredPlanSuggestion {
        provider: provider.to_string(),
        summary,
        categories,
    })
}

#[derive(Debug, Clone, Default)]
pub struct MockProvider;

#[async_trait]
impl AiProvider for MockProvider {
    async fn suggest(
        &self,
        request: SanitizedAiRequest,
        _credentials: Option<&ProviderCredentials>,
    ) -> anyhow::Result<StructuredPlanSuggestion> {
        let categories = request
            .files
            .into_iter()
            .map(|file| CategorySuggestion {
                category: category_for_extension(file.extension.as_deref()),
                confidence: 70,
                file_token: file.token,
            })
            .collect();

        Ok(StructuredPlanSuggestion {
            provider: "mock".to_string(),
            summary: "Mock provider generated classification suggestions only.".to_string(),
            categories,
        })
    }
}

pub fn desensitize_request(request: &AiGatewayRequest) -> SanitizedAiRequest {
    SanitizedAiRequest {
        prompt: redact_secrets(&request.prompt),
        files: request
            .files
            .iter()
            .map(|file| SanitizedFileInput {
                token: stable_file_token(&file.absolute_path),
                extension: file.extension.as_ref().map(|ext| ext.to_lowercase()),
                mime: file.mime.clone(),
                size_bucket: SizeBucket::from_bytes(file.size_bytes),
                path_depth: file
                    .absolute_path
                    .split('/')
                    .filter(|part| !part.is_empty())
                    .count(),
            })
            .collect(),
    }
}

fn build_openai_compatible_request(
    config: &ProviderConfig,
    request: &SanitizedAiRequest,
    credentials: Option<&ProviderCredentials>,
) -> anyhow::Result<ProviderHttpRequest> {
    let api_key = credentials
        .map(ProviderCredentials::expose_for_provider_call)
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("openai-compatible provider requires runtime API key"))?;
    let base_url = config
        .base_url
        .as_deref()
        .unwrap_or(DEFAULT_OPENAI_COMPATIBLE_BASE_URL);
    Ok(ProviderHttpRequest {
        method: "POST".to_string(),
        url: format!("{}/chat/completions", base_url.trim_end_matches('/')),
        headers: vec![
            ProviderHttpHeader::public("Content-Type", "application/json"),
            ProviderHttpHeader::secret("Authorization", format!("Bearer {api_key}")),
        ],
        body: json!({
            "model": config.model.as_deref().unwrap_or("default"),
            "messages": [
                {
                    "role": "system",
                    "content": "Return only JSON with summary and categories[{file_token,category,confidence}]. Do not include paths or file contents."
                },
                {
                    "role": "user",
                    "content": serde_json::to_string(request)?
                }
            ],
            "response_format": {"type": "json_object"},
            "temperature": 0.1
        }),
    })
}

fn build_ollama_request(
    config: &ProviderConfig,
    request: &SanitizedAiRequest,
) -> anyhow::Result<ProviderHttpRequest> {
    let base_url = config
        .base_url
        .as_deref()
        .unwrap_or(DEFAULT_OLLAMA_BASE_URL);
    Ok(ProviderHttpRequest {
        method: "POST".to_string(),
        url: format!("{}/api/generate", base_url.trim_end_matches('/')),
        headers: vec![ProviderHttpHeader::public(
            "Content-Type",
            "application/json",
        )],
        body: json!({
            "model": config.model.as_deref().unwrap_or("llama3.2"),
            "prompt": format!(
                "Return only JSON with summary and categories[{{file_token,category,confidence}}] for this sanitized request: {}",
                serde_json::to_string(request)?
            ),
            "stream": false,
            "format": "json",
            "options": {"temperature": 0.1}
        }),
    })
}

impl SizeBucket {
    fn from_bytes(size_bytes: u64) -> Self {
        match size_bytes {
            0 => Self::Empty,
            1..=1_048_576 => Self::Small,
            1_048_577..=104_857_600 => Self::Medium,
            _ => Self::Large,
        }
    }
}

fn stable_file_token(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    let digest = hasher.finalize();
    let hex = digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("file_{hex}")
}

fn category_for_extension(extension: Option<&str>) -> String {
    match extension.unwrap_or_default().to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "webp" => "Images",
        "pdf" | "doc" | "docx" | "txt" | "md" => "Documents",
        "mp4" | "mov" | "avi" => "Videos",
        "mp3" | "wav" | "flac" => "Audio",
        _ => "Other",
    }
    .to_string()
}

fn redact_secrets(input: &str) -> String {
    input
        .split_whitespace()
        .map(|part| {
            let lower = part.to_ascii_lowercase();
            if lower.contains("api_key=")
                || lower.contains("apikey=")
                || lower.contains("authorization:")
                || lower.starts_with("sk-")
                || looks_like_absolute_path(part)
            {
                "<redacted>"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_absolute_path(input: &str) -> bool {
    input.starts_with('/')
        || input.starts_with('~')
        || input.as_bytes().get(1).is_some_and(|byte| *byte == b':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desensitization_removes_absolute_paths_and_prompt_secrets() {
        let request = AiGatewayRequest {
            prompt: "sort these api_key=secret sk-live-value".to_string(),
            cloud_enabled: true,
            files: vec![AiFileInput {
                absolute_path: "/Users/alice/Private/Taxes/2026-return.pdf".to_string(),
                extension: Some("PDF".to_string()),
                mime: Some("application/pdf".to_string()),
                size_bytes: 2048,
            }],
        };

        let sanitized = desensitize_request(&request);
        let serialized = serde_json::to_string(&sanitized).unwrap();

        assert!(!serialized.contains("/Users/alice"));
        assert!(!serialized.contains("2026-return"));
        assert!(!serialized.contains("secret"));
        assert!(!serialized.contains("sk-live-value"));
        assert_eq!(sanitized.files[0].extension.as_deref(), Some("pdf"));
    }

    #[tokio::test]
    async fn mock_provider_returns_structured_suggestions_only() {
        let gateway = AiGateway::new(MockProvider);
        let result = gateway
            .suggest(
                AiGatewayRequest {
                    prompt: "organize files".to_string(),
                    cloud_enabled: false,
                    files: vec![AiFileInput {
                        absolute_path: "/tmp/a.jpg".to_string(),
                        extension: Some("jpg".to_string()),
                        mime: Some("image/jpeg".to_string()),
                        size_bytes: 3_000,
                    }],
                },
                Some(&ProviderCredentials::new("sk-test")),
            )
            .await
            .unwrap();

        assert_eq!(result.provider, "mock");
        assert_eq!(result.categories[0].category, "Images");
    }

    #[test]
    fn credentials_debug_output_redacts_api_key() {
        let credentials = ProviderCredentials::new("sk-test-secret");
        let debug = format!("{credentials:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("sk-test-secret"));
    }

    #[test]
    fn registry_lists_supported_mvp_providers() {
        let providers = provider_registry()
            .into_iter()
            .map(|provider| provider.provider)
            .collect::<Vec<_>>();

        assert_eq!(providers, vec!["mock", "ollama", "openai-compatible"]);
        assert_eq!(
            parse_provider_kind("deepseek").unwrap(),
            ProviderKind::OpenAiCompatible
        );
        assert_eq!(
            parse_provider_kind("kimi").unwrap(),
            ProviderKind::OpenAiCompatible
        );
    }

    #[test]
    fn openai_compatible_request_uses_sanitized_payload_and_runtime_key() {
        let raw = AiGatewayRequest {
            prompt: "organize /Users/alice/Secret api_key=hidden".to_string(),
            cloud_enabled: true,
            files: vec![AiFileInput {
                absolute_path: "/Users/alice/Secret/tax.pdf".to_string(),
                extension: Some("PDF".to_string()),
                mime: Some("application/pdf".to_string()),
                size_bytes: 2048,
            }],
        };
        let sanitized = desensitize_request(&raw);
        let request = build_provider_request(
            &ProviderConfig {
                provider: "openai-compatible".to_string(),
                base_url: Some("https://api.deepseek.example/v1/".to_string()),
                model: Some("deepseek-chat".to_string()),
                cloud_enabled: true,
            },
            &sanitized,
            Some(&ProviderCredentials::new("sk-runtime-only")),
        )
        .unwrap()
        .unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(
            request.url,
            "https://api.deepseek.example/v1/chat/completions"
        );
        assert_eq!(request.body["model"], "deepseek-chat");
        assert_eq!(request.body["response_format"]["type"], "json_object");
        assert_eq!(
            request.headers[1].expose_for_provider_call(),
            "Bearer sk-runtime-only"
        );
        let debug = format!("{request:?}");
        let body = request.body.to_string();
        assert!(!debug.contains("sk-runtime-only"));
        assert!(!body.contains("/Users/alice"));
        assert!(!body.contains("tax.pdf"));
        assert!(!body.contains("hidden"));
    }

    #[test]
    fn ollama_request_shape_is_local_and_sanitized() {
        let sanitized = SanitizedAiRequest {
            prompt: "organize".to_string(),
            files: vec![SanitizedFileInput {
                token: "file_abc".to_string(),
                extension: Some("jpg".to_string()),
                mime: Some("image/jpeg".to_string()),
                size_bucket: SizeBucket::Small,
                path_depth: 3,
            }],
        };
        let request = build_provider_request(
            &ProviderConfig {
                provider: "ollama".to_string(),
                base_url: Some("http://localhost:11434".to_string()),
                model: Some("llama3.2".to_string()),
                cloud_enabled: false,
            },
            &sanitized,
            None,
        )
        .unwrap()
        .unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.url, "http://localhost:11434/api/generate");
        assert_eq!(request.body["model"], "llama3.2");
        assert_eq!(request.body["stream"], false);
        assert_eq!(request.body["format"], "json");
        assert!(request.body["prompt"]
            .as_str()
            .unwrap()
            .contains("file_abc"));
    }

    #[test]
    fn schema_validation_accepts_structured_suggestions() {
        let request = SanitizedAiRequest {
            prompt: "organize".to_string(),
            files: vec![SanitizedFileInput {
                token: "file_ok".to_string(),
                extension: Some("pdf".to_string()),
                mime: Some("application/pdf".to_string()),
                size_bucket: SizeBucket::Small,
                path_depth: 2,
            }],
        };
        let suggestion = validate_structured_suggestion(
            "mock",
            r#"{"summary":"ok","categories":[{"file_token":"file_ok","category":"Documents","confidence":91}]}"#,
            &request,
        )
        .unwrap();

        assert_eq!(suggestion.summary, "ok");
        assert_eq!(suggestion.categories[0].confidence, 91);
    }

    #[test]
    fn schema_validation_returns_clear_errors_for_bad_json_and_missing_fields() {
        let request = SanitizedAiRequest {
            prompt: "organize".to_string(),
            files: vec![SanitizedFileInput {
                token: "file_ok".to_string(),
                extension: None,
                mime: None,
                size_bucket: SizeBucket::Empty,
                path_depth: 0,
            }],
        };

        let invalid = validate_structured_suggestion("mock", "{nope", &request)
            .unwrap_err()
            .to_string();
        let missing = validate_structured_suggestion(
            "mock",
            r#"{"summary":"ok","categories":[{"file_token":"file_ok","category":"Docs"}]}"#,
            &request,
        )
        .unwrap_err()
        .to_string();

        assert!(invalid.contains("invalid JSON"));
        assert!(missing.contains("missing confidence"));
    }

    #[test]
    fn provider_wrappers_parse_json_output() {
        let request = SanitizedAiRequest {
            prompt: "organize".to_string(),
            files: vec![SanitizedFileInput {
                token: "file_ok".to_string(),
                extension: Some("pdf".to_string()),
                mime: Some("application/pdf".to_string()),
                size_bucket: SizeBucket::Small,
                path_depth: 2,
            }],
        };
        let content = r#"{"summary":"ok","categories":[{"file_token":"file_ok","category":"Documents","confidence":80}]}"#;
        let openai_raw = json!({
            "choices": [{"message": {"content": content}}]
        })
        .to_string();
        let ollama_raw = json!({"response": content}).to_string();

        assert_eq!(
            parse_provider_response("openai-compatible", &openai_raw, &request)
                .unwrap()
                .categories[0]
                .category,
            "Documents"
        );
        assert_eq!(
            parse_provider_response("ollama", &ollama_raw, &request)
                .unwrap()
                .categories[0]
                .file_token,
            "file_ok"
        );
    }
}
