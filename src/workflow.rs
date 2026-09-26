use crate::entity_scope::current_legal_entity_id;
#[cfg(feature="server")]
use crate::idempotency::{claim_job, complete_job};
#[cfg(feature="server")]
use crate::infrastructure::db;
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowDefinitionItem { pub id: Uuid, pub code: String, pub name: String, pub trigger_kind: String, pub version_no: i32, pub timeout_seconds: i32, pub enabled: bool, pub steps_count: i64 }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutomationPolicyItem { pub id: Uuid, pub activity_code: String, pub object_type: String, pub risk_level: String, pub automation_level: i16, pub validation_required: bool, pub enabled: bool }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationRequestItem { pub id: Uuid, pub action_code: String, pub subject_type: String, pub subject_id: Option<Uuid>, pub before_payload: Value, pub proposed_payload: Value, pub reason: String, pub source_payload: Value, pub documents_payload: Value, pub expected_result: Value, pub risk_level: String, pub status: String, pub requested_at: DateTime<Utc> }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostconditionItem { pub id: Uuid, pub check_code: String, pub expected_payload: Value, pub actual_payload: Value, pub status: String, pub checked_at: Option<DateTime<Utc>>, pub error_message: String }
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StartWorkflowResult { pub workflow_run_id: Uuid, pub status: String, pub validation_request_id: Option<Uuid>, pub idempotent_reuse: bool }

fn valid_level(level: i16) -> bool { (0..=5).contains(&level) }
fn risk(value: &str) -> Option<String> { let x=value.trim().to_uppercase(); if matches!(x.as_str(),"LOW"|"MEDIUM"|"HIGH"|"CRITICAL") { Some(x) } else { None } }
fn requires_validation(level: i16, validation_required: bool, risk_level: &str) -> bool { validation_required || level < 4 || matches!(risk_level,"HIGH"|"CRITICAL") }

#[server]
pub async fn list_workflow_definitions() -> Result<Vec<WorkflowDefinitionItem>, ServerFnError> {
    #[cfg(feature="server")]
    { let pool=db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT w.id,w.code,w.name,w.trigger_kind,w.version_no,w.timeout_seconds,w.enabled,COUNT(s.id)::bigint AS steps_count FROM workflow_definitions w LEFT JOIN workflow_steps s ON s.workflow_definition_id=w.id WHERE w.legal_entity_id=$1 GROUP BY w.id ORDER BY w.code,w.version_no DESC").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r|WorkflowDefinitionItem{id:r.get("id"),code:r.get("code"),name:r.get("name"),trigger_kind:r.get("trigger_kind"),version_no:r.get("version_no"),timeout_seconds:r.get("timeout_seconds"),enabled:r.get("enabled"),steps_count:r.get("steps_count")}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_workflow_definitions est exécutée côté serveur"))
}

#[server]
pub async fn save_automation_policy(id: Option<Uuid>, activity_code: String, object_type: String, risk_level: String, automation_level: i16, validation_required: bool, enabled: bool) -> Result<Uuid, ServerFnError> {
    #[cfg(feature="server")]
    { if !valid_level(automation_level) { return Err(ServerFnError::new("Niveau d'automatisation attendu entre 0 et 5")); } let risk_level=risk(&risk_level).ok_or_else(||ServerFnError::new("Risque invalide"))?; let pool=db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let workspace:Uuid=sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?; let activity_code=activity_code.trim().to_uppercase(); let object_type=object_type.trim().to_uppercase(); if activity_code.is_empty() && object_type.is_empty() { return Err(ServerFnError::new("Activité ou type requis")); } let id=id.unwrap_or_else(Uuid::new_v4); sqlx::query_scalar("INSERT INTO automation_policies(id,workspace_id,legal_entity_id,activity_code,object_type,risk_level,automation_level,validation_required,enabled) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT(legal_entity_id,activity_code,object_type,risk_level) DO UPDATE SET automation_level=EXCLUDED.automation_level,validation_required=EXCLUDED.validation_required,enabled=EXCLUDED.enabled,updated_at=now() RETURNING id").bind(id).bind(workspace).bind(entity).bind(&activity_code).bind(&object_type).bind(risk_level).bind(automation_level).bind(validation_required).bind(enabled).fetch_one(pool).await.map_err(ServerFnError::new) }
    #[cfg(not(feature="server"))] { let _=(id,activity_code,object_type,risk_level,automation_level,validation_required,enabled); Err(ServerFnError::new("save_automation_policy est exécutée côté serveur")) }
}

