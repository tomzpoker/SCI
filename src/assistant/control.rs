use crate::assistant::{AssistantAction, AssistantContext, AssistantIntent, AssistantRequest, AssistantResponse};
use crate::assistant::llm::{ConfiguredLlmProvider, LlmProviderKind};
use crate::assistant::tools::{default_tool_catalog, AssistantTool, AssistantToolSpec, ToolResult};
use crate::entity_scope::current_legal_entity_id;
use crate::ui::FormField;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiProviderItem {
    pub id: Uuid,
    pub provider_kind: String,
    pub name: String,
    pub model: String,
    pub endpoint_reference: String,
    pub credential_reference: String,
    pub command_reference: String,
    pub supports_text: bool,
    pub supports_voice: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiInvocationItem {
    pub id: Uuid,
    pub tool_code: String,
    pub risk_level: String,
    pub status: String,
    pub confirmation_required: bool,
    pub validation_request_id: Option<Uuid>,
    pub proposed_payload: Value,
    pub result_payload: Value,
    pub error_message: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiMemoryItem {
    pub id: Uuid,
    pub category: String,
    pub memory_key: String,
    pub memory_value: Value,
    pub user_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiChatItem {
    pub id: Uuid,
    pub speaker: String,
    pub content: String,
    pub tool_code: String,
    pub created_at: String,
}


fn requested_tool(message: &str) -> Option<AssistantTool> {
    let n = message.to_lowercase();
    if n.contains("solde bancaire") || n.contains("solde bancaire réel") || n.contains("solde réel") {
        Some(AssistantTool::GetRealBankBalance)
    } else if n.contains("solde") || n.contains("trésorerie disponible") {
        Some(AssistantTool::GetCashBalance)
    } else if n.contains("prévision") || n.contains("prevision") {
        Some(AssistantTool::GetForecast { horizon_months: 12 })
    } else if n.contains("impay") {
        Some(AssistantTool::GetUnpaidRents)
    } else if n.contains("échéance") || n.contains("echeance") || n.contains("tâche de la semaine") || n.contains("tache de la semaine") {
        Some(AssistantTool::GetUpcomingDeadlines)
    } else if n.contains("tva") {
        let today = Utc::now().date_naive();
        let start = today.with_day(1).unwrap_or(today);
        Some(AssistantTool::CalculateVat { period_start: start, period_end: today })
    } else if n.contains("document") || n.contains("pdf") {
        Some(AssistantTool::SearchDocuments { query: message.trim().to_owned() })
    } else {
        None
    }
}

async fn business_tool(tool: AssistantTool) -> Result<ToolResult, ServerFnError> {
    match tool {
        AssistantTool::GetCashBalance => {
            let v = crate::banking::bank_cash_positions(None).await?;
            Ok(ToolResult{success:true,data:json!({"solde_theorique_cents":v.theoretical_cents,"solde_disponible_cents":v.available_cents}),error:None,uncertainty:String::new()})
        }
        AssistantTool::GetRealBankBalance => {
            let v = crate::banking::bank_cash_positions(None).await?;
            Ok(ToolResult{success:true,data:json!({"bancaire_importe_cents":v.imported_bank_cents,"reel_rapproche_cents":v.reconciled_cents}),error:None,uncertainty:String::new()})
        }
        AssistantTool::GetForecast{horizon_months} => {
            let h = horizon_months.clamp(1,120);
            let v = crate::treasury::build_treasury_forecast(h, "BASE".to_owned()).await?;
            Ok(ToolResult{success:true,data:serde_json::to_value(v).unwrap_or(Value::Null),error:None,uncertainty:String::new()})
        }
        AssistantTool::GetUnpaidRents => {
            let v = crate::collections::list_arrears_cases().await?;
            let total:i64=v.iter().map(|x|x.outstanding_cents).sum();
            Ok(ToolResult{success:true,data:json!({"count":v.len(),"outstanding_cents":total,"cases":v}),error:None,uncertainty:if v.is_empty(){"Aucun dossier d'impayé enregistré.".into()}else{String::new()}})
        }
        AssistantTool::GetUpcomingDeadlines => {
            let v = crate::server::list_tasks().await?;
            let end = Utc::now() + Duration::days(7);
            let upcoming:Vec<_>=v.into_iter().filter(|t|t.due_at <= end && t.due_at >= Utc::now()).collect();
            Ok(ToolResult{success:true,data:serde_json::to_value(&upcoming).unwrap_or(Value::Array(vec![])),error:None,uncertainty:if upcoming.is_empty(){"Aucune tâche à échéance dans les 7 prochains jours.".into()}else{String::new()}})
        }
        AssistantTool::CalculateRentRevision{lease_id} => {
            let today = Utc::now().date_naive();
            let v=crate::leases::calculate_lease_rent_revision(lease_id,"INDEXATION".into(),today.format("%Y-%m").to_string(),today).await?;
            Ok(ToolResult{success:true,data:serde_json::to_value(v).unwrap_or(Value::Null),error:None,uncertainty:String::new()})
        }
        AssistantTool::CalculateVat{period_start,period_end} => {
            let v=crate::fiscal::calculate_vat_preview(period_start,period_end).await?;
            Ok(ToolResult{success:true,data:serde_json::to_value(v).unwrap_or(Value::Null),error:None,uncertainty:String::new()})
        }
        AssistantTool::PrepareVatReturn{period_start,period_end} => {
            let v=crate::fiscal::calculate_vat_declaration(period_start,period_end).await?;
            Ok(ToolResult{success:true,data:serde_json::to_value(v).unwrap_or(Value::Null),error:None,uncertainty:"Préparation créée; validation humaine requise avant finalisation.".into()})
        }
        AssistantTool::PrepareInvoice{lease_id,issue_date,due_date} => {
            crate::server::create_invoice_from_lease(lease_id,issue_date,due_date).await?;
            Ok(ToolResult{success:true,data:json!({"lease_id":lease_id,"issue_date":issue_date,"due_date":due_date,"status":"DRAFT"}),error:None,uncertainty:"La facture est préparée comme brouillon.".into()})
        }
        AssistantTool::PrepareReminder{arrears_case_id} => {
            let id=crate::collections::prepare_collection_action(arrears_case_id,"REMINDER".into()).await?;
            Ok(ToolResult{success:true,data:json!({"collection_action_id":id,"status":"VALIDATION_REQUIRED"}),error:None,uncertainty:"Aucun envoi n'est effectué automatiquement.".into()})
        }
        AssistantTool::SearchDocuments{query} => {
            let v=crate::documents::workflow::search_documents(query.clone()).await?;
            Ok(ToolResult{success:true,data:serde_json::to_value(v).unwrap_or(Value::Array(vec![])),error:None,uncertainty:String::new()})
        }
        AssistantTool::GetLease{lease_id} => {
            let v=crate::leases::list_lease_details().await?; let item=v.into_iter().find(|x|x.id==lease_id);
            Ok(ToolResult{success:item.is_some(),data:serde_json::to_value(&item).unwrap_or(Value::Null),error:None,uncertainty:if item.is_none(){"Bail introuvable dans l'entité active.".into()}else{String::new()}})
        }
        AssistantTool::GetTenant{tenant_id} => {
            let v=crate::server::list_tenants().await?; let item=v.into_iter().find(|x|x.id==tenant_id);
            Ok(ToolResult{success:item.is_some(),data:serde_json::to_value(&item).unwrap_or(Value::Null),error:None,uncertainty:if item.is_none(){"Locataire introuvable dans l'entité active.".into()}else{String::new()}})
        }
        AssistantTool::GetProperty{property_id} => {
            let v=crate::server::list_properties().await?; let item=v.into_iter().find(|x|x.id==property_id);
            Ok(ToolResult{success:item.is_some(),data:serde_json::to_value(&item).unwrap_or(Value::Null),error:None,uncertainty:if item.is_none(){"Bien introuvable dans l'entité active.".into()}else{String::new()}})
        }
        AssistantTool::SimulateTax{label,taxable_base_cents,rate_bp} => {
            let v=crate::fiscal::simulate_tax(label,taxable_base_cents,rate_bp).await?;
            Ok(ToolResult{success:true,data:v,error:None,uncertainty:"Simulation uniquement; aucune écriture déclarative.".into()})
        }
        AssistantTool::CreateDraft{template_code,reference,recipient,body} => {
            let payload=json!({"body":body}); let v=crate::generation::create_generated_document(template_code,reference,recipient,payload).await?;
            Ok(ToolResult{success:true,data:serde_json::to_value(v).unwrap_or(Value::Null),error:None,uncertainty:"Brouillon prévisualisé; sans effet externe.".into()})
        }
        AssistantTool::RequestUserValidation{action_code,subject_type,subject_id,proposed_payload,reason,risk_level} => {
            #[cfg(feature="server")] {
                let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;
                let entity=current_legal_entity_id();
                let workspace:Uuid=sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
                let id:Uuid=sqlx::query_scalar("INSERT INTO validation_requests(workspace_id,legal_entity_id,action_code,subject_type,subject_id,before_payload,proposed_payload,reason,source_payload,documents_payload,expected_result,risk_level) VALUES($1,$2,$3,$4,$5,'{}'::jsonb,$6,$7,'[]'::jsonb,'[]'::jsonb,'{}'::jsonb,$8) RETURNING id").bind(workspace).bind(entity).bind(action_code).bind(subject_type).bind(subject_id).bind(&proposed_payload).bind(reason).bind(risk_level.to_uppercase()).fetch_one(pool).await.map_err(ServerFnError::new)?;
                Ok(ToolResult{success:true,data:json!({"validation_request_id":id,"status":"PENDING"}),error:None,uncertainty:"Validation humaine requise.".into()})
            }
            #[cfg(not(feature="server"))] { let _=(action_code,subject_type,subject_id,proposed_payload,reason,risk_level); Err(ServerFnError::new("request_user_validation est exécutée côté serveur")) }
        }
    }
}

fn spec_for_tool(tool:&AssistantTool)->Option<AssistantToolSpec>{
    let code=match tool {
        AssistantTool::GetCashBalance=>"get_cash_balance",AssistantTool::GetRealBankBalance=>"get_real_bank_balance",AssistantTool::GetForecast{..}=>"get_forecast",AssistantTool::GetUnpaidRents=>"get_unpaid_rents",AssistantTool::GetUpcomingDeadlines=>"get_upcoming_deadlines",AssistantTool::CalculateRentRevision{..}=>"calculate_rent_revision",AssistantTool::CalculateVat{..}=>"calculate_vat",AssistantTool::PrepareVatReturn{..}=>"prepare_vat_return",AssistantTool::PrepareInvoice{..}=>"prepare_invoice",AssistantTool::PrepareReminder{..}=>"prepare_reminder",AssistantTool::SearchDocuments{..}=>"search_documents",AssistantTool::GetLease{..}=>"get_lease",AssistantTool::GetTenant{..}=>"get_tenant",AssistantTool::GetProperty{..}=>"get_property",AssistantTool::SimulateTax{..}=>"simulate_tax",AssistantTool::CreateDraft{..}=>"create_draft",AssistantTool::RequestUserValidation{..}=>"request_user_validation"};
    default_tool_catalog().into_iter().find(|x|x.code==code)
}

fn provider_status(enabled: bool) -> &'static str {
    if enabled { "ACTIF" } else { "inactif" }
}

#[server]
pub async fn list_ai_providers()->Result<Vec<AiProviderItem>,ServerFnError>{
    #[cfg(feature="server")] {
        use sqlx::Row; let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT id,provider_kind,name,model,endpoint_reference,credential_reference,command_reference,supports_text,supports_voice,enabled FROM ai_provider_configs WHERE legal_entity_id=$1 ORDER BY priority,name").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r|AiProviderItem{id:r.get("id"),provider_kind:r.get("provider_kind"),name:r.get("name"),model:r.get("model"),endpoint_reference:r.get("endpoint_reference"),credential_reference:r.get("credential_reference"),command_reference:r.get("command_reference"),supports_text:r.get("supports_text"),supports_voice:r.get("supports_voice"),enabled:r.get("enabled")}).collect())
    }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_ai_providers est exécutée côté serveur"))
}

