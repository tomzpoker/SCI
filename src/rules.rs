use crate::entity_scope::current_legal_entity_id;
use crate::reproducibility::describe_formula;
#[cfg(feature = "server")]
use crate::infrastructure::db;
use chrono::{NaiveDate, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "server")]
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferenceItem {
    pub id: Uuid,
    pub namespace: String,
    pub code: String,
    pub version_no: i32,
    pub valid_from: NaiveDate,
    pub valid_to: Option<NaiveDate>,
    pub source_kind: String,
    pub source_name: String,
    pub source_url: String,
    pub source_reference: String,
    pub verified_at: Option<NaiveDate>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferenceDraft {
    pub namespace: String,
    pub code: String,
    pub version_no: i32,
    pub valid_from: NaiveDate,
    pub valid_to: Option<NaiveDate>,
    pub source_kind: String,
    pub source_name: String,
    pub source_url: String,
    pub source_reference: String,
    pub verified_at: Option<NaiveDate>,
    pub status: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleItem {
    pub id: Uuid,
    pub code: String,
    pub label: String,
    pub engine_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleVersionItem {
    pub id: Uuid,
    pub rule_code: String,
    pub rule_label: String,
    pub version_no: i32,
    pub legal_entity_id: Option<Uuid>,
    pub activity_code: String,
    pub jurisdiction_code: String,
    pub valid_from: NaiveDate,
    pub valid_to: Option<NaiveDate>,
    pub status: String,
    pub source_reference_id: Option<Uuid>,
    pub source_name: String,
    pub definition: Value,
    pub change_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleVersionDraft {
    pub rule_code: String,
    pub rule_label: String,
    pub version_no: i32,
    pub legal_entity_id: Option<Uuid>,
    pub global_scope: bool,
    pub activity_code: Option<String>,
    pub jurisdiction_code: Option<String>,
    pub valid_from: NaiveDate,
    pub valid_to: Option<NaiveDate>,
    pub status: String,
    pub source_reference_id: Option<Uuid>,
    pub definition: Value,
    pub change_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedRuleVersion {
    pub id: Uuid,
    pub rule_code: String,
    pub version_no: i32,
    pub legal_entity_id: Option<Uuid>,
    pub activity_code: Option<String>,
    pub jurisdiction_code: Option<String>,
    pub valid_from: NaiveDate,
    pub valid_to: Option<NaiveDate>,
    pub source_reference_id: Option<Uuid>,
    pub definition: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalculationReplayItem {
    pub original_run_id: Uuid,
    pub replay_run_id: Uuid,
    pub rule_version_id: Uuid,
    pub rule_code: String,
    pub rule_version_no: i32,
    pub as_of_date: NaiveDate,
    pub input_values: Value,
    pub original_output: Value,
    pub replay_output: Value,
    pub matches: bool,
}

fn validate_code(value: &str, label: &str) -> Result<String, ServerFnError> {
    let normalized = value.trim().to_uppercase();
    if normalized.is_empty()
        || !normalized
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(ServerFnError::new(format!("{} invalide", label)));
    }
    Ok(normalized)
}

fn audit_hash(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut state: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        state ^= u64::from(byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{state:016x}")
}

fn json_number(value: &Value, path: &str) -> Result<i64, String> {
    let mut current = value;
    for part in path.split('.') {
        current = current
            .get(part)
            .ok_or_else(|| format!("Entrée absente: {}", path))?;
    }
    current
        .as_i64()
        .or_else(|| current.as_f64().map(|v| v.round() as i64))
        .ok_or_else(|| format!("Entrée non numérique: {}", path))
}

pub fn evaluate_definition(definition: &Value, inputs: &Value) -> Result<Value, String> {
    let operation = definition
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("PASS_THROUGH");
    match operation {
        "PASS_THROUGH" => {
            let path = definition
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| "PASS_THROUGH exige path".to_string())?;
            Ok(Value::from(json_number(inputs, path)?))
        }
        "ADD" => {
            let paths = definition
                .get("paths")
                .and_then(Value::as_array)
                .ok_or_else(|| "ADD exige paths".to_string())?;
            let mut total = 0i64;
            for path in paths {
                total = total.saturating_add(json_number(
                    inputs,
                    path.as_str().ok_or_else(|| "path invalide".to_string())?,
                )?);
            }
            Ok(Value::from(total))
        }
        "SUBTRACT" => {
            let paths = definition
                .get("paths")
                .and_then(Value::as_array)
                .ok_or_else(|| "SUBTRACT exige paths".to_string())?;
            let first = paths.first().and_then(Value::as_str).ok_or_else(|| "SUBTRACT exige au moins un path".to_string())?;
            let mut total = json_number(inputs, first)?;
            for path in paths.iter().skip(1) {
                total = total.saturating_sub(json_number(
                    inputs,
                    path.as_str().ok_or_else(|| "path invalide".to_string())?,
                )?);
            }
            Ok(Value::from(total))
        }
        "MULTIPLY_BPS" => {
            let path = definition
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| "MULTIPLY_BPS exige path".to_string())?;
            let rate_bp = definition
                .get("rate_bp")
                .and_then(Value::as_i64)
                .ok_or_else(|| "MULTIPLY_BPS exige rate_bp".to_string())?;
            let base = json_number(inputs, path)? as i128;
            Ok(Value::from(((base * rate_bp as i128) / 10_000i128) as i64))
        }
        other => Err(format!("Opération Rule Engine non supportée: {}", other)),
    }
}

#[cfg(feature = "server")]
async fn workspace_for_entity(pool: &PgPool, id: Uuid) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true")
        .bind(id)
        .fetch_one(pool)
        .await
}

#[server]
pub async fn list_versioned_references() -> Result<Vec<ReferenceItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let workspace = workspace_for_entity(pool, current_legal_entity_id())
            .await
            .map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT id,namespace,code,version_no,valid_from,valid_to,source_kind,source_name,COALESCE(source_url,''),COALESCE(source_reference,''),verified_at,status FROM versioned_references WHERE workspace_id=$1 ORDER BY namespace,code,version_no DESC",
        )
        .bind(workspace)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| ReferenceItem {
                id: r.get("id"), namespace: r.get("namespace"), code: r.get("code"), version_no: r.get("version_no"),
                valid_from: r.get("valid_from"), valid_to: r.get("valid_to"), source_kind: r.get("source_kind"),
                source_name: r.get("source_name"), source_url: r.get(8), source_reference: r.get(9), verified_at: r.get("verified_at"), status: r.get("status"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_versioned_references est exécutée côté serveur"))
}

#[server]
pub async fn create_versioned_reference(draft: ReferenceDraft) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if draft.version_no < 1 { return Err(ServerFnError::new("La version doit être >= 1")); }
        if draft.valid_to.is_some_and(|v| v < draft.valid_from) { return Err(ServerFnError::new("Période de validité invalide")); }
        let namespace = validate_code(&draft.namespace, "Namespace")?;
        let code = validate_code(&draft.code, "Code référence")?;
        let status = draft.status.trim().to_uppercase();
        let source_kind = draft.source_kind.trim().to_uppercase();
        if status == "PUBLISHED" && draft.verified_at.is_none() { return Err(ServerFnError::new("Une référence publiée doit être vérifiée")); }
        if !matches!(status.as_str(), "DRAFT" | "PUBLISHED" | "RETIRED") { return Err(ServerFnError::new("Statut référence invalide")); }
        if !matches!(source_kind.as_str(), "OFFICIAL" | "OTHER") { return Err(ServerFnError::new("Type de source invalide")); }
        let pool = db().await.map_err(ServerFnError::new)?;
        let workspace = workspace_for_entity(pool, current_legal_entity_id()).await.map_err(ServerFnError::new)?;
        let id: Uuid = sqlx::query_scalar("INSERT INTO versioned_references(workspace_id,namespace,code,version_no,valid_from,valid_to,source_kind,source_name,source_url,source_reference,verified_at,status,payload) VALUES($1,$2,$3,$4,$5,$6,$7,$8,NULLIF($9,''),NULLIF($10,''),$11,$12,$13) RETURNING id")
            .bind(workspace).bind(namespace).bind(code).bind(draft.version_no).bind(draft.valid_from).bind(draft.valid_to).bind(source_kind).bind(draft.source_name.trim()).bind(draft.source_url.trim()).bind(draft.source_reference.trim()).bind(draft.verified_at).bind(status).bind(draft.payload).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _ = draft; Err(ServerFnError::new("create_versioned_reference est exécutée côté serveur")) }
}

#[server]
pub async fn list_rule_versions() -> Result<Vec<RuleVersionItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let workspace = workspace_for_entity(pool, current_legal_entity_id()).await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let rows = sqlx::query(
            "SELECT rv.id,rd.code,rd.label,rv.version_no,rv.legal_entity_id,COALESCE(rv.activity_code,''),COALESCE(rv.jurisdiction_code,''),rv.valid_from,rv.valid_to,rv.status,rv.source_reference_id,COALESCE(vr.source_name,''),rv.definition,rv.change_note FROM rule_versions rv JOIN rule_definitions rd ON rd.id=rv.rule_definition_id LEFT JOIN versioned_references vr ON vr.id=rv.source_reference_id WHERE rv.workspace_id=$1 AND (rv.legal_entity_id IS NULL OR rv.legal_entity_id=$2) ORDER BY rd.code,rv.valid_from DESC,rv.version_no DESC",
        )
        .bind(workspace).bind(entity).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| RuleVersionItem {
            id:r.get("id"), rule_code:r.get("code"), rule_label:r.get("label"), version_no:r.get("version_no"), legal_entity_id:r.get("legal_entity_id"),
            activity_code:r.get(5), jurisdiction_code:r.get(6), valid_from:r.get("valid_from"), valid_to:r.get("valid_to"), status:r.get("status"), source_reference_id:r.get("source_reference_id"), source_name:r.get("source_name"), definition:r.get("definition"), change_note:r.get("change_note")
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_rule_versions est exécutée côté serveur"))
}

#[server]
pub async fn create_rule_version(draft: RuleVersionDraft) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if draft.version_no < 1 { return Err(ServerFnError::new("La version de règle doit être >= 1")); }
        if draft.valid_to.is_some_and(|v| v < draft.valid_from) { return Err(ServerFnError::new("Période de règle invalide")); }
        let code = validate_code(&draft.rule_code, "Code règle")?;
        let status = draft.status.trim().to_uppercase();
        if !matches!(status.as_str(), "DRAFT" | "PUBLISHED" | "RETIRED") { return Err(ServerFnError::new("Statut RuleVersion invalide")); }
        let activity = draft.activity_code.as_deref().map(|v| validate_code(v, "Activité")).transpose()?;
        let jurisdiction = draft.jurisdiction_code.as_deref().map(|v| v.trim().to_uppercase()).filter(|v| !v.is_empty());
        let pool = db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let workspace = workspace_for_entity(pool, entity).await.map_err(ServerFnError::new)?;
        let target_entity = if draft.global_scope { None } else {
            if let Some(id) = draft.legal_entity_id {
                if id != entity { return Err(ServerFnError::new("La RuleVersion doit rester dans l'entité active")); }
            }
            Some(entity)
        };
        let source = draft.source_reference_id;
        if status == "PUBLISHED" && source.is_none() { return Err(ServerFnError::new("Une RuleVersion publiée exige une source")); }
        let rule_id: Uuid = sqlx::query_scalar("INSERT INTO rule_definitions(workspace_id,code,label) VALUES($1,$2,$3) ON CONFLICT(workspace_id,code) DO UPDATE SET label=EXCLUDED.label RETURNING id")
            .bind(workspace).bind(code).bind(draft.rule_label.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let id: Uuid = sqlx::query_scalar("INSERT INTO rule_versions(rule_definition_id,workspace_id,legal_entity_id,activity_code,jurisdiction_code,version_no,valid_from,valid_to,status,source_reference_id,definition,change_note) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING id")
            .bind(rule_id).bind(workspace).bind(draft.legal_entity_id).bind(activity).bind(jurisdiction).bind(draft.version_no).bind(draft.valid_from).bind(draft.valid_to).bind(status).bind(source).bind(draft.definition).bind(draft.change_note.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _ = draft; Err(ServerFnError::new("create_rule_version est exécutée côté serveur")) }
}

#[server]
pub async fn publish_rule_version(rule_version_id: Uuid, source_code: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let code = validate_code(&source_code, "Code source")?;
        let reference: Option<Uuid> = sqlx::query_scalar(
            "SELECT vr.id FROM versioned_references vr JOIN legal_entities e ON e.workspace_id=vr.workspace_id WHERE e.id=$1 AND e.active=true AND vr.namespace='OFFICIAL_SOURCE' AND vr.code=$2 AND vr.status='PUBLISHED' ORDER BY vr.version_no DESC LIMIT 1"
        ).bind(entity).bind(code).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        let reference = reference.ok_or_else(|| ServerFnError::new("Source officielle publiée introuvable"))?;
        let result = sqlx::query(
            "UPDATE rule_versions rv SET status='PUBLISHED',source_reference_id=$3,updated_at=now() WHERE rv.id=$1 AND (rv.legal_entity_id IS NULL OR rv.legal_entity_id=$2) AND rv.status='DRAFT'"
        ).bind(rule_version_id).bind(entity).bind(reference).execute(pool).await.map_err(ServerFnError::new)?;
        if result.rows_affected() == 0 {
            return Err(ServerFnError::new("RuleVersion introuvable ou déjà publiée"));
        }
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=(rule_version_id,source_code); Err(ServerFnError::new("publish_rule_version est exécutée côté serveur")) }
}

