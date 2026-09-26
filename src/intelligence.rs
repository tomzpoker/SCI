use crate::observability::{self, LogDomain};
//! Intelligence layer for SCI FAMILY.
//!
//! The business application remains 100% Rust. Dioxus is the UI layer and the
//! server owns email, OCR, LLM calls, staging, validation and final storage.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use dioxus::fullstack::FileStream;
use dioxus::prelude::*;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use futures_util::StreamExt;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use mail_parser::{MessageParser, MimeHeaders};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use crate::documents::classifier::{classify, ClassificationInput};
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use crate::documents::extractor::{extract, ExtractionInput};
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use crate::documents::DocumentType;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use crate::infrastructure::db;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use sqlx::Row;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use tokio::io::AsyncWriteExt;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
use tokio::sync::OnceCell;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmRequestBody {
    model: String,
    messages: Vec<LlmMessage>,
    temperature: f32,
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
struct EmailAccountConfig {
    id: Uuid,
    label: String,
    address: String,
    imap_host: String,
    imap_port: i32,
    username: String,
    password: String,
    mailbox: String,
    tls: bool,
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
struct EmailAttachmentPayload {
    source_ref: String,
    filename: String,
    content_type: String,
    bytes: Vec<u8>,
    sender: String,
    subject: String,
    received_at: String,
}

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
static OCR_ENGINE: OnceCell<ocr::api::Ocr> = OnceCell::const_new();

#[server]
pub async fn list_llm_accounts() -> Result<Vec<crate::domain::LlmAccountItem>, ServerFnError> {
    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query("SELECT id,name,base_url,model,is_free,supports_voice,enabled,(api_key IS NOT NULL AND btrim(api_key)<>'') AS has_api_key FROM llm_accounts WHERE legal_entity_id=$1 ORDER BY enabled DESC,is_free DESC,name")
            .bind(crate::entity_scope::current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        return Ok(rows.into_iter().map(|r| crate::domain::LlmAccountItem {
            id:r.get("id"), name:r.get("name"), base_url:r.get("base_url"), model:r.get("model"),
            is_free:r.get("is_free"), supports_voice:r.get("supports_voice"), enabled:r.get("enabled"), has_api_key:r.get("has_api_key")
        }).collect());
    }
    #[cfg(any(not(feature = "server"), target_arch = "wasm32"))]
    Err(ServerFnError::new("list_llm_accounts est exécutée côté serveur"))
}

#[server]
pub async fn save_llm_account(id: Option<Uuid>, name:String, base_url:String, model:String, api_key:String, is_free:bool, supports_voice:bool, enabled:bool) -> Result<Uuid,ServerFnError> {
    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    {
        let pool=db().await.map_err(ServerFnError::new)?;
        if name.trim().is_empty()||base_url.trim().is_empty()||model.trim().is_empty(){return Err(ServerFnError::new("Nom, endpoint et modèle sont requis"));}
        let sci=crate::entity_scope::current_legal_entity_id();
        let account_id=match id {
            Some(id)=>{
                let result=if api_key.trim().is_empty(){sqlx::query("UPDATE llm_accounts SET name=$3,base_url=$4,model=$5,is_free=$6,supports_voice=$7,enabled=$8,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(sci).bind(name.trim()).bind(base_url.trim().trim_end_matches('/')).bind(model.trim()).bind(is_free).bind(supports_voice).bind(enabled).execute(pool).await}
                else {sqlx::query("UPDATE llm_accounts SET name=$3,base_url=$4,model=$5,api_key=$6,is_free=$7,supports_voice=$8,enabled=$9,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(sci).bind(name.trim()).bind(base_url.trim().trim_end_matches('/')).bind(model.trim()).bind(api_key.trim()).bind(is_free).bind(supports_voice).bind(enabled).execute(pool).await};
                result.map_err(ServerFnError::new)?;
                id
            }
            None=>sqlx::query_scalar("INSERT INTO llm_accounts(legal_entity_id,name,base_url,model,api_key,is_free,supports_voice,enabled) VALUES($1,$2,$3,$4,NULLIF($5,''),$6,$7,$8) RETURNING id").bind(sci).bind(name.trim()).bind(base_url.trim().trim_end_matches('/')).bind(model.trim()).bind(api_key.trim()).bind(is_free).bind(supports_voice).bind(enabled).fetch_one(pool).await.map_err(ServerFnError::new)?
        };
        audit_intelligence(pool,"SAVE_LLM_ACCOUNT",Some("llm_account"),Some(account_id),json!({"name":name.trim(),"model":model.trim()})).await.map_err(ServerFnError::new)?;
        return Ok(account_id);
    }
    #[cfg(any(not(feature = "server"), target_arch = "wasm32"))]
    Err(ServerFnError::new("save_llm_account est exécutée côté serveur"))
}

#[server]
pub async fn delete_llm_account(id:Uuid)->Result<(),ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;sqlx::query("DELETE FROM llm_accounts WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(crate::entity_scope::current_legal_entity_id()).execute(pool).await.map_err(ServerFnError::new)?;return Ok(());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("delete_llm_account est exécutée côté serveur"))
}

#[server]
pub async fn test_llm_account(id:Uuid,prompt:String)->Result<String,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let row=sqlx::query("SELECT base_url,model,api_key FROM llm_accounts WHERE id=$1 AND legal_entity_id=$2 AND enabled").bind(id).bind(crate::entity_scope::current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Compte LLM introuvable ou désactivé"))?;let base_url:String=row.get("base_url");let model:String=row.get("model");let key:Option<String>=row.get("api_key");return call_llm(&base_url,&model,key.as_deref(),prompt.trim(),"Réponds en français en une phrase.").await.map_err(ServerFnError::new);}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("test_llm_account est exécutée côté serveur"))
}

#[server]
pub async fn assistant_chat(message:String,intent:crate::assistant::AssistantIntent)->Result<crate::assistant::AssistantResponse,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {
        let message=message.trim().to_owned();
        if message.is_empty(){return Ok(crate::assistant::AssistantResponse::information("Décris-moi ce que tu veux vérifier ou préparer."));}
        let pool=db().await.map_err(ServerFnError::new)?;
        let context=assistant_context(pool).await.map_err(ServerFnError::new)?;
        let account=sqlx::query("SELECT id,base_url,model,api_key FROM llm_accounts WHERE legal_entity_id=$1 AND enabled ORDER BY is_free DESC,name LIMIT 1").bind(crate::entity_scope::current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        let llm_answer=if let Some(row)=&account{let id:Uuid=row.get("id");let base:String=row.get("base_url");let model:String=row.get("model");let key:Option<String>=row.get("api_key");let prompt=format!("CONTEXTE SCI:\n{}\n\nDEMANDE UTILISATEUR:\n{}",context,message);match call_llm(&base,&model,key.as_deref(),&prompt,"Tu es l'assistant administratif d'une SCI française à l'IR avec TVA sur encaissements. Ne modifie jamais les données. Pour une action, prépare une proposition claire qui sera validée séparément. Réponds en français et n'invente aucune donnée absente du contexte.").await{Ok(text)=>Some((text,id)),Err(_)=>None}}else{None};
        match intent {
            crate::assistant::AssistantIntent::Information=>{let mut r=crate::assistant::AssistantResponse::information(llm_answer.as_ref().map(|x|x.0.clone()).unwrap_or_else(||fallback_information(&message,&context)));r.provider_id=llm_answer.map(|x|x.1.to_string());Ok(r)}
            crate::assistant::AssistantIntent::PrepareTask|crate::assistant::AssistantIntent::ExecuteTask=>{
                let (action,payload)=derive_proposal(&message);
                let explanation=llm_answer.as_ref().map(|x|x.0.clone()).unwrap_or_else(||format!("Je prépare « {} ». Aucune écriture métier n'est effectuée avant ta validation explicite.",payload.get("title").and_then(|v|v.as_str()).unwrap_or(&action)));
                let proposal_id=sqlx::query_scalar("INSERT INTO assistant_proposals(legal_entity_id,action_type,payload,explanation,status) VALUES($1,$2,$3,$4,'PENDING') RETURNING id").bind(crate::entity_scope::current_legal_entity_id()).bind(&action).bind(&payload).bind(&explanation).fetch_one(pool).await.map_err(ServerFnError::new)?;
                let mut r=crate::assistant::AssistantResponse::proposal(explanation);r.proposal_id=Some(proposal_id);r.provider_id=llm_answer.map(|x|x.1.to_string());Ok(r)
            }
        }
    }
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("assistant_chat est exécutée côté serveur"))
}

#[server]
pub async fn list_assistant_proposals()->Result<Vec<crate::domain::AssistantProposalItem>,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let rows=sqlx::query("SELECT id,action_type,explanation,payload,status,created_at FROM assistant_proposals WHERE legal_entity_id=$1 ORDER BY created_at DESC LIMIT 50").bind(crate::entity_scope::current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;return Ok(rows.into_iter().map(|r|crate::domain::AssistantProposalItem{id:r.get("id"),action_type:r.get("action_type"),explanation:r.get("explanation"),payload:r.get("payload"),status:r.get("status"),created_at:r.get("created_at")}).collect());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("list_assistant_proposals est exécutée côté serveur"))
}

#[server]
pub async fn approve_assistant_proposal(id:Uuid)->Result<(),ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let mut tx=pool.begin().await.map_err(ServerFnError::new)?;let row=sqlx::query("SELECT action_type,payload,status FROM assistant_proposals WHERE id=$1 AND legal_entity_id=$2 FOR UPDATE").bind(id).bind(crate::entity_scope::current_legal_entity_id()).fetch_optional(&mut *tx).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Proposition introuvable"))?;let status:String=row.get("status");if status!="PENDING"{return Err(ServerFnError::new("Cette proposition a déjà été traitée"));}let action:String=row.get("action_type");let payload:serde_json::Value=row.get("payload");match action.as_str(){"CREATE_TASK"=>{let title=payload.get("title").and_then(|v|v.as_str()).unwrap_or("Action préparée par l'assistant").trim();let description=payload.get("description").and_then(|v|v.as_str()).unwrap_or("").trim();sqlx::query("INSERT INTO tasks(legal_entity_id,code,title,description,due_at,state,priority,source,blocking,entity_type,entity_id,occurrence_key) VALUES($1,'ASSISTANT',$2,$3,now()+interval '7 days','READY',85,'ASSISTANT',false,'assistant_proposal',$4,$5) ON CONFLICT(legal_entity_id,code,occurrence_key) DO NOTHING").bind(crate::entity_scope::current_legal_entity_id()).bind(title).bind(description).bind(id).bind(format!("assistant:{}",id)).execute(&mut *tx).await.map_err(ServerFnError::new)?;},_=>return Err(ServerFnError::new("Type d'action non pris en charge"))}sqlx::query("UPDATE assistant_proposals SET status='EXECUTED',decided_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(crate::entity_scope::current_legal_entity_id()).execute(&mut *tx).await.map_err(ServerFnError::new)?;tx.commit().await.map_err(ServerFnError::new)?;audit_intelligence(pool,"APPROVE_ASSISTANT_PROPOSAL",Some("assistant_proposal"),Some(id),payload).await.map_err(ServerFnError::new)?;return Ok(());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("approve_assistant_proposal est exécutée côté serveur"))
}

#[server]
pub async fn reject_assistant_proposal(id:Uuid,reason:String)->Result<(),ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let result=sqlx::query("UPDATE assistant_proposals SET status='REJECTED',decided_at=now() WHERE id=$1 AND legal_entity_id=$2 AND status='PENDING'").bind(id).bind(crate::entity_scope::current_legal_entity_id()).execute(pool).await.map_err(ServerFnError::new)?;if result.rows_affected()==0{return Err(ServerFnError::new("Proposition introuvable ou déjà traitée"));}audit_intelligence(pool,"REJECT_ASSISTANT_PROPOSAL",Some("assistant_proposal"),Some(id),json!({"reason":reason.trim()})).await.map_err(ServerFnError::new)?;return Ok(());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("reject_assistant_proposal est exécutée côté serveur"))
}