#[server]
pub async fn save_ai_provider(provider_kind:String,name:String,model:String,endpoint_reference:String,credential_reference:String,command_reference:String,supports_text:bool,supports_voice:bool,enabled:bool)->Result<Uuid,ServerFnError>{
    #[cfg(feature="server")] {
        let kind=provider_kind.trim().to_uppercase(); if !matches!(kind.as_str(),"LOCAL_LLM"|"EXTERNAL_LLM"|"DISABLED"){return Err(ServerFnError::new("Type de fournisseur IA invalide"));} if name.trim().is_empty(){return Err(ServerFnError::new("Nom du fournisseur requis"));}
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id();
        if enabled {sqlx::query("UPDATE ai_provider_configs SET enabled=false,updated_at=now() WHERE legal_entity_id=$1").bind(e).execute(pool).await.map_err(ServerFnError::new)?;}
        let id:Uuid=sqlx::query_scalar("INSERT INTO ai_provider_configs(legal_entity_id,provider_kind,name,model,endpoint_reference,credential_reference,command_reference,supports_text,supports_voice,enabled) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT(legal_entity_id,provider_kind,name) DO UPDATE SET model=EXCLUDED.model,endpoint_reference=EXCLUDED.endpoint_reference,credential_reference=EXCLUDED.credential_reference,command_reference=EXCLUDED.command_reference,supports_text=EXCLUDED.supports_text,supports_voice=EXCLUDED.supports_voice,enabled=EXCLUDED.enabled,updated_at=now() RETURNING id").bind(e).bind(&kind).bind(name.trim()).bind(model.trim()).bind(endpoint_reference.trim()).bind(credential_reference.trim()).bind(command_reference.trim()).bind(supports_text).bind(supports_voice).bind(enabled).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature="server"))] { let _=(provider_kind,name,model,endpoint_reference,credential_reference,command_reference,supports_text,supports_voice,enabled); Err(ServerFnError::new("save_ai_provider est exécutée côté serveur")) }
}

