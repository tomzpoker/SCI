use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxNoticeSummary {
    pub id: Uuid,
    pub property_id: Uuid,
    pub property_name: String,
    pub fiscal_year: i32,
    pub total_amount_cents: Option<i64>,
    pub cotisations_amount_cents: Option<i64>,
    pub management_fees_cents: Option<i64>,
    pub status: String,
    pub addresses_count: i64,
    pub lines_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxNoticeDocument {
    pub id: Uuid,
    pub notice_id: Uuid,
    pub document_kind: String,
    pub file_name: String,
    pub ocr_confidence: Option<i32>,
    pub uploaded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxNoticeAddress {
    pub id: Uuid,
    pub notice_id: Uuid,
    pub address_label: String,
    pub base_amount_cents: i64,
    pub tax_amount_cents: i64,
    pub display_order: i32,
    pub units: Vec<TaxNoticeAddressUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxNoticeAddressUnit {
    pub id: Option<Uuid>,
    pub unit_id: Uuid,
    pub unit_label: String,
    pub share_bp: i32,
    pub amount_cents: i64,
    pub is_vacant: bool,
    pub status: String,
    pub dolibarr_invoice_id: Option<i64>,
    pub dolibarr_invoice_ref: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxNoticeFee {
    pub id: Uuid,
    pub notice_id: Uuid,
    pub fee_label: String,
    pub fee_amount_cents: i64,
    pub distribution_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxNoticeDetail {
    pub id: Uuid,
    pub property_id: Uuid,
    pub property_name: String,
    pub fiscal_year: i32,
    pub notice_reference: String,
    pub total_amount_cents: Option<i64>,
    pub cotisations_amount_cents: Option<i64>,
    pub management_fees_cents: Option<i64>,
    pub status: String,
    pub notes: Option<String>,
    pub documents: Vec<TaxNoticeDocument>,
    pub addresses: Vec<TaxNoticeAddress>,
    pub fees: Vec<TaxNoticeFee>,
    pub dgfip_invoice_id: Option<i64>,
    pub dgfip_invoice_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadTaxNoticeResult {
    pub document_kind: String,
    pub ocr_confidence: i32,
    pub notice_status: String,
    pub parsed_summary: ParsedOcrSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedOcrSummary {
    pub notice_reference: String,
    pub fiscal_year: Option<i32>,
    pub addresses: Vec<ParsedAddressSummary>,
    pub total_amount_cents: Option<i64>,
    pub cotisations_amount_cents: Option<i64>,
    pub management_fees_cents: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedAddressSummary {
    pub label: String,
    pub base_cents: i64,
    pub tax_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAddressesInput {
    pub notice_id: Uuid,
    pub addresses: Vec<UpdateAddressInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAddressInput {
    pub id: Option<Uuid>,
    pub label: String,
    pub base_cents: i64,
    pub tax_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSharesInput {
    pub address_id: Uuid,
    pub shares: Vec<ShareInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInput {
    pub unit_id: Uuid,
    pub share_bp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFeesInput {
    pub notice_id: Uuid,
    pub total_amount_cents: i64,
    pub cotisations_amount_cents: i64,
    pub management_fees_cents: i64,
    pub fees: Vec<FeeInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeInput {
    pub label: String,
    pub amount_cents: i64,
    pub distribution_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInvoicesResult {
    pub notice_id: Uuid,
    pub created: usize,
    pub skipped_vacant: usize,
    pub failed: usize,
    pub lines: Vec<TaxNoticeAddressUnit>,
}