#[cfg(feature = "server")]
pub async fn resolve_rule_version(
    pool: &PgPool,
    legal_entity_id: Uuid,
    rule_code: &str,
    activity_code: Option<&str>,
    jurisdiction_code: Option<&str>,
    as_of: NaiveDate,
) -> Result<ResolvedRuleVersion, sqlx::Error> {
    let workspace = workspace_for_entity(pool, legal_entity_id).await?;
    let row = sqlx::query(
        "SELECT rv.id,rd.code,rv.version_no,rv.legal_entity_id,rv.activity_code,rv.jurisdiction_code,rv.valid_from,rv.valid_to,rv.source_reference_id,rv.definition FROM rule_versions rv JOIN rule_definitions rd ON rd.id=rv.rule_definition_id WHERE rv.workspace_id=$1 AND rd.code=$2 AND rv.status='PUBLISHED' AND rv.valid_from <= $3 AND (rv.valid_to IS NULL OR rv.valid_to >= $3) AND (rv.legal_entity_id IS NULL OR rv.legal_entity_id=$4) AND (rv.activity_code IS NULL OR rv.activity_code=$5) AND (rv.jurisdiction_code IS NULL OR rv.jurisdiction_code=$6) ORDER BY (CASE WHEN rv.legal_entity_id=$4 THEN 8 ELSE 0 END)+(CASE WHEN rv.activity_code=$5 THEN 4 ELSE 0 END)+(CASE WHEN rv.jurisdiction_code=$6 THEN 2 ELSE 0 END) DESC,rv.version_no DESC LIMIT 1",
    )
    .bind(workspace).bind(rule_code.trim().to_uppercase()).bind(as_of).bind(legal_entity_id).bind(activity_code).bind(jurisdiction_code).fetch_one(pool).await?;
    Ok(ResolvedRuleVersion { id:row.get("id"), rule_code:row.get("code"), version_no:row.get("version_no"), legal_entity_id:row.get("legal_entity_id"), activity_code:row.get("activity_code"), jurisdiction_code:row.get("jurisdiction_code"), valid_from:row.get("valid_from"), valid_to:row.get("valid_to"), source_reference_id:row.get("source_reference_id"), definition:row.get("definition") })
}

