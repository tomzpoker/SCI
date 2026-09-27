use crate::domain::*;
use crate::entity_scope::current_legal_entity_id;
use chrono::Utc;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrevisionFlowItem {
    pub id: Uuid,
    pub label: String,
    pub amount_cents: i64,
    pub color: String,
    pub active: bool,
    pub recurrence: String, // ONCE | MONTHLY | QUARTERLY | YEARLY
    pub start_year: i32,
    pub start_month: i32,
    pub recurrence_day: i32,
    pub payment_day: i32,
    pub matched_tx_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrevisionFlowDraft {
    pub label: String,
    pub amount_cents: i64,
    pub color: String,
    pub active: bool,
    pub recurrence: String,
    pub start_year: i32,
    pub start_month: i32,
    pub recurrence_day: i32,
    pub payment_day: i32,
}

#[server]
pub async fn list_prevision_flows() -> Result<Vec<PrevisionFlowItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::infrastructure::db;
        use sqlx::Row;
        let pool = db().await.map_err(ServerFnError::new)?;
        let id = current_legal_entity_id();
        let rows = sqlx::query(
            r#"
            SELECT id, label, amount_cents, color, active, recurrence,
                   start_year, start_month, recurrence_day, payment_day,
                   matched_tx_ids
            FROM prevision_flows
            WHERE legal_entity_id = $1
            ORDER BY start_year DESC, start_month DESC, label ASC
            "#,
        )
        .bind(id)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let ids_json: serde_json::Value = r.get("matched_tx_ids");
                let matched: Vec<Uuid> = ids_json
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().and_then(|s| Uuid::parse_str(s).ok()))
                            .collect()
                    })
                    .unwrap_or_default();
                PrevisionFlowItem {
                    id: r.get("id"),
                    label: r.get("label"),
                    amount_cents: r.get("amount_cents"),
                    color: r.get("color"),
                    active: r.get("active"),
                    recurrence: r.get("recurrence"),
                    start_year: r.get("start_year"),
                    start_month: r.get::<i16, _>("start_month") as i32,
                    recurrence_day: r.get::<i16, _>("recurrence_day") as i32,
                    payment_day: r.get::<i16, _>("payment_day") as i32,
                    matched_tx_ids: matched,
                }
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_prevision_flows est exécutée côté serveur",
    ))
}

#[server]
pub async fn create_prevision_flow(draft: PrevisionFlowDraft) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::infrastructure::db;
        let pool = db().await.map_err(ServerFnError::new)?;
        let id = current_legal_entity_id();

        if draft.label.trim().is_empty() {
            return Err(ServerFnError::new("Libellé requis"));
        }
        if !["ONCE", "MONTHLY", "QUARTERLY", "YEARLY"].contains(&draft.recurrence.as_str()) {
            return Err(ServerFnError::new("Récurrence invalide"));
        }

        let flow_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO prevision_flows (
                legal_entity_id, label, amount_cents, color, active, recurrence,
                start_year, start_month, recurrence_day, payment_day
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(id)
        .bind(draft.label.trim())
        .bind(draft.amount_cents)
        .bind(&draft.color)
        .bind(draft.active)
        .bind(&draft.recurrence)
        .bind(draft.start_year)
        .bind(draft.start_month as i16)
        .bind(draft.recurrence_day as i16)
        .bind(draft.payment_day as i16)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;

        sqlx::query(
            "INSERT INTO audit_events(legal_entity_id,actor,action,entity_type,entity_id,payload) \
             VALUES($1,'MANAGER','CREATE_PREVISION_FLOW','PREVISION_FLOW',$2,$3)",
        )
        .bind(id)
        .bind(flow_id)
        .bind(serde_json::json!({"label": draft.label, "amount_cents": draft.amount_cents}))
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(flow_id)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "create_prevision_flow est exécutée côté serveur",
    ))
}