#[server]
pub async fn list_automation_policies() -> Result<Vec<AutomationPolicyItem>, ServerFnError> {
    #[cfg(feature="server")]
    { let pool=db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT id,activity_code,object_type,risk_level,automation_level,validation_required,enabled FROM automation_policies WHERE legal_entity_id=$1 ORDER BY activity_code,object_type,risk_level").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r|AutomationPolicyItem{id:r.get("id"),activity_code:r.get("activity_code"),object_type:r.get("object_type"),risk_level:r.get("risk_level"),automation_level:r.get("automation_level"),validation_required:r.get("validation_required"),enabled:r.get("enabled")}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_automation_policies est exécutée côté serveur"))
}

#[server]
pub async fn start_workflow(code: String, input_payload: Value, idempotency_key: String, activity_code: Option<String>, object_type: Option<String>, risk_level: String) -> Result<StartWorkflowResult, ServerFnError> {
    #[cfg(feature="server")]
    { let risk_level=risk(&risk_level).ok_or_else(||ServerFnError::new("Risque invalide"))?; let key=idempotency_key.trim(); if key.is_empty(){return Err(ServerFnError::new("Clé d'idempotence requise"));} let pool=db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let workflow=sqlx::query("SELECT id,enabled FROM workflow_definitions WHERE legal_entity_id=$1 AND code=$2 ORDER BY version_no DESC LIMIT 1").bind(entity).bind(code.trim()).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Workflow introuvable"))?; if !workflow.get::<bool,_>("enabled"){return Err(ServerFnError::new("Workflow désactivé"));} let workflow_id:Uuid=workflow.get("id"); let claim=claim_job(pool,entity,"WORKFLOW",key,&input_payload).await.map_err(ServerFnError::new)?; if !claim.execute { let existing=sqlx::query("SELECT id,status FROM workflow_runs WHERE legal_entity_id=$1 AND idempotency_key=$2").bind(entity).bind(key).fetch_optional(pool).await.map_err(ServerFnError::new)?; if let Some(r)=existing{return Ok(StartWorkflowResult{workflow_run_id:r.get("id"),status:r.get("status"),validation_request_id:None,idempotent_reuse:true});} return Err(ServerFnError::new("Opération déjà traitée mais exécution absente")); }
        let mut validation_id=None; let activity=activity_code.unwrap_or_default().trim().to_uppercase(); let object=object_type.unwrap_or_default().trim().to_uppercase(); let policy=sqlx::query("SELECT automation_level,validation_required FROM automation_policies WHERE legal_entity_id=$1 AND (activity_code=$2 OR activity_code='') AND (object_type=$3 OR object_type='') AND risk_level=$4 AND enabled=true ORDER BY (CASE WHEN activity_code=$2 THEN 2 ELSE 0 END)+(CASE WHEN object_type=$3 THEN 1 ELSE 0 END) DESC LIMIT 1").bind(entity).bind(&activity).bind(&object).bind(&risk_level).fetch_optional(pool).await.map_err(ServerFnError::new)?; let (level,need)=policy.map(|r|(r.get::<i16,_>("automation_level"),r.get::<bool,_>("validation_required"))).unwrap_or((2,true)); let status=if requires_validation(level,need,&risk_level){"WAITING_VALIDATION"}else{"RUNNING"}; let workspace:Uuid=sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?; let run_id:Uuid=sqlx::query_scalar("INSERT INTO workflow_runs(workspace_id,legal_entity_id,workflow_definition_id,idempotency_key,status,current_step_no,input_payload) VALUES($1,$2,$3,$4,$5,1,$6) ON CONFLICT(legal_entity_id,idempotency_key) DO UPDATE SET updated_at=now() RETURNING id").bind(workspace).bind(entity).bind(workflow_id).bind(key).bind(status).bind(&input_payload).fetch_one(pool).await.map_err(ServerFnError::new)?;
        if status=="WAITING_VALIDATION" { let req=sqlx::query_scalar("INSERT INTO validation_requests(workspace_id,legal_entity_id,workflow_run_id,action_code,subject_type,before_payload,proposed_payload,reason,source_payload,documents_payload,expected_result,risk_level) VALUES($1,$2,$3,'WORKFLOW_EXECUTE','WORKFLOW',$4,$5,'Validation requise avant effet métier',$6,$7,$8,$9) ON CONFLICT(workflow_run_id,action_code) DO UPDATE SET status=CASE WHEN validation_requests.status='PENDING' THEN 'PENDING' ELSE validation_requests.status END RETURNING id").bind(workspace).bind(entity).bind(run_id).bind(Value::Object(Default::default())).bind(&input_payload).bind(serde_json::json!([{"type":"workflow","code":code.trim()}])).bind(Value::Array(vec![])).bind(serde_json::json!({"status":"SUCCEEDED"})).bind(&risk_level).fetch_one(pool).await.map_err(ServerFnError::new)?; validation_id=Some(req); }
        complete_job(pool,claim.id,&serde_json::json!({"workflow_run_id":run_id,"status":status})).await.map_err(ServerFnError::new)?;
        Ok(StartWorkflowResult{workflow_run_id:run_id,status:status.into(),validation_request_id:validation_id,idempotent_reuse:false}) }
    #[cfg(not(feature="server"))] { let _=(code,input_payload,idempotency_key,activity_code,object_type,risk_level); Err(ServerFnError::new("start_workflow est exécutée côté serveur")) }
}