#[server]
pub async fn create_calculation_run(rule_version_id: Uuid, as_of_date: NaiveDate, input_values: Value) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let row = sqlx::query("SELECT workspace_id,legal_entity_id,definition,status FROM rule_versions WHERE id=$1").bind(rule_version_id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let workspace: Uuid = row.get("workspace_id");
        let rule_entity: Option<Uuid> = row.get("legal_entity_id");
        if rule_entity.is_some_and(|id| id != entity) { return Err(ServerFnError::new("RuleVersion hors entité active")); }
        if row.get::<String,_>("status") != "PUBLISHED" { return Err(ServerFnError::new("Seule une RuleVersion publiée peut être exécutée")); }
        let definition: Value = row.get("definition");
        let output = evaluate_definition(&definition, &input_values).map_err(ServerFnError::new)?;
        let source = sqlx::query("SELECT rv.version_no,COALESCE(vr.source_name,''),COALESCE(vr.source_reference,''),COALESCE(vr.source_url,'') FROM rule_versions rv LEFT JOIN versioned_references vr ON vr.id=rv.source_reference_id WHERE rv.id=$1")
            .bind(rule_version_id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let formula = describe_formula(&definition);
        let source_name: String = source.get(1);
        let source_reference: String = source.get(2);
        let source_url: String = source.get(3);
        let source_snapshot = serde_json::json!({"source_name":source_name,"source_reference":source_reference,"source_url":source_url});
        let rule_snapshot = serde_json::json!({"rule_version_id":rule_version_id,"version_no":source.get::<i32,_>("version_no"),"definition":definition,"source":source_snapshot.clone()});
        let input_hash = audit_hash(&input_values);
        let output_hash = audit_hash(&output);
        let id: Uuid = sqlx::query_scalar("INSERT INTO rule_calculation_runs(workspace_id,legal_entity_id,rule_version_id,as_of_date,input_values,output_value,formula_text,rule_snapshot,source_snapshot,rounding_mode,input_hash,output_hash,status,actor) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,'INTEGER_CENTS',$10,$11,'COMPLETED','MANAGER') RETURNING id")
            .bind(workspace).bind(entity).bind(rule_version_id).bind(as_of_date).bind(input_values).bind(output).bind(formula).bind(rule_snapshot).bind(source_snapshot).bind(input_hash).bind(output_hash).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _=(rule_version_id,as_of_date,input_values); Err(ServerFnError::new("create_calculation_run est exécutée côté serveur")) }
}