#[server]
pub async fn list_storage_locations()->Result<Vec<crate::domain::StorageLocationItem>,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let rows=sqlx::query("SELECT id,name,storage_type,COALESCE(configuration->>'root_path','') AS root_path,is_active FROM storage_locations WHERE legal_entity_id=$1 ORDER BY is_active DESC,name").bind(crate::entity_scope::current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;return Ok(rows.into_iter().map(|r|crate::domain::StorageLocationItem{id:r.get("id"),name:r.get("name"),storage_type:r.get("storage_type"),root_path:r.get("root_path"),is_active:r.get("is_active")}).collect());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("list_storage_locations est exécutée côté serveur"))
}

#[server]
pub async fn save_storage_location(id:Option<Uuid>,name:String,storage_type:String,root_path:String,is_active:bool)->Result<Uuid,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let kind=storage_type.trim().to_uppercase();if !["LOCAL","NETWORK","NAS"].contains(&kind.as_str()){return Err(ServerFnError::new("Type de stockage : LOCAL, NETWORK ou NAS"));}if name.trim().is_empty()||root_path.trim().is_empty(){return Err(ServerFnError::new("Nom et racine de stockage requis"));}let sci=crate::entity_scope::current_legal_entity_id();let id=match id{Some(id)=>{let r=sqlx::query("UPDATE storage_locations SET name=$3,storage_type=$4,configuration=jsonb_build_object('root_path',$5),is_active=$6,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(sci).bind(name.trim()).bind(&kind).bind(root_path.trim()).bind(is_active).execute(pool).await.map_err(ServerFnError::new)?;if r.rows_affected()==0{return Err(ServerFnError::new("Emplacement introuvable"));}id},None=>sqlx::query_scalar("INSERT INTO storage_locations(legal_entity_id,name,storage_type,configuration,is_active) VALUES($1,$2,$3,jsonb_build_object('root_path',$4),$5) RETURNING id").bind(sci).bind(name.trim()).bind(&kind).bind(root_path.trim()).bind(is_active).fetch_one(pool).await.map_err(ServerFnError::new)?};return Ok(id);}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("save_storage_location est exécutée côté serveur"))
}

#[server]
pub async fn delete_storage_location(id:Uuid)->Result<(),ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;sqlx::query("DELETE FROM storage_locations WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(crate::entity_scope::current_legal_entity_id()).execute(pool).await.map_err(ServerFnError::new)?;return Ok(());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("delete_storage_location est exécutée côté serveur"))
}

#[server]
pub async fn list_email_accounts()->Result<Vec<crate::domain::EmailAccountItem>,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let rows=sqlx::query("SELECT id,label,address,imap_host,imap_port,username,mailbox,tls,enabled,auto_scan,last_scan_at,last_error FROM email_accounts WHERE legal_entity_id=$1 ORDER BY enabled DESC,label").bind(crate::entity_scope::current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;return Ok(rows.into_iter().map(|r|crate::domain::EmailAccountItem{id:r.get("id"),label:r.get("label"),address:r.get("address"),imap_host:r.get("imap_host"),imap_port:r.get("imap_port"),username:r.get("username"),mailbox:r.get("mailbox"),tls:r.get("tls"),enabled:r.get("enabled"),auto_scan:r.get("auto_scan"),last_scan_at:r.get("last_scan_at"),last_error:r.get::<Option<String>,_>("last_error")}).collect());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("list_email_accounts est exécutée côté serveur"))
}

#[server]
pub async fn save_email_account(id:Option<Uuid>,label:String,address:String,imap_host:String,imap_port:i32,username:String,password:String,mailbox:String,tls:bool,enabled:bool,auto_scan:bool)->Result<Uuid,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;if label.trim().is_empty()||address.trim().is_empty()||imap_host.trim().is_empty()||username.trim().is_empty(){return Err(ServerFnError::new("Libellé, adresse, serveur IMAP et identifiant sont requis"));}if !(1..=65535).contains(&imap_port){return Err(ServerFnError::new("Port IMAP invalide"));}let sci=crate::entity_scope::current_legal_entity_id();let id=match id{Some(id)=>{let result=if password.trim().is_empty(){sqlx::query("UPDATE email_accounts SET label=$3,address=$4,imap_host=$5,imap_port=$6,username=$7,mailbox=$8,tls=$9,enabled=$10,auto_scan=$11,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(sci).bind(label.trim()).bind(address.trim()).bind(imap_host.trim()).bind(imap_port).bind(username.trim()).bind(mailbox.trim()).bind(tls).bind(enabled).bind(auto_scan).execute(pool).await}else{sqlx::query("UPDATE email_accounts SET label=$3,address=$4,imap_host=$5,imap_port=$6,username=$7,password=$8,mailbox=$9,tls=$10,enabled=$11,auto_scan=$12,updated_at=now(),last_error=NULL WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(sci).bind(label.trim()).bind(address.trim()).bind(imap_host.trim()).bind(imap_port).bind(username.trim()).bind(password.trim()).bind(mailbox.trim()).bind(tls).bind(enabled).bind(auto_scan).execute(pool).await};result.map_err(ServerFnError::new)?;id},None=>sqlx::query_scalar("INSERT INTO email_accounts(legal_entity_id,label,address,imap_host,imap_port,username,password,mailbox,tls,enabled,auto_scan) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id").bind(sci).bind(label.trim()).bind(address.trim()).bind(imap_host.trim()).bind(imap_port).bind(username.trim()).bind(password.trim()).bind(mailbox.trim()).bind(tls).bind(enabled).bind(auto_scan).fetch_one(pool).await.map_err(ServerFnError::new)?};return Ok(id);}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("save_email_account est exécutée côté serveur"))
}