#[server]
pub async fn list_validation_requests() -> Result<Vec<ValidationRequestItem>, ServerFnError> {
    #[cfg(feature="server")]
    { let pool=db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT id,action_code,subject_type,subject_id,before_payload,proposed_payload,reason,source_payload,documents_payload,expected_result,risk_level,status,requested_at FROM validation_requests WHERE legal_entity_id=$1 ORDER BY CASE risk_level WHEN 'CRITICAL' THEN 0 WHEN 'HIGH' THEN 1 WHEN 'MEDIUM' THEN 2 ELSE 3 END,requested_at DESC").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r|ValidationRequestItem{id:r.get("id"),action_code:r.get("action_code"),subject_type:r.get("subject_type"),subject_id:r.get("subject_id"),before_payload:r.get("before_payload"),proposed_payload:r.get("proposed_payload"),reason:r.get("reason"),source_payload:r.get("source_payload"),documents_payload:r.get("documents_payload"),expected_result:r.get("expected_result"),risk_level:r.get("risk_level"),status:r.get("status"),requested_at:r.get("requested_at")}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_validation_requests est exécutée côté serveur"))
}

#[server]
pub async fn decide_validation_request(id: Uuid, approve: bool, reason: String) -> Result<(), ServerFnError> {
    #[cfg(feature="server")]
    { let pool=db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let status=if approve{"APPROVED"}else{"REJECTED"}; let r=sqlx::query("UPDATE validation_requests SET status=$3,reviewed_at=now(),reviewed_by='MANAGER',review_reason=$4 WHERE id=$1 AND legal_entity_id=$2 AND status='PENDING'").bind(id).bind(entity).bind(status).bind(reason.trim()).execute(pool).await.map_err(ServerFnError::new)?; if r.rows_affected()==0{return Err(ServerFnError::new("Validation introuvable ou déjà traitée"));} if approve {sqlx::query("UPDATE workflow_runs w SET status='RUNNING',updated_at=now() FROM validation_requests v WHERE v.id=$1 AND v.workflow_run_id=w.id AND v.legal_entity_id=$2").bind(id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;} Ok(()) }
    #[cfg(not(feature="server"))] { let _=(id,approve,reason); Err(ServerFnError::new("decide_validation_request est exécutée côté serveur")) }
}

#[server]
pub async fn record_postcondition(workflow_run_id: Uuid, check_code: String, expected_payload: Value, actual_payload: Value) -> Result<PostconditionItem, ServerFnError> {
    #[cfg(feature="server")]
    { let pool=db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let status=if expected_payload==actual_payload{"MATCHED"}else{"MISMATCH"}; let workspace:Uuid=sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let id:Uuid=sqlx::query_scalar("INSERT INTO postcondition_checks(workspace_id,legal_entity_id,workflow_run_id,check_code,expected_payload,actual_payload,status,checked_at,error_message) VALUES($1,$2,$3,$4,$5,$6,$7,now(),CASE WHEN $7='MISMATCH' THEN 'Résultat réel différent du résultat attendu' ELSE '' END) RETURNING id").bind(workspace).bind(entity).bind(workflow_run_id).bind(check_code.trim()).bind(&expected_payload).bind(&actual_payload).bind(status).fetch_one(pool).await.map_err(ServerFnError::new)?; sqlx::query("UPDATE workflow_runs SET status=CASE WHEN $2='MATCHED' THEN 'SUCCEEDED' ELSE 'FAILED' END,output_payload=$3,finished_at=now(),updated_at=now() WHERE id=$1 AND legal_entity_id=$4").bind(workflow_run_id).bind(status).bind(&actual_payload).bind(entity).execute(pool).await.map_err(ServerFnError::new)?; let row=sqlx::query("SELECT id,check_code,expected_payload,actual_payload,status,checked_at,error_message FROM postcondition_checks WHERE id=$1").bind(id).fetch_one(pool).await.map_err(ServerFnError::new)?; Ok(PostconditionItem{id:row.get("id"),check_code:row.get("check_code"),expected_payload:row.get("expected_payload"),actual_payload:row.get("actual_payload"),status:row.get("status"),checked_at:row.get("checked_at"),error_message:row.get("error_message")}) }
    #[cfg(not(feature="server"))] { let _=(workflow_run_id,check_code,expected_payload,actual_payload); Err(ServerFnError::new("record_postcondition est exécutée côté serveur")) }
}

#[cfg(feature="server")]
use sqlx::Row;

#[component]
pub fn WorkflowPage(mut refresh: Signal<u64>) -> Element {
    let defs=use_resource(move||{let _=refresh();async move{list_workflow_definitions().await.unwrap_or_default()}});
    rsx!{section {class:"page-intro",div {div {class:"eyebrow","WORKFLOW • REPRISE • IDEMPOTENCE"},h2 {"Workflows"},p {"Chaque workflow déclare son déclencheur, ses étapes, son timeout et une clé de reprise idempotente."}}}section {class:"panel",h3 {"Définitions actives"},div {class:"data-list",for item in defs.read().as_deref().unwrap_or(&[]).iter(){div {class:"data-row",div {div {class:"data-title","{item.name}"},div {class:"small","{item.code} • v{item.version_no} • {item.steps_count} étape(s) • timeout {item.timeout_seconds}s • {item.trigger_kind}"}}}}}}}
}

#[component]
pub fn ValidationInboxPage(mut refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let items = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_validation_requests().await.unwrap_or_default() }
    });
    let pending: Vec<ValidationRequestItem> = items
        .read()
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter(|item| item.status == "PENDING")
        .cloned()
        .collect();

    rsx! {
        section {
            class: "page-intro",
            div {
                div { class: "eyebrow", "VALIDATION • AVANT / APRÈS" }
                h2 { "Inbox des validations" }
                p { "Les propositions sont regroupées avec risque, sources, documents et résultat attendu avant décision." }
            }
        }
        section {
            class: "panel",
            div {
                class: "data-list",
                for item in pending {
                    div {
                        class: "data-row",
                        div {
                            div { class: "data-title", "{item.action_code} • {item.risk_level}" }
                            div { class: "small", "{item.reason}" }
                            div {
                                class: "change-grid",
                                div { strong { "Avant" }, pre { "{item.before_payload}" } }
                                div { strong { "Après" }, pre { "{item.proposed_payload}" } }
                            }
                            div { class: "small", "Résultat attendu : {item.expected_result}" }
                            div { class: "small", "Sources : {item.source_payload} • Documents : {item.documents_payload}" }
                        }
                        div {
                            class: "button-row",
                            button {
                                class: "primary",
                                onclick: move |_| {
                                    let id = item.id;
                                    async move {
                                        match decide_validation_request(id, true, String::new()).await {
                                            Ok(_) => bump += 1,
                                            Err(_) => bump += 1,
                                        }
                                    }
                                },
                                "Valider"
                            }
                            button {
                                class: "secondary",
                                onclick: move |_| {
                                    let id = item.id;
                                    async move {
                                        match decide_validation_request(id, false, String::from("Refus manager")).await {
                                            Ok(_) => bump += 1,
                                            Err(_) => bump += 1,
                                        }
                                    }
                                },
                                "Refuser"
                            }
                        }
                    }
                }
            }
        }
    }
}