#[server]
pub async fn replay_calculation_run(run_id: Uuid) -> Result<CalculationReplayItem, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let row = sqlx::query("SELECT r.id,r.rule_version_id,r.as_of_date,r.input_values,r.output_value,rv.definition,rv.version_no,rd.code,rv.legal_entity_id,rv.status FROM rule_calculation_runs r JOIN rule_versions rv ON rv.id=r.rule_version_id JOIN rule_definitions rd ON rd.id=rv.rule_definition_id WHERE r.id=$1 AND r.legal_entity_id=$2").bind(run_id).bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let definition: Value = row.get("definition");
        let input_values: Value = row.get("input_values");
        let original_output: Value = row.get("output_value");
        let replay_output = evaluate_definition(&definition, &input_values).map_err(ServerFnError::new)?;
        let formula = describe_formula(&definition);
        let input_hash = audit_hash(&input_values);
        let output_hash = audit_hash(&replay_output);
        let replay_id: Uuid = sqlx::query_scalar("INSERT INTO rule_calculation_runs(workspace_id,legal_entity_id,rule_version_id,as_of_date,input_values,output_value,replayed_from,status,actor,formula_text,rule_snapshot,source_snapshot,rounding_mode,input_hash,output_hash) SELECT workspace_id,legal_entity_id,rule_version_id,as_of_date,input_values,$1,id,'REPLAYED','SYSTEM',$3,rule_snapshot,source_snapshot,'INTEGER_CENTS',$4,$5 FROM rule_calculation_runs WHERE id=$2 RETURNING id")
            .bind(&replay_output).bind(run_id).bind(formula).bind(input_hash).bind(output_hash).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(CalculationReplayItem { original_run_id:run_id, replay_run_id:replay_id, rule_version_id:row.get("rule_version_id"), rule_code:row.get("code"), rule_version_no:row.get("version_no"), as_of_date:row.get("as_of_date"), input_values, original_output:original_output.clone(), replay_output:replay_output.clone(), matches:original_output==replay_output })
    }
    #[cfg(not(feature = "server"))]
    { let _=run_id; Err(ServerFnError::new("replay_calculation_run est exécutée côté serveur")) }
}

