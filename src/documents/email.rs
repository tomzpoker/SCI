use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailScanStatus {
    New,
    Selected,
    Imported,
    Ignored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAccount {
    pub id: uuid::Uuid,
    pub address: String,
    pub provider: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub id: uuid::Uuid,
    pub account_id: uuid::Uuid,
    pub message_id: String,
    pub sender: String,
    pub subject: String,
    pub received_at: String,
    pub attachments: Vec<EmailAttachment>,
    pub status: EmailScanStatus,
}
