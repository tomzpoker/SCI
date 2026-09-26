use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionDirection { In, Out }
impl TransactionDirection { pub fn code(self) -> &'static str { match self { Self::In => "IN", Self::Out => "OUT" } } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedTransactionInput {
    pub occurred_at: DateTime<Utc>,
    pub amount_cents: i64,
    pub direction: TransactionDirection,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub status: String,
    pub external_key: String,
    pub counterparty: String,
    pub label: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FinancialTransactionItem {
    pub id: Uuid,
    pub legal_entity_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub amount_cents: i64,
    pub direction: String,
    pub source_type: String,
    pub status: String,
    pub external_key: String,
    pub counterparty: String,
    pub label: String,
}

pub fn normalize_transaction(input: NormalizedTransactionInput) -> Result<NormalizedTransactionInput, String> {
    if input.amount_cents < 0 { return Err("Le montant normalisé doit être positif ; le sens porte le débit/crédit".into()); }
    if input.source_type.trim().is_empty() { return Err("Source transactionnelle requise".into()); }
    if input.status.trim().is_empty() { return Err("Statut transactionnel requis".into()); }
    Ok(NormalizedTransactionInput { source_type: input.source_type.trim().to_uppercase(), status: input.status.trim().to_uppercase(), counterparty: input.counterparty.trim().into(), label: input.label.trim().into(), ..input })
}

#[server]
pub async fn create_normalized_transaction(input: NormalizedTransactionInput) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let input = normalize_transaction(input).map_err(ServerFnError::new)?;
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = crate::entity_scope::current_legal_entity_id();
        let workspace: Uuid = sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let id: Uuid = sqlx::query_scalar("INSERT INTO financial_transactions(workspace_id,legal_entity_id,occurred_at,amount_cents,direction,source_type,source_id,status,external_key,counterparty,label,metadata) VALUES($1,$2,$3,$4,$5,$6,$7,$8,NULLIF($9,''),$10,$11,$12) ON CONFLICT(legal_entity_id,external_key) WHERE external_key IS NOT NULL AND btrim(external_key)<>'' DO UPDATE SET occurred_at=EXCLUDED.occurred_at,amount_cents=EXCLUDED.amount_cents,direction=EXCLUDED.direction,source_type=EXCLUDED.source_type,source_id=EXCLUDED.source_id,status=EXCLUDED.status,counterparty=EXCLUDED.counterparty,label=EXCLUDED.label,metadata=EXCLUDED.metadata RETURNING id")
            .bind(workspace).bind(entity).bind(input.occurred_at).bind(input.amount_cents).bind(input.direction.code()).bind(&input.source_type).bind(input.source_id).bind(&input.status).bind(&input.external_key).bind(&input.counterparty).bind(&input.label).bind(input.metadata).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _ = input; Err(ServerFnError::new("create_normalized_transaction est exécutée côté serveur")) }
}

#[server]
pub async fn list_normalized_transactions(limit: i32) -> Result<Vec<FinancialTransactionItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT id,legal_entity_id,occurred_at,amount_cents,direction,source_type,status,COALESCE(external_key,''),counterparty,label FROM financial_transactions WHERE legal_entity_id=$1 ORDER BY occurred_at DESC,id DESC LIMIT $2").bind(crate::entity_scope::current_legal_entity_id()).bind(limit.clamp(1,200)).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| FinancialTransactionItem{id:r.get("id"),legal_entity_id:r.get("legal_entity_id"),occurred_at:r.get("occurred_at"),amount_cents:r.get("amount_cents"),direction:r.get("direction"),source_type:r.get("source_type"),status:r.get("status"),external_key:r.get(7),counterparty:r.get("counterparty"),label:r.get("label")}).collect())
    }
    #[cfg(not(feature = "server"))]
    { let _ = limit; Err(ServerFnError::new("list_normalized_transactions est exécutée côté serveur")) }
}

#[cfg(feature = "server")]
use sqlx::Row;

#[cfg(test)]
mod tests { use super::*; #[test] fn direction_codes_are_stable(){assert_eq!(TransactionDirection::In.code(),"IN");assert_eq!(TransactionDirection::Out.code(),"OUT");} }
