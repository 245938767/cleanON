use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
            {
                "<redacted>"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
}
