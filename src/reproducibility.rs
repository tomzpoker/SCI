use chrono::NaiveDate;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalculationExplanation {
    pub run_id: Uuid,
    pub legal_entity_id: Uuid,
    pub as_of_date: NaiveDate,
    pub inputs: Value,
    pub rule_code: String,
    pub rule_version_no: i32,
    pub rule_definition: Value,
    pub source_name: String,
    pub source_reference: String,
    pub source_url: String,
    pub formula: String,
    pub result: Value,
    pub rounding_mode: String,
    pub validation_status: String,
}

pub fn describe_formula(definition: &Value) -> String {
    match definition.get("operation").and_then(Value::as_str).unwrap_or("PASS_THROUGH") {
        "PASS_THROUGH" => format!("Valeur({})", definition.get("path").and_then(Value::as_str).unwrap_or("value")),
        "ADD" => format!("Somme({})", definition.get("paths").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" + ")).unwrap_or_default()),
        "SUBTRACT" => format!("Différence({})", definition.get("paths").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" - ")).unwrap_or_default()),
        "MULTIPLY_BPS" => format!("{} × {} / 10 000", definition.get("path").and_then(Value::as_str).unwrap_or("value"), definition.get("rate_bp").and_then(Value::as_i64).unwrap_or(0)),
        other => format!("Opération {}", other),
    }
}

#[cfg(feature = "server")]
use crate::infrastructure::db;
#[cfg(feature = "server")]
use sqlx::Row;

#[server]
pub async fn explain_calculation_run(run_id: Uuid) -> Result<CalculationExplanation, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let entity = crate::entity_scope::current_legal_entity_id();
        let row = sqlx::query(
            r#"
            SELECT r.id,r.legal_entity_id,r.as_of_date,r.input_values,r.formula_text,
                   r.output_value,r.rounding_mode,r.status,r.rule_version_id,
                   rv.version_no,rv.definition,rd.code,
                   COALESCE(vr.source_name,'') AS source_name,
                   COALESCE(vr.source_reference,'') AS source_reference,
                   COALESCE(vr.source_url,'') AS source_url
            FROM rule_calculation_runs r
            JOIN rule_versions rv ON rv.id=r.rule_version_id
            JOIN rule_definitions rd ON rd.id=rv.rule_definition_id
            LEFT JOIN versioned_references vr ON vr.id=rv.source_reference_id
            WHERE r.id=$1 AND r.legal_entity_id=$2
            "#,
        )
        .bind(run_id).bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(CalculationExplanation {
            run_id: row.get("id"),
            legal_entity_id: row.get("legal_entity_id"),
            as_of_date: row.get("as_of_date"),
            inputs: row.get("input_values"),
            rule_code: row.get("code"),
            rule_version_no: row.get("version_no"),
            rule_definition: row.get("definition"),
            source_name: row.get("source_name"),
            source_reference: row.get("source_reference"),
            source_url: row.get("source_url"),
            formula: row.get("formula_text"),
            result: row.get("output_value"),
            rounding_mode: row.get("rounding_mode"),
            validation_status: row.get("status"),
        })
    }
    #[cfg(not(feature = "server"))]
    { let _ = run_id; Err(ServerFnError::new("explain_calculation_run est exécutée côté serveur")) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formula_description_is_stable() {
        assert_eq!(describe_formula(&serde_json::json!({"operation":"ADD","paths":["a","b"]})), "Somme(a + b)");
    }
}