#[server]
pub async fn update_prevision_flow(
    flow_id: Uuid,
    draft: PrevisionFlowDraft,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::infrastructure::db;
        let pool = db().await.map_err(ServerFnError::new)?;
        let id = current_legal_entity_id();

        if draft.label.trim().is_empty() {
            return Err(ServerFnError::new("Libellé requis"));
        }
        if !["ONCE", "MONTHLY", "QUARTERLY", "YEARLY"].contains(&draft.recurrence.as_str()) {
            return Err(ServerFnError::new("Récurrence invalide"));
        }

        let r = sqlx::query(
            r#"
            UPDATE prevision_flows
               SET label = $3,
                   amount_cents = $4,
                   color = $5,
                   active = $6,
                   recurrence = $7,
                   start_year = $8,
                   start_month = $9,
                   recurrence_day = $10,
                   payment_day = $11,
                   updated_at = now()
             WHERE id = $1 AND legal_entity_id = $2
            "#,
        )
        .bind(flow_id)
        .bind(id)
        .bind(draft.label.trim())
        .bind(draft.amount_cents)
        .bind(&draft.color)
        .bind(draft.active)
        .bind(&draft.recurrence)
        .bind(draft.start_year)
        .bind(draft.start_month as i16)
        .bind(draft.recurrence_day as i16)
        .bind(draft.payment_day as i16)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        if r.rows_affected() == 0 {
            return Err(ServerFnError::new("Flux introuvable"));
        }

        sqlx::query(
            "INSERT INTO audit_events(legal_entity_id,actor,action,entity_type,entity_id,payload) \
             VALUES($1,'MANAGER','UPDATE_PREVISION_FLOW','PREVISION_FLOW',$2,$3)",
        )
        .bind(id)
        .bind(flow_id)
        .bind(serde_json::json!({"label": draft.label}))
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "update_prevision_flow est exécutée côté serveur",
    ))
}

#[server]
pub async fn set_prevision_flow_active(
    flow_id: Uuid,
    active: bool,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::infrastructure::db;
        let pool = db().await.map_err(ServerFnError::new)?;
        let id = current_legal_entity_id();
        sqlx::query(
            "UPDATE prevision_flows SET active = $3, updated_at = now() \
             WHERE id = $1 AND legal_entity_id = $2",
        )
        .bind(flow_id)
        .bind(id)
        .bind(active)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "set_prevision_flow_active est exécutée côté serveur",
    ))
}

#[server]
pub async fn delete_prevision_flow(flow_id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::infrastructure::db;
        let pool = db().await.map_err(ServerFnError::new)?;
        let id = current_legal_entity_id();
        let r = sqlx::query(
            "DELETE FROM prevision_flows WHERE id = $1 AND legal_entity_id = $2",
        )
        .bind(flow_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;
        if r.rows_affected() == 0 {
            return Err(ServerFnError::new("Flux introuvable"));
        }
        sqlx::query(
            "INSERT INTO audit_events(legal_entity_id,actor,action,entity_type,entity_id,payload) \
             VALUES($1,'MANAGER','DELETE_PREVISION_FLOW','PREVISION_FLOW',$2,'{}'::jsonb)",
        )
        .bind(id)
        .bind(flow_id)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "delete_prevision_flow est exécutée côté serveur",
    ))
}

#[server]
pub async fn set_prevision_flow_matches(
    flow_id: Uuid,
    tx_ids: Vec<Uuid>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        use crate::infrastructure::db;
        let pool = db().await.map_err(ServerFnError::new)?;
        let id = current_legal_entity_id();
        let json_value: serde_json::Value = serde_json::Value::Array(
            tx_ids.iter().map(|x| serde_json::Value::String(x.to_string())).collect(),
        );
        let r = sqlx::query(
            "UPDATE prevision_flows SET matched_tx_ids = $3, updated_at = now() \
             WHERE id = $1 AND legal_entity_id = $2",
        )
        .bind(flow_id)
        .bind(id)
        .bind(json_value)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;
        if r.rows_affected() == 0 {
            return Err(ServerFnError::new("Flux introuvable"));
        }
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "set_prevision_flow_matches est exécutée côté serveur",
    ))
}