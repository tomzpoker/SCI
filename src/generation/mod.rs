use chrono::{NaiveDate, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use crate::entity_scope::current_legal_entity_id;
use crate::ui::{FormField, ModuleHeader};

pub mod pdf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentTemplateItem {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub document_kind: String,
    pub version_no: i32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneratedDocumentItem {
    pub id: Uuid,
    pub document_kind: String,
    pub reference: String,
    pub status: String,
    pub subject: String,
    pub recipient: String,
    pub validation_required: bool,
    pub validation_status: String,
    pub pdf_path: String,
    pub pdf_watermark: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaseGenerationCheck {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub payload: Value,
}

fn kind_requires_validation(kind: &str) -> bool {
    matches!(kind, "LEASE" | "MISE_EN_DEMEURE" | "RELANCE" | "TAXE" | "ECHEANCIER" | "REVISION")
}

fn render_template_text(template: &str, payload: &Value) -> String {
    let mut out = template.to_string();
    if let Some(map) = payload.as_object() {
        for (key, value) in map {
            let replacement = match value {
                Value::Null => String::new(),
                Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            out = out.replace(&format!("{{{{{}}}}}", key), &replacement);
        }
    }
    out
}

fn body_html(text: &str) -> String {
    let escaped = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!("<div style=\"font-family:Arial,sans-serif;white-space:pre-wrap\">{}</div>", escaped)
}

fn generated_root() -> PathBuf {
    std::env::var_os("SCI_GENERATED_ROOT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("storage/generated"))
}

#[server]
pub async fn list_templates() -> Result<Vec<DocumentTemplateItem>, ServerFnError> {
    #[cfg(feature="server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query("SELECT id,code,name,document_kind,version_no,active FROM document_templates WHERE legal_entity_id=$1 AND active=true ORDER BY document_kind,code,version_no DESC")
            .bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row;
        Ok(rows.into_iter().map(|r| DocumentTemplateItem{id:r.get("id"),code:r.get("code"),name:r.get("name"),document_kind:r.get("document_kind"),version_no:r.get("version_no"),active:r.get("active")}).collect())
    }
    #[cfg(not(feature="server"))]
    { Err(ServerFnError::new("list_templates est exécutée côté serveur")) }
}

#[server]
pub async fn save_template(code:String, name:String, document_kind:String, subject_template:String, body_template:String, variables:Value) -> Result<Uuid,ServerFnError> {
    #[cfg(feature="server")]
    {
        if code.trim().is_empty() || name.trim().is_empty() || body_template.trim().is_empty() { return Err(ServerFnError::new("Template incomplet")); }
        let kind = document_kind.trim().to_uppercase();
        let allowed = ["COURRIER","EMAIL","LEASE","JUSTIFICATIF","FACTURE","RELANCE","MISE_EN_DEMEURE","REGULARISATION","REVISION","TAXE","ECHEANCIER","OTHER"];
        if !allowed.contains(&kind.as_str()) { return Err(ServerFnError::new("Type de document invalide")); }
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        let version:i32=sqlx::query_scalar("SELECT COALESCE(MAX(version_no),0)+1 FROM document_templates WHERE legal_entity_id=$1 AND code=$2").bind(entity).bind(code.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let id:Uuid=sqlx::query_scalar("INSERT INTO document_templates(legal_entity_id,code,name,document_kind,version_no,subject_template,body_template,variables,active) VALUES($1,$2,$3,$4,$5,$6,$7,$8,true) RETURNING id")
            .bind(entity).bind(code.trim()).bind(name.trim()).bind(kind).bind(version).bind(subject_template).bind(body_template).bind(variables).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature="server"))]
    { let _=(code,name,document_kind,subject_template,body_template,variables); Err(ServerFnError::new("save_template est exécutée côté serveur")) }
}

#[server]
pub async fn save_pdf_profile(header:String,footer:String,logo_path:String)->Result<Uuid,ServerFnError>{
    #[cfg(feature="server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        let id:Uuid=sqlx::query_scalar("INSERT INTO pdf_profiles(legal_entity_id,code,page_size,header_text,footer_text,logo_path,active) VALUES($1,'DEFAULT_A4','A4',$2,$3,$4,true) ON CONFLICT(legal_entity_id,code) DO UPDATE SET header_text=EXCLUDED.header_text,footer_text=EXCLUDED.footer_text,logo_path=EXCLUDED.logo_path,active=true RETURNING id").bind(entity).bind(header.trim()).bind(footer.trim()).bind(logo_path.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature="server"))] { let _=(header,footer,logo_path); Err(ServerFnError::new("save_pdf_profile est exécutée côté serveur")) }
}

#[server]
pub async fn create_generated_document(template_code:String, reference:String, recipient:String, payload:Value) -> Result<GeneratedDocumentItem,ServerFnError> {
    #[cfg(feature="server")]
    {
        if reference.trim().is_empty() { return Err(ServerFnError::new("Référence requise")); }
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        let row=sqlx::query("SELECT id,document_kind,subject_template,body_template FROM document_templates WHERE legal_entity_id=$1 AND code=$2 AND active=true ORDER BY version_no DESC LIMIT 1")
            .bind(entity).bind(template_code.trim()).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Template introuvable"))?;
        use sqlx::Row;
        let template_id:Uuid=row.get("id"); let kind:String=row.get("document_kind"); let subject=render_template_text(row.get::<String,_>("subject_template").as_str(),&payload); let body=render_template_text(row.get::<String,_>("body_template").as_str(),&payload);
        let validation_required=kind_requires_validation(&kind);
        let validation_status=if validation_required{"PENDING"}else{"NOT_REQUIRED"};
        let id:Uuid=sqlx::query_scalar("INSERT INTO generated_documents(legal_entity_id,template_id,document_kind,reference,status,subject,recipient,body_text,body_html,source_payload,validation_required,validation_status,pdf_watermark) VALUES($1,$2,$3,$4,'PREVIEWED',$5,$6,$7,$8,$9,$10,$11,$12) RETURNING id")
            .bind(entity).bind(template_id).bind(&kind).bind(reference.trim()).bind(&subject).bind(recipient.trim()).bind(&body).bind(body_html(&body)).bind(payload).bind(validation_required).bind(validation_status).bind(if validation_required{"BROUILLON — PRÉVISUALISATION"}else{""}).fetch_one(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO document_generation_events(legal_entity_id,generated_document_id,event_type,actor,payload) VALUES($1,$2,'PREVIEWED','USER',$3)").bind(entity).bind(id).bind(json!({"template":template_code.trim()})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(GeneratedDocumentItem{id,document_kind:kind,reference:reference.trim().into(),status:"PREVIEWED".into(),subject,recipient:recipient.trim().into(),validation_required,validation_status:validation_status.into(),pdf_path:String::new(),pdf_watermark:if validation_required{"BROUILLON — PRÉVISUALISATION".into()}else{String::new()},created_at:Utc::now().to_rfc3339()})
    }
    #[cfg(not(feature="server"))]
    { let _=(template_code,reference,recipient,payload); Err(ServerFnError::new("create_generated_document est exécutée côté serveur")) }
}

#[server]
pub async fn approve_generated_document(id:Uuid, approve:bool, reason:String) -> Result<(),ServerFnError> {
    #[cfg(feature="server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        let (required,status):(bool,String)=sqlx::query_as("SELECT validation_required,validation_status FROM generated_documents WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        if !required { return Ok(()); }
        if status!="PENDING" { return Err(ServerFnError::new("Document déjà traité")); }
        let new_status=if approve{"APPROVED"}else{"REJECTED"}; let doc_status=if approve{"READY"}else{"CANCELLED"};
        sqlx::query("UPDATE generated_documents SET validation_status=$3,status=$4,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).bind(new_status).bind(doc_status).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO document_generation_events(legal_entity_id,generated_document_id,event_type,actor,payload) VALUES($1,$2,$3,'MANAGER',$4)").bind(entity).bind(id).bind(if approve{"VALIDATED"}else{"REJECTED"}).bind(json!({"reason":reason.trim()})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature="server"))]
    { let _=(id,approve,reason); Err(ServerFnError::new("approve_generated_document est exécutée côté serveur")) }
}

#[server]
pub async fn list_generated_documents() -> Result<Vec<GeneratedDocumentItem>,ServerFnError> {
    #[cfg(feature="server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT id,document_kind,reference,status,subject,recipient,validation_required,validation_status,COALESCE(pdf_path,'') AS pdf_path,pdf_watermark,created_at::text AS created_at FROM generated_documents WHERE legal_entity_id=$1 ORDER BY created_at DESC LIMIT 100").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; use sqlx::Row;
        Ok(rows.into_iter().map(|r|GeneratedDocumentItem{id:r.get("id"),document_kind:r.get("document_kind"),reference:r.get("reference"),status:r.get("status"),subject:r.get("subject"),recipient:r.get("recipient"),validation_required:r.get("validation_required"),validation_status:r.get("validation_status"),pdf_path:r.get("pdf_path"),pdf_watermark:r.get("pdf_watermark"),created_at:r.get("created_at")}).collect())
    }
    #[cfg(not(feature="server"))]
    { Err(ServerFnError::new("list_generated_documents est exécutée côté serveur")) }
}

#[server]
pub async fn generate_generated_document_pdf(id:Uuid) -> Result<String,ServerFnError> {
    #[cfg(feature="server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        let row=sqlx::query("SELECT reference,subject,body_text,status,validation_required,validation_status FROM generated_documents WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?; use sqlx::Row;
        let reference:String=row.get("reference"); let subject:String=row.get("subject"); let body:String=row.get("body_text"); let status:String=row.get("status"); let required:bool=row.get("validation_required"); let validation:String=row.get("validation_status");
        if required && validation!="APPROVED" { return Err(ServerFnError::new("PDF définitif bloqué : validation humaine requise")); }
        let profile=sqlx::query("SELECT header_text,footer_text,logo_path,margin_top_mm::double precision AS margin_top_mm,margin_right_mm::double precision AS margin_right_mm,margin_bottom_mm::double precision AS margin_bottom_mm,margin_left_mm::double precision AS margin_left_mm FROM pdf_profiles WHERE legal_entity_id=$1 AND active=true ORDER BY code LIMIT 1").bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        let header=profile.as_ref().map(|r|r.get::<String,_>("header_text")).unwrap_or_else(||"SCI FAMILY".into()); let footer=profile.as_ref().map(|r|r.get::<String,_>("footer_text")).unwrap_or_else(||"SCI FAMILY — {reference} — {status} — Page {page}/{pages}".into()); let logo_path=profile.as_ref().map(|r|r.get::<String,_>("logo_path")).unwrap_or_default();
        let mut lines=Vec::new(); lines.push(subject); lines.push(format!("Référence : {}",reference)); lines.push(format!("État : {}",status)); lines.push(format!("Date : {}",Utc::now().date_naive())); lines.push(String::new()); lines.extend(body.lines().map(String::from));
        let watermark=if status=="PREVIEWED" || (required && validation!="APPROVED") {"BROUILLON — PRÉVISUALISATION"} else {""};
        let path=generated_root().join(entity.to_string()).join(format!("{}.pdf",safe_filename(&reference))); fs::create_dir_all(path.parent().unwrap()).map_err(ServerFnError::new)?;
        let spec=pdf::PdfSpec{header,footer,reference:reference.clone(),status:status.clone(),date:Utc::now().date_naive(),lines,watermark:watermark.into(),logo_path:(!logo_path.trim().is_empty()).then(||PathBuf::from(logo_path)),margins_mm:profile.map(|r|(r.get("margin_top_mm"),r.get("margin_right_mm"),r.get("margin_bottom_mm"),r.get("margin_left_mm"))).unwrap_or((18.0,18.0,18.0,18.0))};
        pdf::write_pdf(&path,&spec).map_err(ServerFnError::new)?;
        sqlx::query("UPDATE generated_documents SET pdf_path=$3,pdf_watermark=$4,pdf_generated_at=now(),status=CASE WHEN status='PREVIEWED' THEN status ELSE 'READY' END,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).bind(path.to_string_lossy().to_string()).bind(watermark).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO document_generation_events(legal_entity_id,generated_document_id,event_type,actor,payload) VALUES($1,$2,'PDF_GENERATED','SYSTEM',$3)").bind(entity).bind(id).bind(json!({"path":path,"watermark":watermark})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(path.to_string_lossy().to_string())
    }
    #[cfg(not(feature="server"))]
    { let _=id; Err(ServerFnError::new("generate_generated_document_pdf est exécutée côté serveur")) }
}

fn safe_filename(value:&str)->String { let mut s=value.chars().map(|c|if c.is_ascii_alphanumeric(){c}else{'_'}).collect::<String>(); if s.is_empty(){s="document".into();} s }

#[server]
pub async fn validate_lease_for_generation(lease_id:Uuid)->Result<LeaseGenerationCheck,ServerFnError>{
    #[cfg(feature="server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        let row=sqlx::query("SELECT l.reference,l.start_date,l.end_date,l.rent_amount_cents,l.vat_mode,l.index_code,l.index_base_value,l.index_base_date,l.charges_amount_cents,t.legal_name AS tenant_name,u.label AS unit_label,p.name AS property_name FROM leases l JOIN tenants t ON t.id=l.tenant_id JOIN units u ON u.id=l.unit_id JOIN properties p ON p.id=u.property_id WHERE l.id=$1 AND t.legal_entity_id=$2 AND p.legal_entity_id=$2").bind(lease_id).bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row;
        let reference:String=row.get("reference"); let start:NaiveDate=row.get("start_date"); let end:Option<NaiveDate>=row.get("end_date"); let rent:i64=row.get("rent_amount_cents"); let vat_mode:String=row.get("vat_mode"); let index_code:Option<String>=row.get("index_code"); let index_base_value:Option<rust_decimal::Decimal>=row.get("index_base_value"); let index_base_date:Option<NaiveDate>=row.get("index_base_date"); let charges:i64=row.get("charges_amount_cents");
        let mut errors=Vec::new(); let mut warnings=Vec::new(); if reference.trim().is_empty(){errors.push("Référence du bail manquante".into());} if let Some(e)=end {if e<start{errors.push("Dates du bail incohérentes".into());}} if rent<=0{errors.push("Loyer contractuel non positif".into());} if charges<0{errors.push("Charges contractuelles négatives".into());} if index_code.as_deref().unwrap_or("").trim().is_empty(){warnings.push("Aucune indexation renseignée".into());} else {if index_base_value.is_none(){errors.push("Indice de base manquant".into());} if index_base_date.is_none(){errors.push("Date de base de l'indice manquante".into());}} if vat_mode.trim().is_empty(){errors.push("Mode TVA manquant".into());} if vat_mode.eq_ignore_ascii_case("NONE") {warnings.push("Bail sans TVA".into());}
        let clauses:String=sqlx::query_scalar("SELECT COALESCE(string_agg(body,E'\\n' ORDER BY code,version_no),'') FROM lease_clauses WHERE legal_entity_id=$1 AND lease_id=$2").bind(entity).bind(lease_id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let payload=json!({"reference":reference,"tenant_name":row.get::<String,_>("tenant_name"),"property_name":row.get::<String,_>("property_name"),"unit_label":row.get::<String,_>("unit_label"),"start_date":start,"end_date":end,"rent":format_eur(rent),"charges":format_eur(charges),"vat_mode":vat_mode,"index_code":index_code.unwrap_or_default(),"clauses":clauses});
        Ok(LeaseGenerationCheck{valid:errors.is_empty(),errors,warnings,payload})
    }
    #[cfg(not(feature="server"))]
    { let _=lease_id; Err(ServerFnError::new("validate_lease_for_generation est exécutée côté serveur")) }
}

fn format_eur(cents:i64)->String { format!("{}.{:02} €",cents/100,cents.abs()%100) }

#[server]
pub async fn generate_lease_document(lease_id:Uuid)->Result<GeneratedDocumentItem,ServerFnError>{
    #[cfg(feature="server")]
    {
        let check=validate_lease_for_generation(lease_id).await?; if !check.valid{return Err(ServerFnError::new(format!("Bail non générable : {}",check.errors.join(" ; "))));}
        let reference=check.payload.get("reference").and_then(Value::as_str).unwrap_or("BAIL").to_string(); let recipient=check.payload.get("tenant_name").and_then(Value::as_str).unwrap_or("").to_string();
        create_generated_document("BAIL_STANDARD".into(),format!("BAIL-{}",reference),recipient,check.payload).await
    }
    #[cfg(not(feature="server"))]
    { let _=lease_id; Err(ServerFnError::new("generate_lease_document est exécutée côté serveur")) }
}

#[component]
pub fn GenerationPage(mut refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let templates = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_templates().await.unwrap_or_default() } });
    let docs = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_generated_documents().await.unwrap_or_default() } });
    let leases = use_resource(move || { let _ = refresh(); async move { crate::leases::list_lease_details().await.unwrap_or_default() } });
    let mut template_code = use_signal(|| "JUSTIFICATIF_STANDARD".to_string());
    let mut reference = use_signal(|| "PREVIEW-001".to_string());
    let mut recipient = use_signal(String::new);
    let mut details = use_signal(|| "Prévisualisation manuelle".to_string());
    let mut lease_id = use_signal(String::new);
    let mut msg = use_signal(String::new);

    rsx! {
        ModuleHeader { title: "Courriers / PDF / Génération", kicker: "TEMPLATES • APERÇU • VALIDATION • PDF", detail: "Les modèles sont versionnés. Les documents sensibles restent soumis à la politique de validation." }
        section {
            class: "two-col",
            div {
                class: "panel",
                h3 { "Prévisualiser courrier / email" }
                FormField { label: "Template", value: template_code(), oninput: move |e: FormEvent| template_code.set(e.value()) }
                FormField { label: "Référence", value: reference(), oninput: move |e: FormEvent| reference.set(e.value()) }
                FormField { label: "Destinataire", value: recipient(), oninput: move |e: FormEvent| recipient.set(e.value()) }
                FormField { label: "Détails", value: details(), oninput: move |e: FormEvent| details.set(e.value()) }
                button {
                    class: "primary",
                    onclick: move |_| async move {
                        let current_reference = reference();
                        let current_details = details();
                        let current_recipient = recipient();
                        let current_template = template_code();
                        let payload = json!({
                            "reference": current_reference,
                            "details": current_details,
                            "entity_name": "SCI FAMILY",
                            "tenant_name": current_recipient,
                            "invoice_number": current_reference,
                            "outstanding": "0.00",
                            "due_date": Utc::now().date_naive(),
                            "period": "Période à préciser",
                            "amount": "0.00",
                            "legal_basis": "À qualifier",
                            "tax_label": "Taxe à qualifier",
                            "provision": "0.00",
                            "actual": "0.00",
                            "difference": "0.00",
                            "old_rent": "0.00",
                            "new_index": "À préciser",
                            "new_rent": "0.00",
                            "schedule": "À définir"
                        });
                        match create_generated_document(current_template, current_reference, current_recipient, payload).await {
                            Ok(_) => { msg.set("Prévisualisation créée".into()); bump += 1; }
                            Err(e) => msg.set(e.to_string()),
                        }
                    },
                    "Créer la prévisualisation"
                }
            }
            div {
                class: "panel",
                h3 { "Bail structuré → contrôle → document" }
                FormField { label: "ID du bail", value: lease_id(), oninput: move |e: FormEvent| lease_id.set(e.value()) }
                button {
                    class: "secondary",
                    onclick: move |_| async move {
                        match Uuid::parse_str(&lease_id()) {
                            Ok(id) => match validate_lease_for_generation(id).await {
                                Ok(c) => msg.set(if c.valid { format!("Bail cohérent • {} avertissement(s)", c.warnings.len()) } else { format!("Bail bloqué • {} erreur(s)", c.errors.len()) }),
                                Err(e) => msg.set(e.to_string()),
                            },
                            Err(_) => msg.set("ID de bail invalide".into()),
                        }
                    },
                    "Contrôler le bail"
                }
                button {
                    class: "primary",
                    onclick: move |_| async move {
                        match Uuid::parse_str(&lease_id()) {
                            Ok(id) => match generate_lease_document(id).await {
                                Ok(_) => { msg.set("Bail généré en prévisualisation".into()); bump += 1; }
                                Err(e) => msg.set(e.to_string()),
                            },
                            Err(_) => msg.set("ID de bail invalide".into()),
                        }
                    },
                    "Générer le bail"
                }
                div {
                    class: "small",
                    "Baux disponibles : "
                    for l in leases.read().as_deref().unwrap_or(&[]).iter() {
                        span { class: "small", "{l.id} • {l.reference} · " }
                    }
                }
            }
        }
        span { class: "save-ok", "{msg}" }
        section {
            class: "two-col",
            div {
                class: "panel",
                h3 { "Templates disponibles" }
                for t in templates.read().as_deref().unwrap_or(&[]).iter() {
                    div {
                        class: "data-row",
                        div {
                            div { class: "data-title", "{t.name}" }
                            div { class: "small", "{t.code} • {t.document_kind} • v{t.version_no}" }
                        }
                    }
                }
            }
            div {
                class: "panel",
                h3 { "Profil PDF A4" }
                p { "Le moteur accepte A4, en-tête, pied de page, pagination, référence, statut, date et logo JPEG configuré en base." }
                p { "Le chemin du logo est stocké dans pdf_profiles.logo_path ; les JPEG sont embarqués dans le PDF lorsqu’ils sont accessibles." }
                h3 { "Contrôle avant PDF" }
                p { "Un PDF définitif est bloqué tant qu’une validation requise n’a pas été approuvée. Les PDF d’aperçu peuvent porter un filigrane BROUILLON." }
            }
        }
        section {
            class: "panel",
            h3 { "Documents générés" }
            for d in docs.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{d.reference} • {d.status}" }
                        div { class: "small", "{d.document_kind} • validation {d.validation_status}" }
                    }
                    div {
                        class: "row-actions",
                        if d.validation_status == "PENDING" {
                            button {
                                class: "secondary",
                                onclick: { let id = d.id; move |_| async move { match approve_generated_document(id, true, "Validation manager".into()).await { Ok(_) => { msg.set("Document validé".into()); bump += 1; }, Err(e) => msg.set(e.to_string()) } } },
                                "Valider"
                            }
                        }
                        if d.status != "CANCELLED" && (d.validation_status == "APPROVED" || !d.validation_required) {
                            button {
                                class: "secondary",
                                onclick: { let id = d.id; move |_| async move { match generate_generated_document_pdf(id).await { Ok(_) => { msg.set("PDF généré".into()); bump += 1; }, Err(e) => msg.set(e.to_string()) } } },
                                "Générer PDF"
                            }
                        }
                    }
                }
            }
        }
    }
}


#[cfg(test)]
mod tests { use super::*; #[test] fn template_render(){let s=render_template_text("Bonjour {{name}} — {{n}}",&json!({"name":"SCI","n":3}));assert_eq!(s,"Bonjour SCI — 3");} #[test] fn validation_kind(){assert!(kind_requires_validation("LEASE"));assert!(!kind_requires_validation("JUSTIFICATIF"));} }