#[server]
pub async fn delete_email_account(id:Uuid)->Result<(),ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;sqlx::query("DELETE FROM email_accounts WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(crate::entity_scope::current_legal_entity_id()).execute(pool).await.map_err(ServerFnError::new)?;return Ok(());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("delete_email_account est exécutée côté serveur"))
}

#[server]
pub async fn test_email_account(id:Uuid)->Result<String,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let cfg=load_email_account(pool,id).await.map_err(ServerFnError::new)?;tokio::task::spawn_blocking(move||test_imap(cfg)).await.map_err(|e|ServerFnError::new(e.to_string()))?.map_err(ServerFnError::new)?;return Ok("Connexion IMAP réussie.".into());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("test_email_account est exécutée côté serveur"))
}

#[server]
pub async fn scan_email_now()->Result<u64,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {return scan_email_accounts(true).await.map_err(ServerFnError::new);}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("scan_email_now est exécutée côté serveur"))
}

#[server]
pub async fn startup_email_scan()->Result<u64,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {return scan_email_accounts(false).await.map_err(ServerFnError::new);}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("startup_email_scan est exécutée côté serveur"))
}

#[server]
pub async fn list_pending_documents()->Result<Vec<crate::domain::PendingDocumentItem>,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let rows=sqlx::query("SELECT id,source_type,source_ref,filename,content_type,content_hash,staging_path,sender,subject,received_at,document_date,document_type,classification_confidence,classification_reasons,ocr_text,extracted_data,suggested_title,suggested_storage_key,COALESCE(selected_storage_location_id,'00000000-0000-0000-0000-000000000000') AS selected_storage_location_id,status,error_message FROM document_inbox WHERE legal_entity_id=$1 AND status='PENDING' ORDER BY created_at DESC").bind(crate::entity_scope::current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;return Ok(rows.into_iter().map(pending_from_row).collect());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("list_pending_documents est exécutée côté serveur"))
}

