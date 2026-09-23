use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProvider {
    pub id: String,
    pub name: String,
    pub model: String,
    pub is_free: bool,
    pub supports_text: bool,
    pub supports_voice: bool,
}

impl LlmProvider {
    pub fn free_only(id: &str, name: &str, model: &str, supports_voice: bool) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            model: model.to_owned(),
            is_free: true,
            supports_text: true,
            supports_voice,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub provider_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmError {
    PaidProviderRejected,
    ProviderUnavailable,
    RequestFailed(String),
}
