use crate::domain::ChangeHistoryItem;
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use serde_json::Value;
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::infrastructure::db;
#[cfg(feature = "server")]
use sqlx::PgPool;
#[cfg(feature = "server")]
use sqlx::Row;

#[cfg(feature = "server")]
pub async fn snapshot_legal_entity(pool: &PgPool, id: Uuid) -> Result<Value, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT jsonb_build_object(
            'id', e.id,
            'legal_name', e.legal_name,
            'legal_form_code', e.legal_form_code,
            'tax_regime', e.tax_regime,
            'vat_status', e.vat_status,
            'vat_basis', e.vat_basis,
            'siren', COALESCE(e.siren,''),
            'siret', COALESCE(e.siret,''),
            'registered_office', COALESCE(e.registered_office,''),
            'accounting_period_start', e.accounting_period_start,
            'fiscal_year_end', e.fiscal_year_end,
            'currency_code', e.currency_code,
            'active', e.active,
            'primary_iban', COALESCE((SELECT a.iban FROM legal_entity_bank_accounts a WHERE a.legal_entity_id=e.id AND a.active AND a.is_primary LIMIT 1),''),
            'primary_bic', COALESCE((SELECT a.bic FROM legal_entity_bank_accounts a WHERE a.legal_entity_id=e.id AND a.active AND a.is_primary LIMIT 1),'')
        )
        FROM legal_entities e
        WHERE e.id=$1
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

#[cfg(feature = "server")]
pub async fn snapshot_activity_set(pool: &PgPool, legal_entity_id: Uuid) -> Result<Value, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT COALESCE(
            jsonb_agg(
                jsonb_build_object(
                    'activity_code', a.activity_code,
                    'label', c.label,
                    'is_primary', a.is_primary,
                    'active', a.active,
                    'notes', a.notes
                ) ORDER BY a.activity_code
            ), '[]'::jsonb
        )
        FROM legal_entity_activities a
        JOIN legal_activity_catalog c ON c.code=a.activity_code
        WHERE a.legal_entity_id=$1
        "#,
    )
    .bind(legal_entity_id)
    .fetch_one(pool)
    .await
}

#[cfg(feature = "server")]
pub async fn record_entity_change(
    pool: &PgPool,
    legal_entity_id: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
    before_state: Value,
    after_state: Value,
    reason: Option<&str>,
    effective_at: DateTime<Utc>,
    metadata: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO entity_change_history(
            legal_entity_id,effective_at,author,action,entity_type,entity_id,reason,before_state,after_state,metadata
        ) VALUES($1,$2,'MANAGER',$3,$4,$5,NULLIF($6,''),$7,$8,$9)
        "#,
    )
    .bind(legal_entity_id)
    .bind(effective_at)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(reason.unwrap_or(""))
    .bind(before_state)
    .bind(after_state)
    .bind(metadata)
    .execute(pool)
    .await
    .map(|_| ())
}

#[server]
pub async fn list_change_history(limit: i32) -> Result<Vec<ChangeHistoryItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let limit = limit.clamp(1, 500);
        let rows = sqlx::query(
            r#"
            SELECT id,effective_at,recorded_at,author,action,entity_type,entity_id,
                   COALESCE(reason,'') AS reason,before_state::text,after_state::text,metadata::text
            FROM entity_change_history
            WHERE legal_entity_id=$1
            ORDER BY effective_at DESC,id DESC
            LIMIT $2
            "#,
        )
        .bind(crate::entity_scope::current_legal_entity_id())
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| ChangeHistoryItem {
                id: r.get("id"),
                effective_at: r.get("effective_at"),
                recorded_at: r.get("recorded_at"),
                author: r.get("author"),
                action: r.get("action"),
                entity_type: r.get("entity_type"),
                entity_id: r.get("entity_id"),
                reason: r.get("reason"),
                before_state: r.get("before_state"),
                after_state: r.get("after_state"),
                metadata: r.get("metadata"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = limit;
        Err(ServerFnError::new("list_change_history est exécutée côté serveur"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_date_and_payloads_are_explicit_in_the_model() {
        let item = ChangeHistoryItem {
            id: 1,
            effective_at: Utc::now(),
            recorded_at: Utc::now(),
            author: "MANAGER".into(),
            action: "UPDATE".into(),
            entity_type: "LEGAL_ENTITY".into(),
            entity_id: Uuid::nil(),
            reason: "Test".into(),
            before_state: "{}".into(),
            after_state: "{}".into(),
            metadata: "{}".into(),
        };
        assert_eq!(item.author, "MANAGER");
        assert_eq!(item.entity_type, "LEGAL_ENTITY");
    }
}