#[server]
pub async fn process_uploaded_document(mut upload: FileStream)->Result<crate::domain::PendingDocumentItem,ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {
        const MAX_UPLOAD_BYTES: usize = 30 * 1024 * 1024;
        let filename=upload.file_name().trim().to_owned();
        if filename.is_empty(){return Err(ServerFnError::new("Nom de fichier manquant"));}
        let content_type=upload.content_type().unwrap_or("application/octet-stream").to_owned();
        if !is_supported_document(&filename,&content_type){return Err(ServerFnError::new(format!("Type de fichier non pris en charge : {}",filename)));}
        if let Some(size)=upload.size(){if size>MAX_UPLOAD_BYTES as u64{return Err(ServerFnError::new("Fichier trop volumineux (maximum 30 Mo)"));}}
        let id=Uuid::new_v4();
        let staging=staging_path(id,&filename);
        if let Some(parent)=staging.parent(){tokio::fs::create_dir_all(parent).await.map_err(|e|ServerFnError::new(e.to_string()))?;}
        let mut file=tokio::fs::File::create(&staging).await.map_err(|e|ServerFnError::new(e.to_string()))?;
        let mut hasher=Sha256::new();
        let mut total=0usize;
        while let Some(chunk)=upload.next().await{
            let chunk=chunk.map_err(|e|ServerFnError::new(e.to_string()))?;
            total=total.saturating_add(chunk.len());
            if total>MAX_UPLOAD_BYTES{drop(file);let _=tokio::fs::remove_file(&staging).await;return Err(ServerFnError::new("Fichier trop volumineux (maximum 30 Mo)"));}
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|e|ServerFnError::new(e.to_string()))?;
        }
        file.flush().await.map_err(|e|ServerFnError::new(e.to_string()))?;
        drop(file);
        if total==0{let _=tokio::fs::remove_file(&staging).await;return Err(ServerFnError::new("Fichier vide"));}
        let hash=format!("{:x}",hasher.finalize());
        let source_ref=format!("manual:{}",hash);
        let pool=db().await.map_err(ServerFnError::new)?;
        if let Some(existing)=sqlx::query("SELECT id,status FROM document_inbox WHERE legal_entity_id=$1 AND source_type='MANUAL' AND source_ref=$2 AND filename=$3")
            .bind(crate::entity_scope::current_legal_entity_id()).bind(&source_ref).bind(&filename).fetch_optional(pool).await.map_err(ServerFnError::new)?
        {
            let _=tokio::fs::remove_file(&staging).await;
            let status:String=existing.get("status");
            if status=="PENDING"{return list_pending_by_id(pool,existing.get("id")).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Document en attente introuvable"));}
            return Err(ServerFnError::new(format!("Ce document a déjà été traité ({})",status)));
        }
        let bytes=tokio::fs::read(&staging).await.map_err(|e|{let _=std::fs::remove_file(&staging);ServerFnError::new(e.to_string())})?;
        let text=match extract_document_text(&filename,&content_type,&staging,&bytes).await{Ok(v)=>v,Err(e)=>{let _=tokio::fs::remove_file(&staging).await;return Err(ServerFnError::new(e))}};
        let cls=classify(ClassificationInput{document_id:id,filename:filename.clone(),text:text.clone()});
        let extracted=extract(ExtractionInput{document_id:id,document_type:cls.document_type.clone(),text:text.clone()});
        let data=serde_json::Value::Object(extracted.fields.into_iter().map(|f|(f.name,json!({"value":f.value,"confidence":f.confidence}))).collect());
        let kind=document_type_label(&cls.document_type);
        let date=extracted_document_date(&data);
        let year=date.map(|d|d.year()).unwrap_or_else(||Utc::now().year());
        let suggested_title=suggest_title(&filename,&kind,&data);
        let suggested_storage_key=smart_storage_key(&kind,year,&suggested_title,&filename);
        let reasons=json!(cls.reasons);
        if let Err(e)=sqlx::query("INSERT INTO document_inbox(id,legal_entity_id,source_type,source_ref,filename,content_type,content_hash,staging_path,sender,subject,received_at,document_date,document_type,classification_confidence,classification_reasons,ocr_text,extracted_data,suggested_title,suggested_storage_key,status) VALUES($1,$2,'MANUAL',$3,$4,$5,$6,$7,'','','',$8,$9,$10,$11,$12,$13,$14,$15,'PENDING')")
            .bind(id).bind(crate::entity_scope::current_legal_entity_id()).bind(&source_ref).bind(&filename).bind(&content_type).bind(&hash).bind(staging.to_string_lossy().to_string()).bind(date).bind(kind).bind(cls.confidence).bind(reasons).bind(text).bind(data).bind(&suggested_title).bind(&suggested_storage_key).execute(pool).await
        {let _=tokio::fs::remove_file(&staging).await;return Err(ServerFnError::new(e.to_string()));}
        return list_pending_by_id(pool,id).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Pièce mise en attente mais impossible à relire"));
    }
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))]
    Err(ServerFnError::new("process_uploaded_document est exécutée côté serveur"))
}

#[server]
pub async fn approve_pending_document(id:Uuid,category:String,title:String,storage_key:String,storage_location_id:Option<Uuid>)->Result<(),ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let row=list_pending_by_id(pool,id).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Pièce introuvable ou déjà traitée"))?;let location_id=storage_location_id.ok_or_else(||ServerFnError::new("Choisissez un emplacement de stockage"))?;let root:Option<String>=sqlx::query_scalar("SELECT configuration->>'root_path' FROM storage_locations WHERE id=$1 AND legal_entity_id=$2 AND is_active").bind(location_id).bind(crate::entity_scope::current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?;let root=root.ok_or_else(||ServerFnError::new("Emplacement de stockage inactif ou introuvable"))?;let relative=sanitize_relative_path(&storage_key);if relative.as_os_str().is_empty(){return Err(ServerFnError::new("Chemin de classement invalide"));}let destination=PathBuf::from(root).join(&relative);if let Some(parent)=destination.parent(){tokio::fs::create_dir_all(parent).await.map_err(|e|ServerFnError::new(e.to_string()))?;}move_file(Path::new(&row.staging_path),&destination).await.map_err(ServerFnError::new)?;let data=row.extracted_data.clone();let document_id=sqlx::query_scalar("INSERT INTO documents(legal_entity_id,category,title,file_name,storage_key,content_hash,document_date,extracted_data,storage_location_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id").bind(crate::entity_scope::current_legal_entity_id()).bind(category.trim()).bind(title.trim()).bind(&row.filename).bind(relative.to_string_lossy().to_string()).bind(&row.content_hash).bind(row.document_date).bind(data).bind(location_id).fetch_one(pool).await.map_err(ServerFnError::new)?;sqlx::query("UPDATE document_inbox SET status='APPROVED',updated_at=now(),selected_storage_location_id=$3,error_message=NULL WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(crate::entity_scope::current_legal_entity_id()).bind(location_id).execute(pool).await.map_err(ServerFnError::new)?;audit_intelligence(pool,"APPROVE_DOCUMENT",Some("document"),Some(document_id),json!({"inbox_id":id,"storage_key":relative.to_string_lossy()})).await.map_err(ServerFnError::new)?;return Ok(());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("approve_pending_document est exécutée côté serveur"))
}

