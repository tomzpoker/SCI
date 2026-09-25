use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobClaim {
    pub id: Uuid,
    pub execute: bool,
    pub status: String,
    pub result_payload: Value,
}

pub fn request_hash(payload: &Value) -> String {
    let bytes = serde_json::to_vec(payload).unwrap_or_default();
    let mut state: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        state ^= u64::from(byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{state:016x}")
}

#[cfg(feature = "server")]
pub async fn claim_job(
    pool: &sqlx::PgPool,
    entity: Uuid,
    job_code: &str,
    idempotency_key: &str,
    payload: &Value,
) -> Result<JobClaim, sqlx::Error> {
    let job_code = job_code.trim();
    let key = idempotency_key.trim();
    let hash = request_hash(payload);
    let workspace: Uuid = sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true")
        .bind(entity).fetch_one(pool).await?;
    let mut tx = pool.begin().await?;
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO job_executions(workspace_id,legal_entity_id,job_code,idempotency_key,request_hash,status,result_payload) VALUES($1,$2,$3,$4,$5,'STARTED','{}'::jsonb) ON CONFLICT(legal_entity_id,job_code,idempotency_key) DO NOTHING RETURNING id"
    ).bind(workspace).bind(entity).bind(job_code).bind(key).bind(&hash).fetch_optional(&mut *tx).await?;
    if let Some(id) = inserted {
        tx.commit().await?;
        return Ok(JobClaim { id, execute: true, status: "STARTED".into(), result_payload: Value::Object(Default::default()) });
    }
    let row = sqlx::query("SELECT id,request_hash,status,result_payload FROM job_executions WHERE legal_entity_id=$1 AND job_code=$2 AND idempotency_key=$3 FOR UPDATE")
        .bind(entity).bind(job_code).bind(key).fetch_one(&mut *tx).await?;
    let existing_hash: String = row.get("request_hash");
    if existing_hash != hash {
        return Err(sqlx::Error::Protocol("Idempotency key déjà utilisée avec des entrées différentes".into()));
    }
    let id: Uuid = row.get("id");
    let status: String = row.get("status");
    let result_payload: Value = row.get("result_payload");
    if status == "FAILED" {
        sqlx::query("UPDATE job_executions SET status='STARTED',attempt_count=attempt_count+1,error_message='',started_at=now(),finished_at=NULL,updated_at=now() WHERE id=$1")
            .bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
        return Ok(JobClaim { id, execute: true, status: "STARTED".into(), result_payload });
    }
    if status == "STARTED" {
        let reclaimed: bool = sqlx::query_scalar("SELECT updated_at < now()-interval '15 minutes' FROM job_executions WHERE id=$1")
            .bind(id).fetch_one(&mut *tx).await?;
        if reclaimed {
            sqlx::query("UPDATE job_executions SET attempt_count=attempt_count+1,started_at=now(),updated_at=now(),error_message='' WHERE id=$1")
                .bind(id).execute(&mut *tx).await?;
            tx.commit().await?;
            return Ok(JobClaim { id, execute: true, status: "STARTED".into(), result_payload });
        }
    }
    tx.commit().await?;
    Ok(JobClaim { id, execute: false, status, result_payload })
}

#[cfg(feature = "server")]
pub async fn complete_job(pool: &sqlx::PgPool, id: Uuid, result: &Value) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE job_executions SET status='SUCCEEDED',result_payload=$2,finished_at=now(),updated_at=now() WHERE id=$1")
        .bind(id).bind(result).execute(pool).await.map(|_| ())
}

#[cfg(feature = "server")]
pub async fn fail_job(pool: &sqlx::PgPool, id: Uuid, message: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE job_executions SET status='FAILED',error_message=$2,finished_at=now(),updated_at=now() WHERE id=$1")
        .bind(id).bind(message).execute(pool).await.map(|_| ())
}

#[cfg(feature = "server")]
use sqlx::Row;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_payload_has_same_hash() {
        let a = serde_json::json!({"x":1,"y":"A"});
        assert_eq!(request_hash(&a), request_hash(&a));
    }
    #[test]
    fn different_payload_has_different_hash() {
        assert_ne!(request_hash(&serde_json::json!({"x":1})), request_hash(&serde_json::json!({"x":2})));
    }
}
