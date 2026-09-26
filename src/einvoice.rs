use crate::entity_scope::current_legal_entity_id;
use crate::ui::{FormField, ModuleHeader};
use chrono::{Datelike, NaiveDate, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EInvoiceAdapterKind {
    Pdp,
    Local,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EInvoiceEnvelope {
    pub provider_code: String,
    pub format_code: String,
    pub external_reference: String,
    pub payload: serde_json::Value,
}

/// Port métier e-facturation. Le cœur métier ne connaît que ce contrat.
pub trait EInvoiceProvider {
    fn provider_code(&self) -> &str;
    fn provider_name(&self) -> &str;
    fn adapter_kind(&self) -> EInvoiceAdapterKind;
    fn prepare(&self, invoice: &serde_json::Value) -> Result<EInvoiceEnvelope, String>;
}

#[derive(Debug, Clone)]
pub struct PdpAdapter {
    pub code: String,
    pub name: String,
    pub format: String,
}

impl EInvoiceProvider for PdpAdapter {
    fn provider_code(&self) -> &str { &self.code }
    fn provider_name(&self) -> &str { &self.name }
    fn adapter_kind(&self) -> EInvoiceAdapterKind { EInvoiceAdapterKind::Pdp }
    fn prepare(&self, invoice: &serde_json::Value) -> Result<EInvoiceEnvelope, String> {
        let reference = invoice.get("invoice_number").and_then(|v| v.as_str()).unwrap_or("FACTURE");
        Ok(EInvoiceEnvelope {
            provider_code: self.code.clone(),
            format_code: self.format.clone(),
            external_reference: format!("SCI-{}", reference),
            payload: json!({
                "specification": "e-invoice-provider-neutral-v1",
                "invoice": invoice,
                "transport": "ADAPTER_ONLY",
            }),
        })
    }
}

#[derive(Debug, Clone)]
pub struct LocalAdapter {
    pub code: String,
    pub name: String,
    pub format: String,
}

impl EInvoiceProvider for LocalAdapter {
    fn provider_code(&self) -> &str { &self.code }
    fn provider_name(&self) -> &str { &self.name }
    fn adapter_kind(&self) -> EInvoiceAdapterKind { EInvoiceAdapterKind::Local }
    fn prepare(&self, invoice: &serde_json::Value) -> Result<EInvoiceEnvelope, String> {
        let reference = invoice.get("invoice_number").and_then(|v| v.as_str()).unwrap_or("FACTURE");
        Ok(EInvoiceEnvelope {
            provider_code: self.code.clone(),
            format_code: self.format.clone(),
            external_reference: format!("LOCAL-{}", reference),
            payload: json!({"invoice": invoice, "transport": "LOCAL_ADAPTER_ONLY"}),
        })
    }
}

fn fingerprint_json(value: &serde_json::Value) -> String {
    let mut hasher = DefaultHasher::new();
    value.to_string().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EInvoiceProviderItem {
    pub id: Uuid,
    pub provider_code: String,
    pub provider_name: String,
    pub adapter_kind: String,
    pub format_code: String,
    pub endpoint_reference: String,
    pub credential_reference: String,
    pub active: bool,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EInvoiceEventItem {
    pub id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub provider_code: String,
    pub direction: String,
    pub event_type: String,
    pub external_id: String,
    pub status_code: String,
    pub error_message: String,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EReportingResult {
    pub id: Uuid,
    pub provider_code: String,
    pub report_type: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub invoice_count: i64,
    pub payment_count: i64,
    pub transaction_net_cents: i64,
    pub payment_cents: i64,
    pub status: String,
}

fn adapter_for(code: &str, name: &str, kind: &str, format: &str) -> Box<dyn EInvoiceProvider> {
    match kind {
        "LOCAL_ADAPTER" => Box::new(LocalAdapter { code: code.into(), name: name.into(), format: format.into() }),
        "DISABLED" => Box::new(LocalAdapter { code: code.into(), name: name.into(), format: format.into() }),
        _ => Box::new(PdpAdapter { code: code.into(), name: name.into(), format: format.into() }),
    }
}

#[server]
pub async fn list_einvoice_providers() -> Result<Vec<EInvoiceProviderItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query("SELECT id,provider_code,provider_name,adapter_kind,format_code,endpoint_reference,credential_reference,active,effective_from,effective_to FROM einvoice_providers WHERE legal_entity_id=$1 ORDER BY active DESC,effective_from DESC,provider_code")
            .bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row;
        Ok(rows.into_iter().map(|r| EInvoiceProviderItem {
            id: r.get("id"), provider_code: r.get("provider_code"), provider_name: r.get("provider_name"), adapter_kind: r.get("adapter_kind"), format_code: r.get("format_code"), endpoint_reference: r.get("endpoint_reference"), credential_reference: r.get("credential_reference"), active: r.get("active"), effective_from: r.get("effective_from"), effective_to: r.get("effective_to"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_einvoice_providers est exécutée côté serveur"))
}

#[server]
pub async fn activate_einvoice_provider(provider_code: String, provider_name: String, adapter_kind: String, format_code: String, endpoint_reference: String, credential_reference: String, effective_from: NaiveDate) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let code = provider_code.trim().to_uppercase();
        let name = provider_name.trim();
        let kind = adapter_kind.trim().to_uppercase();
        let format = format_code.trim().to_uppercase();
        if code.is_empty() || name.is_empty() { return Err(ServerFnError::new("Code et nom de plateforme requis")); }
        if !matches!(kind.as_str(), "PDP_ADAPTER"|"LOCAL_ADAPTER"|"DISABLED") { return Err(ServerFnError::new("Type d'adapter invalide")); }
        if format.is_empty() { return Err(ServerFnError::new("Format e-facture requis")); }
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        sqlx::query("UPDATE einvoice_providers SET active=false,effective_to=CASE WHEN effective_to IS NULL OR effective_to >= $2 THEN $2 - INTERVAL '1 day' ELSE effective_to END,updated_at=now() WHERE legal_entity_id=$1 AND active=true")
            .bind(entity).bind(effective_from).execute(pool).await.map_err(ServerFnError::new)?;
        let id: Uuid = sqlx::query_scalar("INSERT INTO einvoice_providers(legal_entity_id,provider_code,provider_name,adapter_kind,format_code,endpoint_reference,credential_reference,capabilities,active,effective_from) VALUES($1,$2,$3,$4,$5,$6,$7,$8,true,$9) ON CONFLICT(legal_entity_id,provider_code,effective_from) DO UPDATE SET provider_name=EXCLUDED.provider_name,adapter_kind=EXCLUDED.adapter_kind,format_code=EXCLUDED.format_code,endpoint_reference=EXCLUDED.endpoint_reference,credential_reference=EXCLUDED.credential_reference,capabilities=EXCLUDED.capabilities,active=true,effective_to=NULL,updated_at=now() RETURNING id")
            .bind(entity).bind(&code).bind(name).bind(&kind).bind(&format).bind(endpoint_reference.trim()).bind(credential_reference.trim()).bind(json!({"receiving":true,"issuing":true,"ereporting":true})).bind(effective_from).fetch_one(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO audit_events(sci_id,legal_entity_id,actor,action,entity_type,entity_id,payload) VALUES(NULL,$1,'MANAGER','EINVOICE_PROVIDER_SWITCH','EINVOICE_PROVIDER',$2,$3)")
            .bind(entity).bind(id).bind(json!({"provider_code":code,"provider_name":name,"effective_from":effective_from,"adapter_kind":kind})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _=(provider_code,provider_name,adapter_kind,format_code,endpoint_reference,credential_reference,effective_from); Err(ServerFnError::new("activate_einvoice_provider est exécutée côté serveur")) }
}

#[server]
pub async fn list_einvoice_events(limit: i32) -> Result<Vec<EInvoiceEventItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query("SELECT id,invoice_id,provider_code,direction,event_type,external_id,status_code,error_message,occurred_at FROM einvoice_events WHERE legal_entity_id=$1 ORDER BY occurred_at DESC,id DESC LIMIT $2")
            .bind(current_legal_entity_id()).bind(limit.clamp(1,200)).fetch_all(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row;
        Ok(rows.into_iter().map(|r| EInvoiceEventItem { id:r.get("id"),invoice_id:r.get("invoice_id"),provider_code:r.get("provider_code"),direction:r.get("direction"),event_type:r.get("event_type"),external_id:r.get("external_id"),status_code:r.get("status_code"),error_message:r.get("error_message"),occurred_at:r.get("occurred_at") }).collect())
    }
    #[cfg(not(feature = "server"))]
    { let _=limit; Err(ServerFnError::new("list_einvoice_events est exécutée côté serveur")) }
}

#[server]
pub async fn prepare_einvoice(invoice_id: Uuid) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let provider = sqlx::query("SELECT provider_code,provider_name,adapter_kind,format_code FROM einvoice_providers WHERE legal_entity_id=$1 AND active=true AND effective_from<=CURRENT_DATE AND (effective_to IS NULL OR effective_to>=CURRENT_DATE) ORDER BY effective_from DESC LIMIT 1")
            .bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Aucune plateforme e-facture active"))?;
        use sqlx::Row;
        let pcode:String=provider.get("provider_code"); let pname:String=provider.get("provider_name"); let pkind:String=provider.get("adapter_kind"); let pformat:String=provider.get("format_code");
        let invoice = sqlx::query("SELECT i.invoice_number,i.issue_date,i.due_date,i.net_cents,i.vat_cents,i.gross_cents,i.status,i.document_kind,COALESCE(t.legal_name,'') AS tenant_name,COALESCE(t.siret,'') AS tenant_siret FROM invoices i LEFT JOIN leases l ON l.id=i.lease_id LEFT JOIN tenants t ON t.id=l.tenant_id WHERE i.id=$1 AND i.legal_entity_id=$2")
            .bind(invoice_id).bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Facture introuvable"))?;
        let status:String=invoice.get("status");
        if status=="DRAFT" { return Err(ServerFnError::new("Un brouillon ne peut pas être préparé pour émission e-facture")); }
        if !matches!(status.as_str(),"VALIDATED"|"ISSUED"|"PAID_PARTIAL"|"PAID"|"OVERDUE") { return Err(ServerFnError::new("État de facture non transmissible")); }
        let lines=sqlx::query("SELECT line_no,description,quantity,net_cents,vat_rate_bp,vat_cents,gross_cents FROM billing_invoice_lines WHERE legal_entity_id=$1 AND invoice_id=$2 ORDER BY line_no")
            .bind(entity).bind(invoice_id).fetch_all(pool).await.map_err(ServerFnError::new)?;
        let line_json=lines.into_iter().map(|r|json!({"line_no":r.get::<i32,_>("line_no"),"description":r.get::<String,_>("description"),"quantity":r.get::<rust_decimal::Decimal,_>("quantity"),"net_cents":r.get::<i64,_>("net_cents"),"vat_rate_bp":r.get::<i32,_>("vat_rate_bp"),"vat_cents":r.get::<i64,_>("vat_cents"),"gross_cents":r.get::<i64,_>("gross_cents")})).collect::<Vec<_>>();
        let invoice_payload=json!({"invoice_number":invoice.get::<String,_>("invoice_number"),"issue_date":invoice.get::<NaiveDate,_>("issue_date"),"due_date":invoice.get::<NaiveDate,_>("due_date"),"net_cents":invoice.get::<i64,_>("net_cents"),"vat_cents":invoice.get::<i64,_>("vat_cents"),"gross_cents":invoice.get::<i64,_>("gross_cents"),"document_kind":invoice.get::<String,_>("document_kind"),"tenant_name":invoice.get::<String,_>("tenant_name"),"tenant_siret":invoice.get::<String,_>("tenant_siret"),"lines":line_json});
        let adapter=adapter_for(&pcode,&pname,&pkind,&pformat); let envelope=adapter.prepare(&invoice_payload).map_err(ServerFnError::new)?; let content_hash=fingerprint_json(&envelope.payload);
        let ext_ref=envelope.external_reference.clone();
        sqlx::query("INSERT INTO einvoice_documents(legal_entity_id,invoice_id,provider_code,flow,format_code,external_id,payload_json,content_hash,emitted_at) VALUES($1,$2,$3,'ISSUED',$4,$5,$6,$7,now())")
            .bind(entity).bind(invoice_id).bind(&pcode).bind(&envelope.format_code).bind(&ext_ref).bind(&envelope.payload).bind(&content_hash).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO einvoice_events(legal_entity_id,invoice_id,provider_code,direction,event_type,external_id,status_code,payload) VALUES($1,$2,$3,'OUTBOUND','PREPARED',$4,'READY',$5)")
            .bind(entity).bind(invoice_id).bind(&pcode).bind(&ext_ref).bind(&envelope.payload).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE invoices SET electronic_status='READY',electronic_provider_code=$3,electronic_format=$4,electronic_external_id=$5,electronic_received_at=NULL,updated_at=now() WHERE id=$1 AND legal_entity_id=$2")
            .bind(invoice_id).bind(entity).bind(&pcode).bind(&envelope.format_code).bind(&ext_ref).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(format!("Préparation e-facture créée • {} • {} • empreinte {}",pname,envelope.format_code,content_hash))
    }
    #[cfg(not(feature = "server"))]
    { let _=invoice_id; Err(ServerFnError::new("prepare_einvoice est exécutée côté serveur")) }
}

#[server]
pub async fn queue_einvoice_emission(invoice_id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        let row=sqlx::query("SELECT electronic_provider_code,electronic_external_id,status FROM invoices WHERE id=$1 AND legal_entity_id=$2").bind(invoice_id).bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Facture introuvable"))?;
        use sqlx::Row; let provider:String=row.get("electronic_provider_code"); let external_id:String=row.get("electronic_external_id"); let status:String=row.get("status");
        if provider.trim().is_empty() || external_id.trim().is_empty() { return Err(ServerFnError::new("Préparer d'abord la e-facture")); }
        if status=="DRAFT" { return Err(ServerFnError::new("Brouillon interdit à l'émission")); }
        sqlx::query("UPDATE invoices SET electronic_status='QUEUED',electronic_sent_at=NULL,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(invoice_id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO einvoice_events(legal_entity_id,invoice_id,provider_code,direction,event_type,external_id,status_code,payload) VALUES($1,$2,$3,'OUTBOUND','SUBMITTED',$4,'QUEUED',$5)").bind(entity).bind(invoice_id).bind(provider).bind(external_id).bind(json!({"transport":"adapter_queue","network_transmission":false})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=invoice_id; Err(ServerFnError::new("queue_einvoice_emission est exécutée côté serveur")) }
}

#[server]
pub async fn record_einvoice_status(invoice_id: Option<Uuid>, provider_code: String, event_type: String, external_id: String, status_code: String, payload: serde_json::Value, error_code: String, error_message: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let kind=event_type.trim().to_uppercase();
        if !matches!(kind.as_str(),"ACKNOWLEDGED"|"DELIVERED"|"RECEIVED"|"REJECTED"|"ERROR") { return Err(ServerFnError::new("Événement e-facture invalide")); }
        sqlx::query("INSERT INTO einvoice_events(legal_entity_id,invoice_id,provider_code,direction,event_type,external_id,status_code,payload,error_code,error_message) VALUES($1,$2,$3,'INBOUND',$4,$5,$6,$7,$8,$9)")
            .bind(entity).bind(invoice_id).bind(provider_code.trim()).bind(&kind).bind(external_id.trim()).bind(status_code.trim()).bind(&payload).bind(error_code.trim()).bind(error_message.trim()).execute(pool).await.map_err(ServerFnError::new)?;
        if let Some(id)=invoice_id {
            let mapped=match kind.as_str(){"ACKNOWLEDGED"=>"ACKNOWLEDGED","DELIVERED"=>"DELIVERED","RECEIVED"=>"RECEIVED","REJECTED"=>"REJECTED","ERROR"=>"ERROR",_=>"ERROR"};
            sqlx::query("UPDATE invoices SET electronic_status=$3,electronic_external_id=COALESCE(NULLIF($4,''),electronic_external_id),electronic_received_at=now(),updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).bind(mapped).bind(external_id.trim()).execute(pool).await.map_err(ServerFnError::new)?;
        }
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=(invoice_id,provider_code,event_type,external_id,status_code,payload,error_code,error_message); Err(ServerFnError::new("record_einvoice_status est exécutée côté serveur")) }
}

#[server]
pub async fn receive_einvoice(provider_code:String, external_id:String, format_code:String, payload:serde_json::Value, invoice_id:Option<Uuid>) -> Result<Uuid,ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let hash=fingerprint_json(&payload);
        let id:Uuid=sqlx::query_scalar("INSERT INTO einvoice_documents(legal_entity_id,invoice_id,provider_code,flow,format_code,external_id,payload_json,content_hash,received_at) VALUES($1,$2,$3,'RECEIVED',$4,$5,$6,$7,now()) RETURNING id")
            .bind(entity).bind(invoice_id).bind(provider_code.trim()).bind(format_code.trim()).bind(external_id.trim()).bind(&payload).bind(hash).fetch_one(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO einvoice_events(legal_entity_id,invoice_id,provider_code,direction,event_type,external_id,status_code,payload) VALUES($1,$2,$3,'INBOUND','RECEIVED',$4,'RECEIVED',$5)")
            .bind(entity).bind(invoice_id).bind(provider_code.trim()).bind(external_id.trim()).bind(&payload).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _=(provider_code,external_id,format_code,payload,invoice_id); Err(ServerFnError::new("receive_einvoice est exécutée côté serveur")) }
}

#[server]
pub async fn prepare_ereporting(period_start:NaiveDate, period_end:NaiveDate, report_type:String)->Result<EReportingResult,ServerFnError>{
    #[cfg(feature="server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let kind=report_type.trim().to_uppercase();
        if period_end<period_start{return Err(ServerFnError::new("Période e-reporting invalide"));}
        let provider:String=sqlx::query_scalar("SELECT provider_code FROM einvoice_providers WHERE legal_entity_id=$1 AND active=true ORDER BY effective_from DESC LIMIT 1").bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Aucune plateforme active pour l'e-reporting"))?;
        let invoice_count:i64=sqlx::query_scalar("SELECT COUNT(*)::bigint FROM invoices WHERE legal_entity_id=$1 AND issue_date BETWEEN $2 AND $3 AND status<>'CANCELLED'").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let payment_count:i64=sqlx::query_scalar("SELECT COUNT(*)::bigint FROM payments WHERE legal_entity_id=$1 AND payment_date BETWEEN $2 AND $3").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let net:i64=sqlx::query_scalar("SELECT COALESCE(SUM(net_cents),0)::bigint FROM invoices WHERE legal_entity_id=$1 AND issue_date BETWEEN $2 AND $3 AND status<>'CANCELLED'").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let paid:i64=sqlx::query_scalar("SELECT COALESCE(SUM(amount_cents),0)::bigint FROM payments WHERE legal_entity_id=$1 AND payment_date BETWEEN $2 AND $3").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let payload=json!({"report_type":kind,"period_start":period_start,"period_end":period_end,"invoice_count":invoice_count,"payment_count":payment_count,"invoice_net_cents":net,"payment_cents":paid});
        let id:Uuid=sqlx::query_scalar("INSERT INTO einvoice_ereporting(legal_entity_id,provider_code,period_start,period_end,report_type,status,payload_json) VALUES($1,$2,$3,$4,$5,'READY',$6) RETURNING id").bind(entity).bind(&provider).bind(period_start).bind(period_end).bind(&kind).bind(&payload).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(EReportingResult{id,provider_code:provider,report_type:kind,period_start,period_end,invoice_count,payment_count,transaction_net_cents:net,payment_cents:paid,status:"READY".into()})
    }
    #[cfg(not(feature="server"))]
    { let _=(period_start,period_end,report_type); Err(ServerFnError::new("prepare_ereporting est exécutée côté serveur")) }
}

#[component]
pub fn EInvoicePage(mut refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let providers = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_einvoice_providers().await.unwrap_or_default() } });
    let events = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_einvoice_events(40).await.unwrap_or_default() } });
    let mut code = use_signal(|| "GENERIC_PDP".to_owned());
    let mut name = use_signal(|| "Plateforme agréée générique".to_owned());
    let mut kind = use_signal(|| "PDP_ADAPTER".to_owned());
    let mut format = use_signal(|| "MIXED".to_owned());
    let mut endpoint = use_signal(String::new);
    let mut credential = use_signal(String::new);
    let mut date = use_signal(|| Utc::now().date_naive().to_string());
    let mut invoice_id = use_signal(String::new);
    let mut msg = use_signal(String::new);
    let mut start = use_signal(|| Utc::now().date_naive().with_day(1).unwrap_or_else(|| Utc::now().date_naive()).to_string());
    let mut end = use_signal(|| Utc::now().date_naive().to_string());

    rsx! {
        ModuleHeader { title: "E-facturation", kicker: "PROVIDER • PDP • RÉCEPTION • ÉMISSION • E-REPORTING", detail: "Le cœur métier reste indépendant du fournisseur. Un changement de plateforme conserve l’historique." }
        section {
            class: "two-col",
            div {
                class: "panel",
                h3 { "Changer de plateforme" }
                FormField { label: "Code", value: code(), oninput: move |e: FormEvent| code.set(e.value()) }
                FormField { label: "Nom", value: name(), oninput: move |e: FormEvent| name.set(e.value()) }
                FormField { label: "Adapter", value: kind(), oninput: move |e: FormEvent| kind.set(e.value()) }
                FormField { label: "Format", value: format(), oninput: move |e: FormEvent| format.set(e.value()) }
                FormField { label: "Référence endpoint", value: endpoint(), oninput: move |e: FormEvent| endpoint.set(e.value()) }
                FormField { label: "Référence credential", value: credential(), oninput: move |e: FormEvent| credential.set(e.value()) }
                FormField { label: "Effet à partir du", value: date(), oninput: move |e: FormEvent| date.set(e.value()) }
                button {
                    class: "primary",
                    onclick: move |_| async move {
                        match NaiveDate::parse_from_str(&date(), "%Y-%m-%d") {
                            Ok(d) => match activate_einvoice_provider(code(), name(), kind(), format(), endpoint(), credential(), d).await {
                                Ok(_) => { msg.set("Plateforme activée ; ancienne configuration historisée.".into()); bump += 1; }
                                Err(e) => msg.set(e.to_string()),
                            },
                            Err(_) => msg.set("Date invalide".into()),
                        }
                    },
                    "Activer"
                }
            }
            div {
                class: "panel",
                h3 { "Préparer / émettre" }
                FormField { label: "ID facture", value: invoice_id(), oninput: move |e: FormEvent| invoice_id.set(e.value()) }
                button {
                    class: "secondary",
                    onclick: move |_| async move {
                        match Uuid::parse_str(&invoice_id()) {
                            Ok(id) => match prepare_einvoice(id).await { Ok(value) => msg.set(value), Err(e) => msg.set(e.to_string()) },
                            Err(_) => msg.set("ID facture invalide".into()),
                        }
                    },
                    "Préparer e-facture"
                }
                button {
                    class: "secondary",
                    onclick: move |_| async move {
                        match Uuid::parse_str(&invoice_id()) {
                            Ok(id) => match queue_einvoice_emission(id).await { Ok(_) => { msg.set("Émission mise en file de l’adapter (aucun envoi réseau implicite).".into()); bump += 1; }, Err(e) => msg.set(e.to_string()) },
                            Err(_) => msg.set("ID facture invalide".into()),
                        }
                    },
                    "Mettre en file"
                }
                FormField { label: "Période début", value: start(), oninput: move |e: FormEvent| start.set(e.value()) }
                FormField { label: "Période fin", value: end(), oninput: move |e: FormEvent| end.set(e.value()) }
                button {
                    class: "secondary",
                    onclick: move |_| async move {
                        let from = NaiveDate::parse_from_str(&start(), "%Y-%m-%d");
                        let to = NaiveDate::parse_from_str(&end(), "%Y-%m-%d");
                        match (from, to) {
                            (Ok(from), Ok(to)) => match prepare_ereporting(from, to, "TRANSACTION".into()).await {
                                Ok(r) => msg.set(format!("E-reporting prêt : {} factures, {} paiements, net {}", r.invoice_count, r.payment_count, r.transaction_net_cents)),
                                Err(e) => msg.set(e.to_string()),
                            },
                            _ => msg.set("Période invalide".into()),
                        }
                    },
                    "Préparer e-reporting"
                }
            }
        }
        span { class: "save-ok", "{msg}" }
        section {
            class: "panel",
            h3 { "Fournisseurs" }
            for p in providers.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{p.provider_name} • {p.provider_code}" }
                        div { class: "small", "{p.adapter_kind} • {p.format_code} • effet {p.effective_from}" }
                    }
                    div { class: "row-value", if p.active { "ACTIF" } else { "HISTORIQUE" } }
                }
            }
        }
        section {
            class: "panel",
            h3 { "Historique e-facturation" }
            for e in events.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{e.event_type} • {e.provider_code} • {e.direction}" }
                        div { class: "small", "{e.occurred_at} • {e.status_code} • {e.external_id}" }
                        if !e.error_message.is_empty() { div { class: "small", "{e.error_message}" } }
                    }
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn pdp_adapter_is_provider_neutral(){let a=PdpAdapter{code:"PDP1".into(),name:"PDP 1".into(),format:"MIXED".into()};let p=a.prepare(&json!({"invoice_number":"F-1"})).unwrap();assert_eq!(p.provider_code,"PDP1");assert_eq!(p.external_reference,"SCI-F-1");}
    #[test] fn fingerprint_stable(){let a=fingerprint_json(&json!({"a":1}));let b=fingerprint_json(&json!({"a":1}));assert_eq!(a,b);}
}