#[server]
pub async fn list_ai_tools()->Result<Vec<AssistantToolSpec>,ServerFnError>{
    #[cfg(feature="server")] { use sqlx::Row; let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT tool_code,label_fr,risk_level,read_only,confirmation_required,authorized_roles FROM ai_tool_catalog WHERE legal_entity_id=$1 AND enabled=true ORDER BY tool_code").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r| AssistantToolSpec{code:r.get("tool_code"),label_fr:r.get("label_fr"),risk_level:r.get("risk_level"),read_only:r.get("read_only"),confirmation_required:r.get("confirmation_required"),authorized_roles:serde_json::from_value(r.get::<Value,_>("authorized_roles")).unwrap_or_else(|_| vec!["MANAGER".into()])}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_ai_tools est exécutée côté serveur"))
}

#[server]
pub async fn execute_ai_tool(tool:AssistantTool)->Result<ToolResult,ServerFnError>{
    let spec=spec_for_tool(&tool).ok_or_else(||ServerFnError::new("Tool IA inconnu"))?;
    #[cfg(feature="server")] { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let principal=crate::security::require_permission(pool,"AI_USE").await.map_err(ServerFnError::new)?; if !spec.authorized_roles.iter().any(|r|r.eq_ignore_ascii_case(principal.role.as_deref().unwrap_or(""))){return Err(ServerFnError::new("Rôle non autorisé pour ce tool"));} }
    if spec.confirmation_required {
        #[cfg(feature="server")] {
            let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let workspace:Uuid=sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?; let code=spec.code.clone(); let payload=serde_json::to_value(&tool).unwrap_or(Value::Null);
            let validation:Uuid=sqlx::query_scalar(r##"INSERT INTO validation_requests(workspace_id,legal_entity_id,action_code,subject_type,subject_id,before_payload,proposed_payload,reason,source_payload,documents_payload,expected_result,risk_level) VALUES($1,$2,$3,'AI_TOOL',NULL,'{}'::jsonb,$4,'Validation humaine requise pour un tool IA à impact','["AI"]'::jsonb,'[]'::jsonb,'{"status":"EXECUTED"}'::jsonb,$5) RETURNING id"##).bind(workspace).bind(entity).bind(format!("AI_TOOL:{}",code)).bind(&payload).bind(&spec.risk_level).fetch_one(pool).await.map_err(ServerFnError::new)?;
            let id:Uuid=sqlx::query_scalar("INSERT INTO ai_tool_invocations(legal_entity_id,tool_code,risk_level,read_only,confirmation_required,status,validation_request_id,arguments) VALUES($1,$2,$3,$4,$5,'VALIDATION_REQUIRED',$6,$7) RETURNING id").bind(entity).bind(&code).bind(&spec.risk_level).bind(spec.read_only).bind(spec.confirmation_required).bind(validation).bind(payload).fetch_one(pool).await.map_err(ServerFnError::new)?;
            return Ok(ToolResult{success:false,data:json!({"invocation_id":id,"validation_request_id":validation,"status":"VALIDATION_REQUIRED"}),error:None,uncertainty:"Une validation humaine explicite est nécessaire avant exécution.".into()});
        }
    }
    business_tool(tool).await
}