fn valid_to_label(value: Option<chrono::NaiveDate>) -> String {
    value.map(|d| d.to_string()).unwrap_or_else(|| "ouverte".to_owned())
}

#[component]
pub fn RulesPage(mut refresh: Signal<u64>) -> Element {
    let references = use_resource(move || {
        let _ = refresh();
        async move { list_versioned_references().await.unwrap_or_default() }
    });
    let rules = use_resource(move || {
        let _ = refresh();
        async move { list_rule_versions().await.unwrap_or_default() }
    });
    let mut msg = use_signal(String::new);
    let mut ref_code = use_signal(String::new);
    let mut ref_source = use_signal(|| "impots.gouv.fr".to_string());
    let mut ref_url = use_signal(|| "https://www.impots.gouv.fr".to_string());
    let mut ref_version = use_signal(|| "1".to_string());
    let mut rule_code = use_signal(String::new);
    let mut rule_label = use_signal(String::new);
    let mut rule_version = use_signal(|| "1".to_string());
    let mut rule_definition = use_signal(|| r#"{"operation":"ADD","paths":["base_cents","ajustement_cents"]}"#.to_string());
    let mut rule_source = use_signal(|| "IMPOTS_GOUV_FR".to_string());
    let mut rule_activity = use_signal(String::new);
    let mut rule_jurisdiction = use_signal(String::new);
    rsx! {
        section {
            class: "page-intro",
            div {
                div { class: "eyebrow", "RÉFÉRENTIELS • RULE ENGINE" }
                h2 { "Référentiels & règles versionnées" }
                p { "Les références et RuleVersions publiées sont immuables. Une évolution ajoute une nouvelle version et ne réécrit pas l'historique." }
            }
        }
        section {
            class: "panel",
            h3 { "Référentiels" }
            div {
                class: "form-grid",
                FormField { label: "Code référence", value: ref_code(), oninput: move |e: FormEvent| ref_code.set(e.value()) },
                FormField { label: "Source", value: ref_source(), oninput: move |e: FormEvent| ref_source.set(e.value()) },
                FormField { label: "URL source", value: ref_url(), oninput: move |e: FormEvent| ref_url.set(e.value()) },
                FormField { label: "Version", value: ref_version(), oninput: move |e: FormEvent| ref_version.set(e.value()) },
            }
            div {
                class: "action-row",
                button {
                    class: "primary",
                    onclick: move |_| {
                        let code = ref_code();
                        let source = ref_source();
                        let url = ref_url();
                        let version = ref_version().parse().unwrap_or(1);
                        let today = Utc::now().date_naive().to_string();
                        async move {
                            match create_versioned_reference(ReferenceDraft {
                                namespace: "GENERAL".into(), code, version_no: version,
                                valid_from: NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap_or(Utc::now().date_naive()),
                                valid_to: None, source_kind: "OFFICIAL".into(), source_name: source, source_url: url,
                                source_reference: String::new(), verified_at: Some(Utc::now().date_naive()),
                                status: "PUBLISHED".into(), payload: serde_json::json!({}),
                            }).await {
                                Ok(_) => { msg.set("Référence publiée".into()); refresh += 1; },
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    },
                    "Publier"
                }
                span { class: "save-ok", "{msg}" }
            }
        }
        section {
            class: "panel",
            for item in references.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{item.namespace} / {item.code} v{item.version_no}" }
                        div { class: "small", "{item.source_name} • validité {item.valid_from} → {valid_to_label(item.valid_to)} • {item.status}" }
                    }
                }
            }
        }
        section {
            class: "panel",
            h3 { "RuleVersions" }
            div {
                class: "form-grid",
                FormField { label: "Code règle", value: rule_code(), oninput: move |e: FormEvent| rule_code.set(e.value()) },
                FormField { label: "Libellé", value: rule_label(), oninput: move |e: FormEvent| rule_label.set(e.value()) },
                FormField { label: "Version", value: rule_version(), oninput: move |e: FormEvent| rule_version.set(e.value()) },
                FormField { label: "Définition JSON", value: rule_definition(), oninput: move |e: FormEvent| rule_definition.set(e.value()) },
                FormField { label: "Code source", value: rule_source(), oninput: move |e: FormEvent| rule_source.set(e.value()) },
                FormField { label: "Activité SARL (optionnelle)", value: rule_activity(), oninput: move |e: FormEvent| rule_activity.set(e.value()) },
                FormField { label: "Juridiction (optionnelle)", value: rule_jurisdiction(), oninput: move |e: FormEvent| rule_jurisdiction.set(e.value()) },
            }
            p { class: "small", "Opérations supportées : PASS_THROUGH, ADD, SUBTRACT, MULTIPLY_BPS." }
            div {
                class: "action-row",
                button {
                    class: "primary",
                    onclick: move |_| {
                        let code = rule_code(); let label = rule_label(); let version = rule_version().parse().unwrap_or(1);
                        let definition = serde_json::from_str(&rule_definition()).unwrap_or_else(|_| serde_json::json!({"operation":"PASS_THROUGH","path":"value"}));
                        let activity = rule_activity(); let jurisdiction = rule_jurisdiction(); let today = Utc::now().date_naive().to_string();
                        async move {
                            match create_rule_version(RuleVersionDraft {
                                rule_code: code, rule_label: label, version_no: version, legal_entity_id: Some(current_legal_entity_id()), global_scope: false,
                                activity_code: if activity.trim().is_empty() { None } else { Some(activity) },
                                jurisdiction_code: if jurisdiction.trim().is_empty() { None } else { Some(jurisdiction) },
                                valid_from: NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap_or(Utc::now().date_naive()),
                                valid_to: None, status: "DRAFT".into(), source_reference_id: None, definition, change_note: "Création initiale".into(),
                            }).await {
                                Ok(_) => { msg.set("RuleVersion enregistrée en brouillon".into()); refresh += 1; },
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    },
                    "Enregistrer le brouillon"
                }
            }
        }
        section {
            class: "panel",
            for item in rules.read().as_deref().unwrap_or(&[]).iter() {
                RuleVersionRow { item: item.clone(), refresh, source_code: rule_source() }
            }
        }
        section {
            class: "panel",
            h3 { "Reproductibilité des calculs" }
            p { class: "small", "Chaque calcul critique conserve ses entrées, sa règle/version, sa source, sa formule, son résultat et son mode d'arrondi. L'API explique un résultat par son identifiant de run." }
        }
    }
}
#[component]
fn RuleVersionRow(item: RuleVersionItem, mut refresh: Signal<u64>, source_code: String) -> Element {
    let id = item.id;
    let source = source_code.clone();
    rsx! {
        div {class:"data-row",
            div {
                div { class: "data-title", {format!("{} — v{}", item.rule_code, item.version_no)} }
                div { class: "small", {format!("{} • {} → {} • {}", item.rule_label, item.valid_from, item.valid_to.map(|d| d.to_string()).unwrap_or_else(|| "ouverte".into()), item.status)} }
                div { class: "small", {format!("Source: {}", if item.source_name.is_empty() { "non renseignée" } else { &item.source_name })} }
            }
            if item.status=="DRAFT" {
                button {class:"secondary",onclick:move |_|{let source=source.clone();async move{if publish_rule_version(id,source).await.is_ok(){refresh+=1}}},"Publier"}
            }
        }
    }
}

#[component]
fn FormField(label: &'static str, value: String, oninput: EventHandler<FormEvent>) -> Element {
    rsx! {label {class:"field",span {"{label}"},input {value:value,oninput:oninput}}}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_definition_is_deterministic() {
        let inputs = serde_json::json!({"a":10,"b":7});
        let def = serde_json::json!({"operation":"ADD","paths":["a","b"]});
        assert_eq!(evaluate_definition(&def,&inputs).unwrap(), Value::from(17));
    }

    #[test]
    fn subtract_definition_is_deterministic() {
        let inputs = serde_json::json!({"a":10,"b":7});
        let def = serde_json::json!({"operation":"SUBTRACT","paths":["a","b"]});
        assert_eq!(evaluate_definition(&def,&inputs).unwrap(), Value::from(3));
    }

    #[test]
    fn published_rule_requires_source_by_contract() {
        let d=RuleVersionDraft{rule_code:"RULE".into(),rule_label:"Rule".into(),version_no:1,legal_entity_id:None,global_scope:true,activity_code:None,jurisdiction_code:None,valid_from:Utc::now().date_naive(),valid_to:None,status:"PUBLISHED".into(),source_reference_id:None,definition:serde_json::json!({}),change_note:String::new()};
        assert_eq!(d.status,"PUBLISHED");
        assert!(d.source_reference_id.is_none());
    }
}
