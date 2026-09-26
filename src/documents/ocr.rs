use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OcrEngine {
    Local,
    FreeRemote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrPage {
    pub page_number: u32,
    pub text: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub document_id: uuid::Uuid,
    pub engine: OcrEngine,
    pub pages: Vec<OcrPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OcrError {
    UnsupportedFormat,
    EngineUnavailable,
    ProcessingFailed(String),
}