#[server]
pub async fn execute_approved_ai_tool(invocation_id:Uuid)->Result<ToolResult,ServerFnError>{
    #[cfg(feature="server")] {
        use sqlx::Row; let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id();
        let row=sqlx::query("SELECT tool_code,arguments,validation_request_id,status FROM ai_tool_invocations WHERE id=$1 AND legal_entity_id=$2").bind(invocation_id).bind(e).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Invocation IA introuvable"))?;
        if row.get::<String,_>("status")!="VALIDATION_REQUIRED"{return Err(ServerFnError::new("Invocation déjà traitée"));}
        let validation:Uuid=row.get("validation_request_id"); let vstatus:String=sqlx::query_scalar("SELECT status FROM validation_requests WHERE id=$1 AND legal_entity_id=$2").bind(validation).bind(e).fetch_one(pool).await.map_err(ServerFnError::new)?; if vstatus!="APPROVED"{return Err(ServerFnError::new("Validation humaine non approuvée"));}
        let code:String=row.get("tool_code"); let payload:Value=row.get("arguments");
        let tool=tool_from_json(&code,payload.clone()).ok_or_else(||ServerFnError::new("Payload du tool invalide"))?;
        let spec=default_tool_catalog().into_iter().find(|x|x.code==code).ok_or_else(||ServerFnError::new("Policy IA introuvable"))?;
        let principal=crate::security::current_principal(pool).await.map_err(ServerFnError::new)?;
        if principal.role.as_deref()==Some("AI_AGENT") { return Err(ServerFnError::new("Une session humaine est requise pour exécuter un tool à impact")); }
        if !spec.authorized_roles.iter().any(|r|r.eq_ignore_ascii_case(principal.role.as_deref().unwrap_or(""))) { return Err(ServerFnError::new("Rôle non autorisé pour ce tool")); }
        let service_request=crate::services::ServiceOperationRequest{service_code:"AI_ASSISTANT".into(),operation_code:format!("TOOL:{}",code),request_payload:payload.clone(),validation_payload:json!({"validation_request_id":validation}),postcondition_payload:json!({"success":true}),risk_level:spec.risk_level.clone()};
        let _service_id=crate::services::record_service_operation(pool,e,&service_request,"STARTED").await.map_err(ServerFnError::new)?;
        let result=business_tool(tool).await?;
        let actual=json!({"success":result.success,"data":result.data,"uncertainty":result.uncertainty});
        let post_status=if result.success{"MATCHED"}else{"MISMATCH"};
        let workspace:Uuid=sqlx::query_scalar("SELECT workspace_id FROM legal_entities WHERE id=$1 AND active=true").bind(e).fetch_one(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO postcondition_checks(workspace_id,legal_entity_id,validation_request_id,workflow_run_id,check_code,expected_payload,actual_payload,status,checked_at,error_message) VALUES($1,$2,$3,NULL,'AI_TOOL_RESULT',$4,$5,$6,now(),$7)").bind(workspace).bind(e).bind(validation).bind(json!({"success":true})).bind(&actual).bind(post_status).bind(result.error.clone().unwrap_or_default()).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE ai_tool_invocations SET status=CASE WHEN $3 THEN 'EXECUTED' ELSE 'FAILED' END,result=$4,uncertainty=$5,error_message=COALESCE($6,''),executed_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(invocation_id).bind(e).bind(result.success).bind(&result.data).bind(&result.uncertainty).bind(&result.error).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO audit_events(sci_id,legal_entity_id,actor,action,entity_type,entity_id,payload) VALUES(NULL,$1,$2,'AI_TOOL_EXECUTED','AI_TOOL_INVOCATION',$3,$4)").bind(e).bind(principal.role.as_deref().unwrap_or("USER")).bind(invocation_id).bind(json!({"tool_code":code,"success":result.success,"postcondition":post_status})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(result)
    }
    #[cfg(not(feature="server"))] { let _=invocation_id; Err(ServerFnError::new("execute_approved_ai_tool est exécutée côté serveur")) }
}

