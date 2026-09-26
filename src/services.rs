use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceOperationItem {
    pub id: Uuid,
    pub legal_entity_id: Uuid,
    pub service_code: String,
    pub operation_code: String,
    pub risk_level: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceOperationRequest {
    pub service_code: String,
    pub operation_code: String,
    pub request_payload: Value,
    pub validation_payload: Value,
    pub postcondition_payload: Value,
    pub risk_level: String,
}

#[cfg(feature="server")]
pub async fn record_service_operation(pool:&sqlx::PgPool, entity:Uuid, request:&ServiceOperationRequest, status:&str)->Result<Uuid,sqlx::Error>{
    let workspace:Uuid=sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true").bind(entity).fetch_one(pool).await?;
    sqlx::query_scalar("INSERT INTO service_operations(workspace_id,legal_entity_id,service_code,operation_code,request_payload,validation_payload,postcondition_payload,risk_level,status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id").bind(workspace).bind(entity).bind(request.service_code.trim()).bind(request.operation_code.trim()).bind(&request.request_payload).bind(&request.validation_payload).bind(&request.postcondition_payload).bind(request.risk_level.trim().to_uppercase()).bind(status).fetch_one(pool).await
}

#[server]
pub async fn list_service_operations(limit:i32)->Result<Vec<ServiceOperationItem>,ServerFnError>{
    #[cfg(feature="server")]
    {let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;let rows=sqlx::query("SELECT id,legal_entity_id,service_code,operation_code,risk_level,status FROM service_operations WHERE legal_entity_id=$1 ORDER BY occurred_at DESC,id DESC LIMIT $2").bind(crate::entity_scope::current_legal_entity_id()).bind(limit.clamp(1,200)).fetch_all(pool).await.map_err(ServerFnError::new)?;Ok(rows.into_iter().map(|r|ServiceOperationItem{id:r.get("id"),legal_entity_id:r.get("legal_entity_id"),service_code:r.get("service_code"),operation_code:r.get("operation_code"),risk_level:r.get("risk_level"),status:r.get("status")}).collect())}
    #[cfg(not(feature="server"))]
    {let _=limit;Err(ServerFnError::new("list_service_operations est exécutée côté serveur"))}
}
#[cfg(feature="server")]
use sqlx::Row;

pub fn validate_service_request(req:&ServiceOperationRequest)->Result<(),String>{
    if req.service_code.trim().is_empty(){return Err("Service requis".into())}
    if req.operation_code.trim().is_empty(){return Err("Opération requise".into())}
    if !matches!(req.risk_level.trim().to_uppercase().as_str(),"LOW"|"MEDIUM"|"HIGH"|"CRITICAL"){return Err("Niveau de risque service invalide".into())}
    Ok(())
}

#[cfg(test)]mod tests{use super::*;#[test]fn service_contract_validates_risk(){let r=ServiceOperationRequest{service_code:"X".into(),operation_code:"Y".into(),request_payload:Value::Null,validation_payload:Value::Null,postcondition_payload:Value::Null,risk_level:"HIGH".into()};assert!(validate_service_request(&r).is_ok());}}

#[cfg(feature="server")]
pub async fn record_financial_transaction(
    pool: &sqlx::PgPool,
    entity: Uuid,
    occurred_at: chrono::DateTime<chrono::Utc>,
    amount_cents: i64,
    direction: &str,
    source_type: &str,
    source_id: Option<Uuid>,
    status: &str,
    external_key: Option<&str>,
    counterparty: &str,
    label: &str,
    metadata: Value,
) -> Result<Uuid, sqlx::Error> {
    let workspace: Uuid = sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true").bind(entity).fetch_one(pool).await?;
    sqlx::query_scalar("INSERT INTO financial_transactions(workspace_id,legal_entity_id,occurred_at,amount_cents,direction,source_type,source_id,status,external_key,counterparty,label,metadata) VALUES($1,$2,$3,$4,$5,$6,$7,$8,NULLIF($9,''),$10,$11,$12) ON CONFLICT(legal_entity_id,external_key) WHERE external_key IS NOT NULL AND btrim(external_key)<>'' DO UPDATE SET occurred_at=EXCLUDED.occurred_at,amount_cents=EXCLUDED.amount_cents,direction=EXCLUDED.direction,source_type=EXCLUDED.source_type,source_id=EXCLUDED.source_id,status=EXCLUDED.status,counterparty=EXCLUDED.counterparty,label=EXCLUDED.label,metadata=EXCLUDED.metadata RETURNING id")
        .bind(workspace).bind(entity).bind(occurred_at).bind(amount_cents).bind(direction).bind(source_type).bind(source_id).bind(status).bind(external_key.unwrap_or("")).bind(counterparty).bind(label).bind(metadata).fetch_one(pool).await
}

#[cfg(feature="server")]
pub async fn record_business_event(
    pool: &sqlx::PgPool,
    entity: Uuid,
    event_type: &str,
    occurred_at: chrono::DateTime<chrono::Utc>,
    source_type: &str,
    source_id: Option<Uuid>,
    idempotency_key: &str,
    payload: Value,
) -> Result<Uuid, sqlx::Error> {
    let workspace: Uuid = sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true").bind(entity).fetch_one(pool).await?;
    sqlx::query_scalar("INSERT INTO business_events(workspace_id,legal_entity_id,event_type,occurred_at,source_type,source_id,idempotency_key,payload) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(legal_entity_id,idempotency_key) DO UPDATE SET payload=EXCLUDED.payload RETURNING id")
        .bind(workspace).bind(entity).bind(event_type).bind(occurred_at).bind(source_type).bind(source_id).bind(idempotency_key).bind(payload).fetch_one(pool).await
}