#[server]
pub async fn reject_pending_document(id:Uuid,reason:String)->Result<(),ServerFnError>{
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    {let pool=db().await.map_err(ServerFnError::new)?;let result=sqlx::query("UPDATE document_inbox SET status='REJECTED',updated_at=now(),error_message=$3 WHERE id=$1 AND legal_entity_id=$2 AND status='PENDING'").bind(id).bind(crate::entity_scope::current_legal_entity_id()).bind(reason.trim()).execute(pool).await.map_err(ServerFnError::new)?;if result.rows_affected()==0{return Err(ServerFnError::new("Pièce introuvable ou déjà traitée"));}if let Some(path)=sqlx::query_scalar::<_,String>("SELECT staging_path FROM document_inbox WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(crate::entity_scope::current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?{let _=tokio::fs::remove_file(path).await;}return Ok(());}
    #[cfg(any(not(feature="server"), target_arch = "wasm32"))] Err(ServerFnError::new("reject_pending_document est exécutée côté serveur"))
}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
async fn queue_document_bytes(source_type:&str,source_ref:String,filename:String,content_type:String,bytes:Vec<u8>,sender:String,subject:String,received_at:String)->Result<crate::domain::PendingDocumentItem,String>{
    if !is_supported_document(&filename,&content_type){return Err(format!("Type de fichier non pris en charge : {}",filename));}
    let pool=db().await.map_err(|e|e.to_string())?;let hash=hex_hash(&bytes);let sci=crate::entity_scope::current_legal_entity_id();
    if let Some(existing)=sqlx::query("SELECT id,status FROM document_inbox WHERE legal_entity_id=$1 AND source_type=$2 AND source_ref=$3 AND filename=$4").bind(sci).bind(source_type).bind(&source_ref).bind(&filename).fetch_optional(pool).await.map_err(|e|e.to_string())?{let status:String=existing.get("status");if status=="PENDING"{return list_pending_by_id(pool,existing.get("id")).await.map_err(|e|e.to_string())?.ok_or_else(||"Document en attente introuvable".into());}return Err(format!("Ce document a déjà été traité ({})",status));}
    let id=Uuid::new_v4();let staging=staging_path(id,&filename);if let Some(parent)=staging.parent(){tokio::fs::create_dir_all(parent).await.map_err(|e|e.to_string())?;}tokio::fs::write(&staging,&bytes).await.map_err(|e|e.to_string())?;
    let text=match extract_document_text(&filename,&content_type,&staging,&bytes).await{Ok(v)=>v,Err(e)=>{let _=tokio::fs::remove_file(&staging).await;return Err(e)}};
    let cls=classify(ClassificationInput{document_id:id,filename:filename.clone(),text:text.clone()});let extracted=extract(ExtractionInput{document_id:id,document_type:cls.document_type.clone(),text:text.clone()});let data=serde_json::Value::Object(extracted.fields.into_iter().map(|f|(f.name,json!({"value":f.value,"confidence":f.confidence}))).collect());let kind=document_type_label(&cls.document_type);let date=extracted_document_date(&data);let year=date.map(|d|d.year()).unwrap_or_else(||Utc::now().year());let suggested_title=suggest_title(&filename,&kind,&data);let suggested_storage_key=smart_storage_key(&kind,year,&suggested_title,&filename);let reasons=json!(cls.reasons);sqlx::query("INSERT INTO document_inbox(id,legal_entity_id,source_type,source_ref,filename,content_type,content_hash,staging_path,sender,subject,received_at,document_date,document_type,classification_confidence,classification_reasons,ocr_text,extracted_data,suggested_title,suggested_storage_key,status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,'PENDING')").bind(id).bind(sci).bind(source_type).bind(&source_ref).bind(&filename).bind(&content_type).bind(hash).bind(staging.to_string_lossy().to_string()).bind(sender).bind(subject).bind(received_at).bind(date).bind(kind).bind(cls.confidence).bind(reasons).bind(text).bind(data).bind(&suggested_title).bind(&suggested_storage_key).execute(pool).await.map_err(|e|e.to_string())?;list_pending_by_id(pool,id).await.map_err(|e|e.to_string())?.ok_or_else(||"Pièce mise en attente mais impossible à relire".into())
}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn pending_from_row(r:sqlx::postgres::PgRow)->crate::domain::PendingDocumentItem{crate::domain::PendingDocumentItem{id:r.get("id"),source_type:r.get("source_type"),source_ref:r.get("source_ref"),filename:r.get("filename"),content_type:r.get("content_type"),content_hash:r.get("content_hash"),staging_path:r.get("staging_path"),sender:r.get("sender"),subject:r.get("subject"),received_at:r.get("received_at"),document_date:r.get("document_date"),document_type:r.get("document_type"),classification_confidence:r.get("classification_confidence"),classification_reasons:r.get("classification_reasons"),ocr_text:r.get("ocr_text"),extracted_data:r.get("extracted_data"),suggested_title:r.get("suggested_title"),suggested_storage_key:r.get("suggested_storage_key"),selected_storage_location_id:r.get("selected_storage_location_id"),status:r.get("status"),error_message:r.get("error_message")}}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
async fn list_pending_by_id(pool:&sqlx::PgPool,id:Uuid)->Result<Option<crate::domain::PendingDocumentItem>,sqlx::Error>{let row=sqlx::query("SELECT id,source_type,source_ref,filename,content_type,content_hash,staging_path,sender,subject,received_at,document_date,document_type,classification_confidence,classification_reasons,ocr_text,extracted_data,suggested_title,suggested_storage_key,COALESCE(selected_storage_location_id,'00000000-0000-0000-0000-000000000000') AS selected_storage_location_id,status,error_message FROM document_inbox WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(crate::entity_scope::current_legal_entity_id()).fetch_optional(pool).await?;Ok(row.map(pending_from_row))}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
async fn extract_document_text(filename:&str,content_type:&str,staging_path:&Path,bytes:&[u8])->Result<String,String>{
    let ext=Path::new(filename).extension().and_then(|v|v.to_str()).unwrap_or("").to_lowercase();
    if ext=="txt"||content_type.starts_with("text/"){return String::from_utf8(bytes.to_vec()).map_err(|_|"Texte non UTF-8".to_string());}
    if ext=="pdf"||content_type.eq_ignore_ascii_case("application/pdf"){let doc=lopdf::Document::load_mem(bytes).map_err(|e|format!("PDF illisible : {}",e))?;let pages:Vec<u32>=doc.get_pages().keys().copied().collect();if pages.is_empty(){return Ok(String::new());}return doc.extract_text(&pages).map_err(|e|format!("Extraction PDF impossible : {}",e));}
    if is_ocr_image(filename,content_type){let engine=OCR_ENGINE.get_or_try_init(||async{let o=ocr::api::Ocr::new().map_err(|e|e.to_string())?;o.initialize().await.map_err(|e|e.to_string())?;Ok::<ocr::api::Ocr,String>(o)}).await.map_err(|e|e.to_string())?;let result=engine.recognize_text_from_file(staging_path.to_string_lossy().as_ref()).await.map_err(|e|e.to_string())?;return Ok(result.text);}
    Err(format!("Format non supporté pour analyse : {}",filename))
}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn is_ocr_image(filename:&str,content_type:&str)->bool{let e=Path::new(filename).extension().and_then(|v|v.to_str()).unwrap_or("").to_lowercase();["png","jpg","jpeg","tiff","tif","bmp","gif","webp"].contains(&e.as_str())||content_type.starts_with("image/")}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
async fn scan_email_accounts(force_all:bool)->Result<u64,String>{let pool=db().await.map_err(|e|e.to_string())?;let rows=sqlx::query("SELECT id FROM email_accounts WHERE legal_entity_id=$1 AND enabled AND ($2 OR auto_scan)").bind(crate::entity_scope::current_legal_entity_id()).bind(force_all).fetch_all(pool).await.map_err(|e|e.to_string())?;let mut total=0;for r in rows{let id:Uuid=r.get("id");match load_email_account(pool,id).await{Ok(cfg)=>{let cfg_id=cfg.id;let result=tokio::task::spawn_blocking(move||fetch_email_attachments(cfg)).await.map_err(|e|e.to_string())?;match result{Ok(items)=>{let mut added=0;for item in items{match queue_document_bytes("EMAIL",item.source_ref,item.filename,item.content_type,item.bytes,item.sender,item.subject,item.received_at).await{Ok(_)=>{added+=1;total+=1},Err(e) if e.contains("déjà été traité")||e.contains("already")=>{},Err(e)=>{observability::record_warning("email attachment rejected", LogDomain::Ocr, &e)}}}sqlx::query("UPDATE email_accounts SET last_scan_at=now(),last_error=NULL,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(cfg_id).bind(crate::entity_scope::current_legal_entity_id()).execute(pool).await.map_err(|e|e.to_string())?;tracing::info!(domain=%LogDomain::Ocr, event="email_scan_completed", account_id=%cfg_id, added);},Err(e)=>{observability::record_error(LogDomain::Ocr,&e,"email attachment fetch failed");sqlx::query("UPDATE email_accounts SET last_error=$3,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(cfg_id).bind(crate::entity_scope::current_legal_entity_id()).bind(&e).execute(pool).await.map_err(|x|x.to_string())?;}}},Err(e)=>{observability::record_warning("email account load failed", LogDomain::Ocr, &e);}}}Ok(total)}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
async fn load_email_account(pool:&sqlx::PgPool,id:Uuid)->Result<EmailAccountConfig,String>{let r=sqlx::query("SELECT id,label,address,imap_host,imap_port,username,password,mailbox,tls FROM email_accounts WHERE id=$1 AND legal_entity_id=$2 AND enabled").bind(id).bind(crate::entity_scope::current_legal_entity_id()).fetch_optional(pool).await.map_err(|e|e.to_string())?.ok_or_else(||"Compte e-mail introuvable ou désactivé".to_string())?;Ok(EmailAccountConfig{id:r.get("id"),label:r.get("label"),address:r.get("address"),imap_host:r.get("imap_host"),imap_port:r.get("imap_port"),username:r.get("username"),password:r.get("password"),mailbox:r.get("mailbox"),tls:r.get("tls")})}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn test_imap(cfg:EmailAccountConfig)->Result<(),String>{let tls=native_tls::TlsConnector::builder().build().map_err(|e|e.to_string())?;let client=imap::connect((cfg.imap_host.as_str(),cfg.imap_port as u16),cfg.imap_host.as_str(),&tls).map_err(|e|e.to_string())?;let mut session=client.login(cfg.username,cfg.password).map_err(|e|e.0.to_string())?;session.select(cfg.mailbox).map_err(|e|e.to_string())?;session.logout().map_err(|e|e.to_string())?;Ok(())}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn fetch_email_attachments(cfg:EmailAccountConfig)->Result<Vec<EmailAttachmentPayload>,String>{let tls=native_tls::TlsConnector::builder().build().map_err(|e|e.to_string())?;let client=imap::connect((cfg.imap_host.as_str(),cfg.imap_port as u16),cfg.imap_host.as_str(),&tls).map_err(|e|e.to_string())?;let mut session=client.login(cfg.username,cfg.password).map_err(|e|e.0.to_string())?;session.select(cfg.mailbox.clone()).map_err(|e|e.to_string())?;let uids=session.search("UNSEEN").map_err(|e|e.to_string())?;let mut output=Vec::new();for seq in uids{let fetches=session.fetch(seq.to_string(),"RFC822").map_err(|e|e.to_string())?;for fetch in fetches.iter(){let raw=fetch.body().ok_or_else(||"Message MIME vide".to_string())?;let parsed=MessageParser::default().parse(raw).ok_or_else(||"Message MIME invalide".to_string())?;let sender=parsed.return_address().unwrap_or("").to_string();let subject=parsed.subject().unwrap_or("").to_string();let received_at=parsed.date().map(mail_date_to_rfc3339).unwrap_or_else(||Utc::now().to_rfc3339());let source_message_id=parsed.message_id().unwrap_or("").to_string();for (i,part) in parsed.attachments().enumerate(){let fallback=format!("piece-{}.bin",i);let filename=part.attachment_name().unwrap_or(fallback.as_str()).to_string();if !is_supported_document(&filename,""){continue;}let bytes=part.contents().to_vec();let content_type=part.content_type().map(|ct|{format!("{}/{}",ct.ctype(),ct.subtype().unwrap_or("octet-stream"))}).unwrap_or_else(||"application/octet-stream".into());let source_ref=if source_message_id.is_empty(){format!("{}:{}:{}",cfg.address,seq,i)}else{format!("{}:{}",cfg.address,source_message_id)};output.push(EmailAttachmentPayload{source_ref,filename,content_type,bytes,sender:sender.clone(),subject:subject.clone(),received_at:received_at.clone()});}}}session.logout().map_err(|e|e.to_string())?;Ok(output)}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn mail_date_to_rfc3339(d:&mail_parser::DateTime)->String{let offset_seconds=if d.tz_before_gmt{-((d.tz_hour as i32)*3600+(d.tz_minute as i32)*60)}else{(d.tz_hour as i32)*3600+(d.tz_minute as i32)*60};let fixed=chrono::FixedOffset::east_opt(offset_seconds).unwrap_or(chrono::FixedOffset::east_opt(0).unwrap());match fixed.with_ymd_and_hms(d.year as i32,d.month as u32,d.day as u32,d.hour as u32,d.minute as u32,d.second as u32){chrono::LocalResult::Single(dt)=>dt.to_rfc3339(),_=>Utc::now().to_rfc3339()}}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
async fn assistant_context(pool:&sqlx::PgPool)->Result<String,sqlx::Error>{
    let sci=crate::entity_scope::current_legal_entity_id();
    let profile=sqlx::query("SELECT legal_name,registered_office FROM scis WHERE id=$1").bind(sci).fetch_optional(pool).await?;
    let unpaid:i64=sqlx::query_scalar("SELECT COUNT(*) FROM invoices WHERE legal_entity_id=$1 AND status<>'BROUILLON' AND paid_cents<gross_cents").bind(sci).fetch_one(pool).await?;
    let overdue:i64=sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE legal_entity_id=$1 AND due_at<now() AND state NOT IN ('DONE','SKIPPED')").bind(sci).fetch_one(pool).await?;
    let unmatched:i64=sqlx::query_scalar("SELECT COUNT(*) FROM bank_transactions WHERE legal_entity_id=$1 AND reconciliation_status='UNMATCHED'").bind(sci).fetch_one(pool).await?;
    let pending:i64=sqlx::query_scalar("SELECT COUNT(*) FROM document_inbox WHERE legal_entity_id=$1 AND status='PENDING'").bind(sci).fetch_one(pool).await?;
    let upcoming=sqlx::query("SELECT title,due_at,state FROM tasks WHERE legal_entity_id=$1 AND state NOT IN ('DONE','SKIPPED') ORDER BY due_at LIMIT 8").bind(sci).fetch_all(pool).await?;
    let mut out=String::new();
    if let Some(row)=profile{let name:String=row.get("legal_name");let office:String=row.get::<Option<String>,_>("registered_office").unwrap_or_default();out.push_str(&format!("SCI: {} — {}\n",name,office));}
    out.push_str(&format!("Factures ouvertes: {}\nTâches en retard: {}\nMouvements bancaires non rapprochés: {}\nDocuments en attente de validation: {}\n",unpaid,overdue,unmatched,pending));
    for row in upcoming{let title:String=row.get("title");let due:DateTime<Utc>=row.get("due_at");let state:String=row.get("state");out.push_str(&format!("Prochaine tâche: {} — {} — {}\n",due.to_rfc3339(),title,state));}
    Ok(out)
}

#[cfg(any(not(feature="server"), target_arch = "wasm32"))]
async fn browser_voice_capture_impl()->Result<String,String>{let mut eval=document::eval(r#"const SR=window.SpeechRecognition||window.webkitSpeechRecognition;if(!SR){dioxus.send('__ERROR__:La dictée vocale n’est pas disponible dans ce navigateur.');}else{const r=new SR();r.lang='fr-FR';r.interimResults=false;r.maxAlternatives=1;r.onresult=e=>dioxus.send(e.results[0][0].transcript);r.onerror=e=>dioxus.send('__ERROR__:Dictée vocale: '+(e.error||'erreur'));r.onend=()=>{};r.start();}"#);let value:String=eval.recv().await.map_err(|e|e.to_string())?;if let Some(msg)=value.strip_prefix("__ERROR__:"){Err(msg.to_string())}else{Ok(value)}}


#[cfg(any(not(feature="server"), target_arch = "wasm32"))]
pub async fn browser_voice_capture()->Result<String,String>{browser_voice_capture_impl().await}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
pub async fn browser_voice_capture()->Result<String,String>{Err("La dictée vocale doit être déclenchée dans le navigateur".into())}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn fallback_information(message:&str,context:&str)->String{let n=message.to_lowercase();if n.contains("impay")||n.contains("créance"){return format!("Voici le contexte actuel :\n{}",context);}if n.contains("document")||n.contains("pièce"){return format!("La file contient les éléments détectés suivants :\n{}",context);}if n.contains("banque")||n.contains("rapproch"){return format!("Contexte bancaire :\n{}",context);}if n.contains("tva"){return format!("Situation disponible dans le contexte :\n{}",context);}format!("Je n'ai pas de réponse LLM disponible, mais voici le contexte métier accessible :\n{}",context)}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn derive_proposal(message:&str)->(String,serde_json::Value){let lower=message.to_lowercase();let title=if lower.contains("mail")||lower.contains("email"){format!("Revoir la file documentaire issue des e-mails") }else if lower.contains("loyer"){format!("Contrôler et préparer les loyers demandés") }else if lower.contains("banque")||lower.contains("rapproch"){format!("Revoir les mouvements bancaires non rapprochés") }else{format!("Traiter la demande de l’assistant")};("CREATE_TASK".into(),json!({"title":title,"description":message.trim()}))}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
async fn call_llm(base_url:&str,model:&str,api_key:Option<&str>,prompt:&str,system:&str)->Result<String,String>{let body=LlmRequestBody{model:model.to_owned(),messages:vec![LlmMessage{role:"system".into(),content:system.into()},LlmMessage{role:"user".into(),content:prompt.into()}],temperature:0.1};let client=reqwest::Client::new();let mut request=client.post(format!("{}/chat/completions",base_url.trim_end_matches('/')).as_str()).json(&body);if let Some(key)=api_key.filter(|v|!v.trim().is_empty()){request=request.bearer_auth(key);}let response=request.timeout(std::time::Duration::from_secs(60)).send().await.map_err(|e|e.to_string())?;if !response.status().is_success(){return Err(format!("LLM HTTP {}",response.status()));}let value:serde_json::Value=response.json().await.map_err(|e|e.to_string())?;value.get("choices").and_then(|v|v.get(0)).and_then(|v|v.get("message")).and_then(|v|v.get("content")).and_then(|v|v.as_str()).map(str::to_owned).ok_or_else(||"Réponse LLM invalide".into())}

#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn staging_path(id:Uuid,filename:&str)->PathBuf{let root=std::env::var("SCI_DOCUMENTS_INBOX_ROOT").unwrap_or_else(|_|".sci-inbox".into());PathBuf::from(root).join(crate::entity_scope::current_legal_entity_id().to_string()).join("pending").join(format!("{}_{}",id,sanitize_filename(filename)))}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn hex_hash(bytes:&[u8])->String{let mut h=Sha256::new();h.update(bytes);format!("{:x}",h.finalize())}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn is_supported_document(filename:&str,content_type:&str)->bool{let e=Path::new(filename).extension().and_then(|v|v.to_str()).unwrap_or("").to_lowercase();["pdf","png","jpg","jpeg","tif","tiff","bmp","gif","webp","txt","csv"].contains(&e.as_str())||matches!(content_type,"application/pdf")||content_type.starts_with("image/")||content_type.starts_with("text/")}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn document_type_label(k:&DocumentType)->String{match k{DocumentType::InvoiceSupplier=>"FACTURE_FOURNISSEUR",DocumentType::RentInvoice=>"FACTURE_LOYER",DocumentType::BankStatement=>"RELEVE_BANCAIRE",DocumentType::PaymentProof=>"PREUVE_PAIEMENT",DocumentType::Lease=>"BAIL",DocumentType::LeaseAmendment=>"AVENANT",DocumentType::TaxDocument=>"DOCUMENT_FISCAL",DocumentType::Insurance=>"ASSURANCE",DocumentType::Administrative=>"ADMINISTRATIF",DocumentType::Correspondence=>"COURRIER",DocumentType::SupportingDocument=>"JUSTIFICATIF",DocumentType::Unknown=>"A_CLASSER"}.into()}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn suggest_title(filename:&str,kind:&str,extracted:&serde_json::Value)->String{if let Some(s)=extracted.get("supplier").and_then(|v|v.get("value")).and_then(|v|v.as_str()).filter(|v|!v.trim().is_empty()){return format!("{} — {}",kind,s.trim());}let stem=Path::new(filename).file_stem().and_then(|v|v.to_str()).unwrap_or(filename);format!("{} — {}",kind,stem)}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn smart_storage_key(kind:&str,year:i32,title:&str,filename:&str)->String{let folder=match kind{"FACTURE_FOURNISSEUR"=>"Comptabilite/Factures-fournisseurs","FACTURE_LOYER"=>"Locations/Factures-loyers","RELEVE_BANCAIRE"=>"Banque/Releves","BAIL"=>"Locations/Baux","PREUVE_PAIEMENT"=>"Finance/Preuves-paiement","DOCUMENT_FISCAL"=>"Fiscalite/Impots","ASSURANCE"=>"Assurances","ADMINISTRATIF"=>"Administratif",_=>"A-classer"};format!("{}/{}/{}",folder,year,sanitize_filename_with_title(title,filename))}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn sanitize_filename_with_title(title:&str,filename:&str)->String{let ext=Path::new(filename).extension().and_then(|v|v.to_str()).map(|e|format!(".{}",e)).unwrap_or_default();let mut base=sanitize_filename(title);if base.len()>120{base.truncate(120);}if !ext.is_empty()&&!base.to_lowercase().ends_with(&ext.to_lowercase()){base.push_str(&ext);}base}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn sanitize_filename(name:&str)->String{name.chars().map(|c|if c.is_ascii_alphanumeric()||matches!(c,'-'|'_'|'.'){c}else{'-'}).collect::<String>().trim_matches('-').trim_matches('.').to_string()}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn sanitize_relative_path(path:&str)->PathBuf{path.split(['\\','/']).filter(|p|!p.is_empty()&&*p!="."&&*p!="..").map(sanitize_filename).filter(|p|!p.is_empty()).fold(PathBuf::new(),|mut a,p|{a.push(p);a})}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
async fn move_file(from:&Path,to:&Path)->Result<(),String>{if from==to{return Ok(());}match tokio::fs::rename(from,to).await{Ok(())=>Ok(()),Err(_)=>{tokio::fs::copy(from,to).await.map_err(|e|e.to_string())?;tokio::fs::remove_file(from).await.map_err(|e|e.to_string())?;Ok(())}}}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
fn extracted_document_date(data:&serde_json::Value)->Option<NaiveDate>{for key in ["invoice_date","statement_date","payment_date","start_date","tax_date","expiry_date"]{if let Some(s)=data.get(key).and_then(|v|v.get("value")).and_then(|v|v.as_str()){if let Ok(d)=NaiveDate::parse_from_str(s,"%Y-%m-%d"){return Some(d);}}}None}
#[cfg(all(feature="server", not(target_arch = "wasm32")))]
async fn audit_intelligence(pool:&sqlx::PgPool,action:&str,entity_type:Option<&str>,entity_id:Option<Uuid>,payload:serde_json::Value)->Result<(),sqlx::Error>{sqlx::query("INSERT INTO audit_events(legal_entity_id,actor,action,entity_type,entity_id,payload) VALUES($1,'USER',$2,$3,$4,$5)").bind(crate::entity_scope::current_legal_entity_id()).bind(action).bind(entity_type).bind(entity_id).bind(payload).execute(pool).await.map(|_|())}

#[cfg(test)]
mod tests{
    use super::*;
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    #[test]fn sanitize_paths(){let p=sanitize_relative_path("../..\\Comptabilite/2026/Facture Fournisseur.pdf");assert!(!p.to_string_lossy().contains(".."));}
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    #[test]fn storage_routing(){assert!(smart_storage_key("FACTURE_FOURNISSEUR",2026,"ACME — facture 12","doc.pdf").starts_with("Comptabilite/Factures-fournisseurs/2026/"));}
    #[cfg(all(feature="server", not(target_arch = "wasm32")))]
    #[test]fn supported_document(){assert!(is_supported_document("a.pdf","application/pdf"));assert!(is_supported_document("a.jpg","image/jpeg"));assert!(!is_supported_document("a.exe","application/octet-stream"));}
}