fn tool_from_json(code:&str,p:Value)->Option<AssistantTool>{
    let key=match code {
        "get_cash_balance"=>"GetCashBalance","get_real_bank_balance"=>"GetRealBankBalance","get_forecast"=>"GetForecast","get_unpaid_rents"=>"GetUnpaidRents","get_upcoming_deadlines"=>"GetUpcomingDeadlines","calculate_rent_revision"=>"CalculateRentRevision","calculate_vat"=>"CalculateVat","prepare_vat_return"=>"PrepareVatReturn","prepare_invoice"=>"PrepareInvoice","prepare_reminder"=>"PrepareReminder","search_documents"=>"SearchDocuments","get_lease"=>"GetLease","get_tenant"=>"GetTenant","get_property"=>"GetProperty","simulate_tax"=>"SimulateTax","create_draft"=>"CreateDraft","request_user_validation"=>"RequestUserValidation",_=>return None
    };
    let inner=p.get(key).cloned().unwrap_or(p);
    serde_json::from_value(inner).ok()
}

#[server]
pub async fn list_ai_invocations()->Result<Vec<AiInvocationItem>,ServerFnError>{
    #[cfg(feature="server")] { use sqlx::Row; let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT id,tool_code,risk_level,status,confirmation_required,validation_request_id,arguments,result,COALESCE(error_message,'') AS error_message,created_at FROM ai_tool_invocations WHERE legal_entity_id=$1 ORDER BY created_at DESC LIMIT 100").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r|AiInvocationItem{id:r.get("id"),tool_code:r.get("tool_code"),risk_level:r.get("risk_level"),status:r.get("status"),confirmation_required:r.get("confirmation_required"),validation_request_id:r.get("validation_request_id"),proposed_payload:r.get("arguments"),result_payload:r.get("result"),error_message:r.get("error_message"),created_at:r.get::<chrono::DateTime<Utc>,_>("created_at").to_rfc3339()}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_ai_invocations est exécutée côté serveur"))
}

