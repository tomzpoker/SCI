use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LlmProviderKind {
    LocalLLM,
    ExternalLLM,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfiguredLlmProvider {
    pub id: String,
    pub name: String,
    pub model: String,
    pub kind: LlmProviderKind,
    pub endpoint_reference: String,
    pub supports_text: bool,
    pub supports_voice: bool,
    pub enabled: bool,
}

/// Contrat utilisé par l'assistant. Le cœur ne dépend d'aucun SDK LLM.
pub trait LlmProvider {
    fn kind(&self) -> LlmProviderKind;
    fn name(&self) -> &str;
    fn supports_text(&self) -> bool;
    fn supports_voice(&self) -> bool;
    fn generate(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError>;
}

#[derive(Debug, Clone)]
pub struct DisabledLlmProvider;

impl LlmProvider for DisabledLlmProvider {
    fn kind(&self) -> LlmProviderKind { LlmProviderKind::Disabled }
    fn name(&self) -> &str { "IA désactivée" }
    fn supports_text(&self) -> bool { false }
    fn supports_voice(&self) -> bool { false }
    fn generate(&self, _request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        Err(LlmError::ProviderUnavailable)
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

#[derive(Debug, Clone)]
pub struct LocalLlmProvider {
    pub name: String,
    pub model: String,
    pub command_reference: Option<String>,
    pub supports_voice: bool,
}

impl LlmProvider for LocalLlmProvider {
    fn kind(&self) -> LlmProviderKind { LlmProviderKind::LocalLLM }
    fn name(&self) -> &str { &self.name }
    fn supports_text(&self) -> bool { true }
    fn supports_voice(&self) -> bool { self.supports_voice }
    fn generate(&self, _request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        Err(LlmError::RequestFailed(format!("Adapter local non lancé: {}", self.command_reference.as_deref().unwrap_or("aucune commande configurée"))))
    }
}

#[derive(Debug, Clone)]
pub struct ExternalLlmProvider {
    pub name: String,
    pub model: String,
    pub endpoint_reference: String,
    pub supports_voice: bool,
}

impl LlmProvider for ExternalLlmProvider {
    fn kind(&self) -> LlmProviderKind { LlmProviderKind::ExternalLLM }
    fn name(&self) -> &str { &self.name }
    fn supports_text(&self) -> bool { true }
    fn supports_voice(&self) -> bool { self.supports_voice }
    fn generate(&self, _request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        Err(LlmError::RequestFailed(format!("Adapter externe non connecté: {}", self.endpoint_reference)))
    }
}

impl ConfiguredLlmProvider {
    pub fn disabled() -> Self {
        Self {
            id: "disabled".into(),
            name: "IA désactivée".into(),
            model: String::new(),
            kind: LlmProviderKind::Disabled,
            endpoint_reference: String::new(),
            supports_text: false,
            supports_voice: false,
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_provider_is_safe() {
        let p = DisabledLlmProvider;
        assert_eq!(p.kind(), LlmProviderKind::Disabled);
        assert!(p.generate(&LlmRequest { prompt: "x".into(), system_prompt: None }).is_err());
    }
}
