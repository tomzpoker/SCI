use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BusinessEventInput {
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub idempotency_key: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BusinessEventItem {
    pub id: Uuid,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub idempotency_key: String,
    pub status: String,
}

pub const STANDARD_EVENTS: &[&str] = &[
    "LeaseCreated","InvoiceIssued","PaymentDetected","StockReceived","StockSold","TaxDeadlineReached",
    "PropertyCreated","TenantCreated","DocumentRegistered","BankImported","TaskCreated",
];

pub fn canonical_event_type(value: &str) -> Option<String> {
    let candidate=value.trim();
    STANDARD_EVENTS.iter().find(|x| x.eq_ignore_ascii_case(candidate)).map(|x| (*x).to_string())
}

#[server]
pub async fn emit_business_event(input: BusinessEventInput) -> Result<Uuid, ServerFnError> {
    #[cfg(feature="server")]
    {
        let event_type=canonical_event_type(&input.event_type).ok_or_else(||ServerFnError::new("Type d'événement métier inconnu"))?;
        if input.source_type.trim().is_empty() || input.idempotency_key.trim().is_empty(){return Err(ServerFnError::new("Source et idempotence requises"));}
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity=crate::entity_scope::current_legal_entity_id();
        let workspace:Uuid=sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let id:Uuid=sqlx::query_scalar("INSERT INTO business_events(workspace_id,legal_entity_id,event_type,occurred_at,source_type,source_id,idempotency_key,payload) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(legal_entity_id,idempotency_key) DO UPDATE SET payload=EXCLUDED.payload RETURNING id").bind(workspace).bind(entity).bind(event_type).bind(input.occurred_at).bind(input.source_type.trim()).bind(input.source_id).bind(input.idempotency_key.trim()).bind(input.payload).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature="server"))]
    { let _=input; Err(ServerFnError::new("emit_business_event est exécutée côté serveur")) }
}

#[server]
pub async fn list_business_events(limit:i32)->Result<Vec<BusinessEventItem>,ServerFnError>{
    #[cfg(feature="server")]
    {let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let rows=sqlx::query("SELECT id,event_type,occurred_at,source_type,source_id,idempotency_key,status FROM business_events WHERE legal_entity_id=$1 ORDER BY occurred_at DESC,id DESC LIMIT $2").bind(crate::entity_scope::current_legal_entity_id()).bind(limit.clamp(1,200)).fetch_all(pool).await.map_err(ServerFnError::new)?;Ok(rows.into_iter().map(|r|BusinessEventItem{id:r.get("id"),event_type:r.get("event_type"),occurred_at:r.get("occurred_at"),source_type:r.get("source_type"),source_id:r.get("source_id"),idempotency_key:r.get("idempotency_key"),status:r.get("status")}).collect())}
    #[cfg(not(feature="server"))]
    {let _=limit;Err(ServerFnError::new("list_business_events est exécutée côté serveur"))}
}
#[cfg(feature="server")]
use sqlx::Row;
#[cfg(test)] mod tests{use super::*;#[test]fn event_catalog_is_standardized(){assert_eq!(canonical_event_type("InvoiceIssued").as_deref(),Some("InvoiceIssued"));assert_eq!(canonical_event_type("unknown"),None);}}