#[server]
pub async fn list_ai_memory()->Result<Vec<AiMemoryItem>,ServerFnError>{
    #[cfg(feature="server")] { use sqlx::Row; let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT id,category,memory_key,value,user_approved FROM ai_memory WHERE legal_entity_id=$1 ORDER BY created_at DESC").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r|AiMemoryItem{id:r.get("id"),category:r.get("category"),memory_key:r.get("memory_key"),memory_value:r.get("value"),user_approved:r.get("user_approved")}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_ai_memory est exécutée côté serveur"))
}

#[server]
pub async fn save_ai_memory(category:String,memory_key:String,memory_value:Value,user_approved:bool)->Result<Uuid,ServerFnError>{
    #[cfg(feature="server")] { let cat=category.trim().to_uppercase(); if !matches!(cat.as_str(),"PREFERENCE"|"HABIT"|"AUTOMATION"|"DOCUMENT_STYLE"|"UX"){return Err(ServerFnError::new("Catégorie de mémoire IA interdite"));} if !user_approved{return Err(ServerFnError::new("La mémoire IA doit être approuvée par l'utilisateur"));} let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let id:Uuid=sqlx::query_scalar("INSERT INTO ai_memory(legal_entity_id,category,memory_key,value,user_approved) VALUES($1,$2,$3,$4,true) RETURNING id").bind(current_legal_entity_id()).bind(cat).bind(memory_key.trim()).bind(memory_value).fetch_one(pool).await.map_err(ServerFnError::new)?; Ok(id) }
    #[cfg(not(feature="server"))] { let _=(category,memory_key,memory_value,user_approved); Err(ServerFnError::new("save_ai_memory est exécutée côté serveur")) }
}

#[server]
pub async fn chat_with_assistant(message:String,input_mode:String,output_mode:String)->Result<AssistantResponse,ServerFnError>{
    #[cfg(feature="server")]
    let conversation_id = {
        use sqlx::Row;
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let e=current_legal_entity_id();
        let provider_kind:String=sqlx::query_scalar("SELECT provider_kind FROM ai_provider_configs WHERE legal_entity_id=$1 AND enabled=true ORDER BY priority,name LIMIT 1").bind(e).fetch_optional(pool).await.map_err(ServerFnError::new)?.unwrap_or_else(||"DISABLED".into());
        let id:Uuid=sqlx::query_scalar("INSERT INTO ai_conversations(legal_entity_id,title,input_mode,output_mode,provider_kind) VALUES($1,'Assistant SCI',$2,$3,$4) RETURNING id").bind(e).bind(input_mode.to_uppercase()).bind(if output_mode.to_uppercase()=="SYNTHESIZED_VOICE"{"SYNTHESIZED_VOICE"}else{"TEXT"}).bind(provider_kind).fetch_one(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO ai_messages(legal_entity_id,conversation_id,role,content,grounded,uncertainty) VALUES($1,$2,'USER',$3,true,'')").bind(e).bind(id).bind(message.trim()).execute(pool).await.map_err(ServerFnError::new)?;
        id
    };
    #[cfg(not(feature="server"))]
    let conversation_id=Uuid::nil();
    let tool=requested_tool(&message); let context=AssistantContext{workspace_id:Uuid::nil(),legal_entity_id:current_legal_entity_id()};
    if let Some(tool)=tool {
        let code=match &tool {AssistantTool::GetCashBalance=>"get_cash_balance",AssistantTool::GetRealBankBalance=>"get_real_bank_balance",AssistantTool::GetForecast{..}=>"get_forecast",AssistantTool::GetUnpaidRents=>"get_unpaid_rents",AssistantTool::GetUpcomingDeadlines=>"get_upcoming_deadlines",AssistantTool::CalculateVat{..}=>"calculate_vat",AssistantTool::SearchDocuments{..}=>"search_documents",_=>""};
        if code.is_empty(){return Ok(crate::assistant::handle_request(&AssistantRequest{message,intent:AssistantIntent::Information},&context));}
        let result=business_tool(tool).await?; let mut response=match code {
            "get_cash_balance"=>format!("Solde disponible : {:.2} € ; solde théorique : {:.2} €.",result.data.get("solde_disponible_cents").and_then(Value::as_i64).unwrap_or(0) as f64/100.0,result.data.get("solde_theorique_cents").and_then(Value::as_i64).unwrap_or(0) as f64/100.0),
            "get_real_bank_balance"=>format!("Solde bancaire importé : {:.2} € ; solde rapproché : {:.2} €.",result.data.get("bancaire_importe_cents").and_then(Value::as_i64).unwrap_or(0) as f64/100.0,result.data.get("reel_rapproche_cents").and_then(Value::as_i64).unwrap_or(0) as f64/100.0),
            "get_unpaid_rents"=>format!("{} dossier(s) d'impayé, pour {:.2} € restant(s).",result.data.get("count").and_then(Value::as_u64).unwrap_or(0),result.data.get("outstanding_cents").and_then(Value::as_i64).unwrap_or(0) as f64/100.0),
            "get_upcoming_deadlines"=>format!("{} tâche(s) arrivent à échéance dans les 7 prochains jours.",result.data.as_array().map(|a|a.len()).unwrap_or(0)),
            "calculate_vat"=>format!("TVA calculée pour la période : CA HT {:.2} €, TVA collectée {:.2} €, TVA déductible {:.2} €, solde {:.2} €.",result.data.get("ca_ht_cents").and_then(Value::as_i64).unwrap_or(0) as f64/100.0,result.data.get("collected_vat_cents").and_then(Value::as_i64).unwrap_or(0) as f64/100.0,result.data.get("deductible_vat_cents").and_then(Value::as_i64).unwrap_or(0) as f64/100.0,result.data.get("payable_vat_cents").and_then(Value::as_i64).unwrap_or(0) as f64/100.0),
            "search_documents"=>format!("{} document(s) correspondent à la recherche.",result.data.as_array().map(|a|a.len()).unwrap_or(0)),
            _=>"Données récupérées depuis le moteur métier.".into(),
        };
        if !result.uncertainty.is_empty(){response.push_str(&format!(" {}",result.uncertainty));}
        if input_mode.to_uppercase()=="TRANSCRIBED_VOICE" { response.push_str(" Entrée reçue sous forme de transcription vocale."); }
        if output_mode.to_uppercase()=="SYNTHESIZED_VOICE" { response.push_str(" Sortie voix demandée : le moteur fournit le texte source à une synthèse vocale externe."); }
        let mut out=AssistantResponse::information(response.clone()); out.provider_id=Some(active_llm_provider_id().await.unwrap_or_else(|_|"disabled".into())); out.uncertainty=result.uncertainty.clone();
        #[cfg(feature="server")] { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; sqlx::query("INSERT INTO ai_messages(legal_entity_id,conversation_id,role,content,grounded,uncertainty,tool_code) VALUES($1,$2,'ASSISTANT',$3,true,$4,$5)").bind(current_legal_entity_id()).bind(conversation_id).bind(&response).bind(&result.uncertainty).bind(code).execute(pool).await.map_err(ServerFnError::new)?; sqlx::query("UPDATE ai_conversations SET updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(conversation_id).bind(current_legal_entity_id()).execute(pool).await.map_err(ServerFnError::new)?; }
        return Ok(out);
    }
    let out=crate::assistant::handle_request(&AssistantRequest{message,intent:AssistantIntent::Information},&context);
    #[cfg(feature="server")] { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; sqlx::query("INSERT INTO ai_messages(legal_entity_id,conversation_id,role,content,grounded,uncertainty) VALUES($1,$2,'ASSISTANT',$3,true,$4)").bind(current_legal_entity_id()).bind(conversation_id).bind(&out.message).bind(&out.uncertainty).execute(pool).await.map_err(ServerFnError::new)?; }
    Ok(out)
}

async fn active_llm_provider_id()->Result<String,ServerFnError>{
    #[cfg(feature="server")] { use sqlx::Row; let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let row=sqlx::query("SELECT id,provider_kind FROM ai_provider_configs WHERE legal_entity_id=$1 AND enabled=true ORDER BY priority,name LIMIT 1").bind(current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?; if let Some(r)=row { return Ok(format!("{}:{}",r.get::<Uuid,_>("id"),r.get::<String,_>("provider_kind"))); } }
    Ok(ConfiguredLlmProvider::disabled().id)
}

#[component]
pub fn AssistantPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let providers = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_ai_providers().await.unwrap_or_default() }
    });
    let tools = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_ai_tools().await.unwrap_or_else(|_| default_tool_catalog()) }
    });
    let inv = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_ai_invocations().await.unwrap_or_default() }
    });
    let mut message = use_signal(String::new);
    let mut result = use_signal(String::new);
    let mut input_mode = use_signal(|| "TEXT".to_owned());
    let mut output_mode = use_signal(|| "TEXT".to_owned());
    let mut provider_kind = use_signal(|| "DISABLED".to_owned());
    let mut provider_name = use_signal(|| "IA désactivée".to_owned());
    let mut provider_model = use_signal(String::new);
    let mut provider_endpoint = use_signal(String::new);
    let mut provider_credential = use_signal(String::new);
    let mut provider_command = use_signal(String::new);
    let mut provider_msg = use_signal(String::new);

    rsx! {
        section {
            class: "panel",
            h2 { "Assistant IA" }
            p { class: "small", "L'IA est optionnelle. Les chiffres réels viennent des moteurs métier; les actions à impact passent par validation humaine." }
            div {
                class: "form-grid",
                FormField { label: "Question", value: message(), oninput: move |e: FormEvent| message.set(e.value()) }
                label {
                    class: "field",
                    span { "Entrée" }
                    select {
                        value: input_mode(),
                        onchange: move |e: FormEvent| input_mode.set(e.value()),
                        option { value: "TEXT", "Texte" }
                        option { value: "TRANSCRIBED_VOICE", "Voix transcrite" }
                    }
                }
                label {
                    class: "field",
                    span { "Sortie" }
                    select {
                        value: output_mode(),
                        onchange: move |e: FormEvent| output_mode.set(e.value()),
                        option { value: "TEXT", "Texte" }
                        option { value: "SYNTHESIZED_VOICE", "Texte + voix" }
                    }
                }
            }
            button {
                class: "primary",
                onclick: move |_| {
                    let m = message();
                    let i = input_mode();
                    let o = output_mode();
                    async move {
                        match chat_with_assistant(m, i, o).await {
                            Ok(value) => result.set(value.message),
                            Err(error) => result.set(error.to_string()),
                        }
                    }
                },
                "Interroger l’assistant"
            }
            if !result().is_empty() {
                div { class: "callout", "{result}" }
            }
        }
        section {
            class: "panel",
            h2 { "Configuration du provider IA" }
            div {
                class: "form-grid",
                FormField { label: "Type", value: provider_kind(), oninput: move |e: FormEvent| provider_kind.set(e.value()) }
                FormField { label: "Nom", value: provider_name(), oninput: move |e: FormEvent| provider_name.set(e.value()) }
                FormField { label: "Modèle", value: provider_model(), oninput: move |e: FormEvent| provider_model.set(e.value()) }
                FormField { label: "Référence endpoint", value: provider_endpoint(), oninput: move |e: FormEvent| provider_endpoint.set(e.value()) }
                FormField { label: "Référence credential", value: provider_credential(), oninput: move |e: FormEvent| provider_credential.set(e.value()) }
                FormField { label: "Référence commande locale", value: provider_command(), oninput: move |e: FormEvent| provider_command.set(e.value()) }
            }
            button {
                class: "secondary",
                onclick: move |_| {
                    let k = provider_kind();
                    let n = provider_name();
                    let m = provider_model();
                    let e = provider_endpoint();
                    let c = provider_credential();
                    let cmd = provider_command();
                    async move {
                        match save_ai_provider(k, n, m, e, c, cmd, true, false, true).await {
                            Ok(_) => { provider_msg.set("Provider enregistré et activé pour l’entité active.".into()); bump += 1; }
                            Err(error) => provider_msg.set(error.to_string()),
                        }
                    }
                },
                "Enregistrer / activer"
            }
            span { class: "save-ok", "{provider_msg}" }
            for provider in providers.read().as_deref().unwrap_or(&[]).iter().cloned() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{provider.name}" }
                        div { class: "small", "{provider.provider_kind} · {provider.model} · {provider_status(provider.enabled)}" }
                    }
                }
            }
        }
        section {
            class: "panel",
            h2 { "Tools IA autorisés" }
            for tool in tools.read().as_deref().unwrap_or(&[]).iter().cloned() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{tool.label_fr}" }
                        div { class: "small", {format!("{} · risque {} · {} · {}", tool.code, tool.risk_level, if tool.read_only { "lecture seule" } else { "modification" }, if tool.confirmation_required { "validation obligatoire" } else { "sans confirmation" })} }
                    }
                }
            }
        }
        section {
            class: "panel",
            h2 { "Invocations / validations" }
            for invocation in inv.read().as_deref().unwrap_or(&[]).iter().cloned() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{invocation.tool_code}" }
                        div { class: "small", "{invocation.status} · risque {invocation.risk_level} · {invocation.created_at}" }
                    }
                    if invocation.status == "VALIDATION_REQUIRED" {
                        button {
                            class: "secondary",
                            onclick: move |_| {
                                let id = invocation.id;
                                async move {
                                    let _ = execute_approved_ai_tool(id).await;
                                    bump += 1;
                                }
                            },
                            "Exécuter après approbation"
                        }
                    }
                }
            }
        }
    }
}

