use chrono::{Datelike, Duration, NaiveDate, Utc};
use dioxus::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::entity_scope::current_legal_entity_id;
use crate::ui::{FormField, ModuleHeader};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VatDeclarationItem {
    pub id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub form_code: String,
    pub status: String,
    pub ca_ht_cents: i64,
    pub collected_vat_cents: i64,
    pub deductible_vat_cents: i64,
    pub payable_vat_cents: i64,
    pub credit_vat_cents: i64,
    pub corrections_vat_cents: i64,
    pub anomalies: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VatAdvanceItem {
    pub id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub payment_date: NaiveDate,
    pub taxable_base_cents: i64,
    pub vat_rate_bp: i32,
    pub vat_cents: i64,
    pub status: String,
    pub source_type: String,
    pub source_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VatEntryItem {
    pub id: Uuid,
    pub entry_kind: String,
    pub exigibility_basis: String,
    pub transaction_date: NaiveDate,
    pub taxable_net_cents: i64,
    pub vat_rate_bp: i32,
    pub vat_cents: i64,
    pub source_type: String,
    pub source_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FiscalResultItem {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub management_income_cents: i64,
    pub accounting_result_cents: i64,
    pub fiscal_result_cents: i64,
    pub treasury_result_cents: i64,
    pub deductions_cents: i64,
    pub reintegrations_cents: i64,
    pub anomalies: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fiscal2072Item {
    pub id: Uuid,
    pub tax_year: i32,
    pub form_code: String,
    pub status: String,
    pub data: Value,
    pub anomalies: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuotePartItem {
    pub associate_id: Uuid,
    pub associate_name: String,
    pub ownership_pct: Decimal,
    pub amount_cents: i64,
    pub formula: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VatDeclarationFieldItem {
    pub field_code: String,
    pub label: String,
    pub value_cents: Option<i64>,
    pub value_text: Option<String>,
    pub source_type: String,
    pub source_ids: Value,
    pub formula: String,
    pub validation_status: String,
    pub validation_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FiscalDossierItem {
    pub id: Uuid,
    pub run_id: Uuid,
    pub tax_year: i32,
    pub revenues_cents: i64,
    pub charges_cents: i64,
    pub result_cents: i64,
    pub quote_parts: Value,
    pub document_ids: Value,
    pub history: Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaxObligationItem {
    pub id: Uuid,
    pub tax_code: String,
    pub label: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub applicability: String,
    pub qualification_needed: bool,
    pub due_date: Option<NaiveDate>,
    pub amount_cents: Option<i64>,
    pub notes: String,
}

fn month_end(start: NaiveDate) -> NaiveDate {
    let next = if start.month() == 12 {
        NaiveDate::from_ymd_opt(start.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(start.year(), start.month() + 1, 1).unwrap()
    };
    next - Duration::days(1)
}

fn euro(cents: i64) -> String { format!("{}.{:02} €", cents / 100, cents.abs() % 100) }

fn vat_status_fr(code: &str) -> &'static str {
    match code {
        "DRAFT" => "Brouillon",
        "VALIDATION_REQUIRED" => "Validation requise",
        "VALIDATED" => "Validée",
        "FINAL" => "Finale",
        "BLOCKED" => "Bloquée",
        _ => "Inconnu",
    }
}

#[server]
pub async fn register_vat_entry(
    period_start: NaiveDate,
    period_end: NaiveDate,
    entry_kind: String,
    exigibility_basis: String,
    transaction_date: NaiveDate,
    taxable_net_cents: i64,
    vat_rate_bp: i32,
    vat_cents: i64,
    source_type: String,
    source_id: Option<Uuid>,
    source_document_id: Option<Uuid>,
    source_label: String,
    idempotency_key: Option<String>,
    notes: String,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if period_end < period_start || taxable_net_cents < 0 || !(0..=10000).contains(&vat_rate_bp) || source_type.trim().is_empty() {
            return Err(ServerFnError::new("Entrée TVA invalide"));
        }
        if vat_cents.abs() > ((Decimal::from(taxable_net_cents) * Decimal::from(vat_rate_bp)).round_dp(0).to_i64().unwrap_or(i64::MAX)).saturating_add(1) {
            return Err(ServerFnError::new("TVA incohérente avec la base et le taux"));
        }
        let kind = entry_kind.trim().to_uppercase();
        if !matches!(kind.as_str(), "COLLECTED" | "DEDUCTIBLE" | "CORRECTION") { return Err(ServerFnError::new("Type TVA invalide")); }
        let basis = exigibility_basis.trim().to_uppercase();
        if !matches!(basis.as_str(), "COLLECTION" | "INVOICE" | "ADJUSTMENT" | "UNKNOWN") { return Err(ServerFnError::new("Base d'exigibilité invalide")); }
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let id: Uuid = sqlx::query_scalar(r#"
            INSERT INTO vat_entries(legal_entity_id,period_start,period_end,entry_kind,exigibility_basis,transaction_date,taxable_net_cents,vat_rate_bp,vat_cents,source_type,source_id,source_document_id,source_label,idempotency_key,notes)
            VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,NULLIF($14,''),$15)
            ON CONFLICT(legal_entity_id,idempotency_key) WHERE idempotency_key IS NOT NULL AND btrim(idempotency_key)<>''
            DO UPDATE SET taxable_net_cents=EXCLUDED.taxable_net_cents,vat_rate_bp=EXCLUDED.vat_rate_bp,vat_cents=EXCLUDED.vat_cents,source_label=EXCLUDED.source_label,notes=EXCLUDED.notes
            RETURNING id
        "#)
        .bind(current_legal_entity_id()).bind(period_start).bind(period_end).bind(kind).bind(basis).bind(transaction_date)
        .bind(taxable_net_cents).bind(vat_rate_bp).bind(vat_cents).bind(source_type.trim()).bind(source_id).bind(source_document_id)
        .bind(source_label.trim()).bind(idempotency_key.unwrap_or_default().trim()).bind(notes.trim())
        .fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _=(period_start,period_end,entry_kind,exigibility_basis,transaction_date,taxable_net_cents,vat_rate_bp,vat_cents,source_type,source_id,source_document_id,source_label,idempotency_key,notes); Err(ServerFnError::new("register_vat_entry est exécutée côté serveur")) }
}

#[server]
pub async fn register_vat_advance(
    period_start: NaiveDate,
    period_end: NaiveDate,
    payment_date: NaiveDate,
    taxable_base_cents: i64,
    vat_rate_bp: i32,
    vat_cents: i64,
    source_type: String,
    source_id: Option<Uuid>,
    source_document_id: Option<Uuid>,
    status: String,
    notes: String,
    idempotency_key: Option<String>,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature="server")]
    {
        if period_end < period_start || payment_date < period_start || taxable_base_cents < 0 || !(0..=10000).contains(&vat_rate_bp) || vat_cents < 0 || source_type.trim().is_empty() {
            return Err(ServerFnError::new("Acompte TVA invalide"));
        }
        let expected=((Decimal::from(taxable_base_cents)*Decimal::from(vat_rate_bp))/Decimal::from(10000)).round_dp(0).to_i64().unwrap_or(i64::MAX);
        if vat_cents > expected.saturating_add(1) { return Err(ServerFnError::new("Acompte TVA incohérent avec la base et le taux")); }
        let st=status.trim().to_uppercase(); if !matches!(st.as_str(),"EXPECTED"|"DOCUMENTED"|"PAID"|"CANCELLED") { return Err(ServerFnError::new("Statut d'acompte TVA invalide")); }
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let id:Uuid=sqlx::query_scalar(r#"INSERT INTO vat_advances(legal_entity_id,period_start,period_end,payment_date,taxable_base_cents,vat_rate_bp,vat_cents,source_type,source_id,source_document_id,status,notes,idempotency_key)
            VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,NULLIF($13,''))
            ON CONFLICT(legal_entity_id,idempotency_key) WHERE idempotency_key IS NOT NULL AND btrim(idempotency_key)<>''
            DO UPDATE SET period_start=EXCLUDED.period_start,period_end=EXCLUDED.period_end,payment_date=EXCLUDED.payment_date,taxable_base_cents=EXCLUDED.taxable_base_cents,vat_rate_bp=EXCLUDED.vat_rate_bp,vat_cents=EXCLUDED.vat_cents,status=EXCLUDED.status,notes=EXCLUDED.notes
            RETURNING id"#)
            .bind(current_legal_entity_id()).bind(period_start).bind(period_end).bind(payment_date).bind(taxable_base_cents).bind(vat_rate_bp).bind(vat_cents).bind(source_type.trim()).bind(source_id).bind(source_document_id).bind(st).bind(notes.trim()).bind(idempotency_key.unwrap_or_default().trim()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature="server"))]
    { let _=(period_start,period_end,payment_date,taxable_base_cents,vat_rate_bp,vat_cents,source_type,source_id,source_document_id,status,notes,idempotency_key); Err(ServerFnError::new("register_vat_advance est exécutée côté serveur")) }
}

#[server]
pub async fn list_vat_advances() -> Result<Vec<VatAdvanceItem>, ServerFnError> {
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; use sqlx::Row; let rows=sqlx::query("SELECT id,period_start,period_end,payment_date,taxable_base_cents,vat_rate_bp,vat_cents,status,source_type,COALESCE(source_type,'') AS source_label FROM vat_advances WHERE legal_entity_id=$1 ORDER BY payment_date DESC,id DESC LIMIT 100").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r|VatAdvanceItem{id:r.get("id"),period_start:r.get("period_start"),period_end:r.get("period_end"),payment_date:r.get("payment_date"),taxable_base_cents:r.get("taxable_base_cents"),vat_rate_bp:r.get("vat_rate_bp"),vat_cents:r.get("vat_cents"),status:r.get("status"),source_type:r.get("source_type"),source_label:r.get("source_label")}).collect()) }
    #[cfg(not(feature="server"))]
    { Err(ServerFnError::new("list_vat_advances est exécutée côté serveur")) }
}

#[server]
pub async fn calculate_vat_declaration(period_start: NaiveDate, period_end: NaiveDate) -> Result<VatDeclarationItem, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if period_end < period_start { return Err(ServerFnError::new("Période TVA invalide")); }
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); use sqlx::Row;
        let basis:String=sqlx::query_scalar("SELECT vat_basis FROM legal_entities WHERE id=$1").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let vat_operable:bool=sqlx::query_scalar("SELECT vat_status <> 'NONE' FROM legal_entities WHERE id=$1").bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let mut anomalies=Vec::<Value>::new(); if !vat_operable { anomalies.push(json!({"code":"VAT_STATUS_NONE","message":"Le statut TVA de l'entité est NON ASSUJETTI/INACTIF."})); }
        let collected_query = if basis.to_uppercase()=="COLLECTION" { r#"
            SELECT COALESCE(SUM(CASE WHEN i.document_kind='CREDIT_NOTE' THEN -1 ELSE 1 END * ROUND((p.amount_cents::numeric*i.vat_cents)/NULLIF(i.gross_cents,0)))::bigint,0) AS vat,
                   COALESCE(SUM(CASE WHEN i.document_kind='CREDIT_NOTE' THEN -1 ELSE 1 END * ROUND((p.amount_cents::numeric*i.net_cents)/NULLIF(i.gross_cents,0)))::bigint,0) AS net
            FROM payments p JOIN invoices i ON i.id=p.invoice_id AND i.legal_entity_id=p.legal_entity_id
            WHERE p.legal_entity_id=$1 AND p.received_at::date BETWEEN $2 AND $3 AND i.status NOT IN ('DRAFT','CANCELLED')
        "# } else { r#"
            SELECT COALESCE(SUM(CASE WHEN document_kind='CREDIT_NOTE' THEN -vat_cents ELSE vat_cents END),0)::bigint AS vat,
                   COALESCE(SUM(CASE WHEN document_kind='CREDIT_NOTE' THEN -net_cents ELSE net_cents END),0)::bigint AS net
            FROM invoices WHERE legal_entity_id=$1 AND issue_date BETWEEN $2 AND $3 AND status NOT IN ('DRAFT','CANCELLED')
        "# };
        let c=sqlx::query(collected_query).bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let collected:i64=c.get("vat"); let ca_ht:i64=c.get("net");
        let deductible:i64=sqlx::query_scalar("SELECT COALESCE(SUM(vat_cents),0)::bigint FROM vat_entries WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND entry_kind='DEDUCTIBLE'").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let corrections:i64=sqlx::query_scalar("SELECT COALESCE(SUM(vat_cents),0)::bigint FROM vat_entries WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND entry_kind='CORRECTION'").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let advances:i64=sqlx::query_scalar("SELECT COALESCE(SUM(vat_cents),0)::bigint FROM vat_advances WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND status IN ('DOCUMENTED','PAID')").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let collection_source_ids:Value=if basis.to_uppercase()=="COLLECTION" { sqlx::query_scalar("SELECT COALESCE(jsonb_agg(i.id ORDER BY i.issue_date,i.id),'[]'::jsonb) FROM payments p JOIN invoices i ON i.id=p.invoice_id AND i.legal_entity_id=p.legal_entity_id WHERE p.legal_entity_id=$1 AND p.received_at::date BETWEEN $2 AND $3 AND i.status NOT IN ('DRAFT','CANCELLED')").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)? } else { sqlx::query_scalar("SELECT COALESCE(jsonb_agg(id ORDER BY issue_date,id),'[]'::jsonb) FROM invoices WHERE legal_entity_id=$1 AND issue_date BETWEEN $2 AND $3 AND status NOT IN ('DRAFT','CANCELLED')").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)? };
        let deductible_source_ids:Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(id ORDER BY transaction_date,id),'[]'::jsonb) FROM vat_entries WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND entry_kind='DEDUCTIBLE'").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let correction_source_ids:Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(id ORDER BY transaction_date,id),'[]'::jsonb) FROM vat_entries WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND entry_kind='CORRECTION'").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let advance_source_ids:Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(id ORDER BY payment_date,id),'[]'::jsonb) FROM vat_advances WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND status IN ('DOCUMENTED','PAID')").bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let net=collected-deductible+corrections; let payable=net.max(0); let credit=(-net).max(0); let internal_balance=(net-advances).max(0);
        if basis.to_uppercase()=="COLLECTION" && collected==0 { anomalies.push(json!({"code":"NO_COLLECTION","message":"Aucune TVA collectée exigible par encaissement sur la période."})); }
        let id:Uuid=sqlx::query_scalar(r#"
            INSERT INTO vat_declarations(legal_entity_id,period_start,period_end,form_code,form_millesime,status,ca_ht_cents,collected_vat_cents,deductible_vat_cents,payable_vat_cents,credit_vat_cents,corrections_vat_cents,anomalies)
            VALUES($1,$2,$3,'3310-CA3',2026,'VALIDATION_REQUIRED',$4,$5,$6,$7,$8,$9,$10)
            ON CONFLICT(legal_entity_id,period_start,period_end,form_code) DO UPDATE SET status=CASE WHEN vat_declarations.status='FINAL' THEN 'FINAL' ELSE 'VALIDATION_REQUIRED' END,ca_ht_cents=EXCLUDED.ca_ht_cents,collected_vat_cents=EXCLUDED.collected_vat_cents,deductible_vat_cents=EXCLUDED.deductible_vat_cents,payable_vat_cents=EXCLUDED.payable_vat_cents,credit_vat_cents=EXCLUDED.credit_vat_cents,corrections_vat_cents=EXCLUDED.corrections_vat_cents,anomalies=EXCLUDED.anomalies,updated_at=now()
            RETURNING id
        "#).bind(entity).bind(period_start).bind(period_end).bind(ca_ht).bind(collected).bind(deductible).bind(payable).bind(credit).bind(corrections).bind(Value::Array(anomalies.clone())).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let fields=[
            ("CA_HT","Chiffre d'affaires HT",ca_ht,"PAYMENTS/INVOICES",collection_source_ids.clone(),"Paiements ou factures selon régime"),
            ("TVA_COLLECTEE","TVA collectée exigible",collected,"PAYMENTS/INVOICES",collection_source_ids.clone(),"Calcul TVA exigible"),
            ("TVA_DEDUCTIBLE","TVA déductible",deductible,"VAT_ENTRIES",deductible_source_ids,"Somme des entrées DEDUCTIBLE"),
            ("CORRECTIONS","Corrections TVA",corrections,"VAT_ENTRIES",correction_source_ids,"Somme des corrections"),
            ("TVA_NETTE","TVA nette",net,"CALCULATION",json!([]),"TVA collectée - TVA déductible + corrections"),
            ("TVA_A_PAYER","TVA à payer",payable,"CALCULATION",json!([]),"max(TVA nette,0)"),
            ("CREDIT_TVA","Crédit de TVA",credit,"CALCULATION",json!([]),"max(-TVA nette,0)"),
            ("ACOMPTES_TVA","Acomptes TVA documentés",advances,"VAT_ADVANCES",advance_source_ids,"Somme des acomptes documentés/payés de la période"),
            ("SOLDE_INTERNE_APRES_ACOMPTES","Solde interne après acomptes",internal_balance,"CALCULATION",json!([]),"max(TVA nette - acomptes documentés,0); vérification du régime/formulaire requise"),
        ];
        for (code,label,value,source,source_ids,formula) in fields {
            sqlx::query("INSERT INTO vat_declaration_fields(legal_entity_id,declaration_id,field_code,label,value_cents,source_type,source_ids,formula,validation_status,validation_message) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT(legal_entity_id,declaration_id,field_code) DO UPDATE SET value_cents=EXCLUDED.value_cents,source_type=EXCLUDED.source_type,source_ids=EXCLUDED.source_ids,formula=EXCLUDED.formula,validation_status=EXCLUDED.validation_status,validation_message=EXCLUDED.validation_message")
                .bind(entity).bind(id).bind(code).bind(label).bind(value).bind(source).bind(source_ids).bind(formula).bind(if code=="TVA_NETTE"{"WARNING"}else{"OK"}).bind(if code=="TVA_NETTE"{"Vérification humaine recommandée"}else{"Calcul déterministe"}).execute(pool).await.map_err(ServerFnError::new)?;
        }
        Ok(VatDeclarationItem{id,period_start,period_end,form_code:"3310-CA3".into(),status:"VALIDATION_REQUIRED".into(),ca_ht_cents:ca_ht,collected_vat_cents:collected,deductible_vat_cents:deductible,payable_vat_cents:payable,credit_vat_cents:credit,corrections_vat_cents:corrections,anomalies:Value::Array(anomalies)})
    }
    #[cfg(not(feature = "server"))]
    { let _=(period_start,period_end); Err(ServerFnError::new("calculate_vat_declaration est exécutée côté serveur")) }
}

#[server]
pub async fn request_vat_validation(declaration_id:Uuid) -> Result<Uuid, ServerFnError> {
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let (workspace, status):(Uuid,String)=sqlx::query_as("SELECT e.workspace_id,d.status FROM legal_entities e JOIN vat_declarations d ON d.legal_entity_id=e.id WHERE e.id=$1 AND d.id=$2").bind(entity).bind(declaration_id).fetch_one(pool).await.map_err(ServerFnError::new)?; if status=="FINAL" {return Err(ServerFnError::new("Déclaration déjà finale"));} let payload:Value=sqlx::query_scalar("SELECT jsonb_build_object('period_start',period_start,'period_end',period_end,'ca_ht_cents',ca_ht_cents,'collected_vat_cents',collected_vat_cents,'deductible_vat_cents',deductible_vat_cents,'payable_vat_cents',payable_vat_cents,'credit_vat_cents',credit_vat_cents) FROM vat_declarations WHERE id=$1 AND legal_entity_id=$2").bind(declaration_id).bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?; let req:Uuid=sqlx::query_scalar("INSERT INTO validation_requests(workspace_id,legal_entity_id,action_code,subject_type,subject_id,proposed_payload,reason,source_payload,documents_payload,expected_result,risk_level) VALUES($1,$2,'VAT_DECLARATION_VALIDATE','VAT_DECLARATION',$3,$4,'Validation humaine requise avant déclaration finale','[\"vat_declarations\"]'::jsonb,'[]'::jsonb,'{\"status\":\"FINAL\"}'::jsonb,'HIGH') RETURNING id").bind(workspace).bind(entity).bind(declaration_id).bind(payload).fetch_one(pool).await.map_err(ServerFnError::new)?; sqlx::query("UPDATE vat_declarations SET validation_request_id=$1,status='VALIDATION_REQUIRED',updated_at=now() WHERE id=$2 AND legal_entity_id=$3").bind(req).bind(declaration_id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?; Ok(req) }
    #[cfg(not(feature="server"))] { let _=declaration_id; Err(ServerFnError::new("request_vat_validation est exécutée côté serveur")) }
}

#[server]
pub async fn finalize_vat_declaration(declaration_id:Uuid) -> Result<(),ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); let approved:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM validation_requests v JOIN vat_declarations d ON d.validation_request_id=v.id WHERE d.id=$1 AND d.legal_entity_id=$2 AND v.status='APPROVED')").bind(declaration_id).bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?; if !approved{return Err(ServerFnError::new("Validation humaine requise avant finalisation"));} sqlx::query("UPDATE vat_declarations SET status='FINAL',validated_at=now(),validated_by='MANAGER',updated_at=now() WHERE id=$1 AND legal_entity_id=$2 AND status<>'FINAL'").bind(declaration_id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?; Ok(()) }
    #[cfg(not(feature="server"))] { let _=declaration_id; Err(ServerFnError::new("finalize_vat_declaration est exécutée côté serveur")) }
}

#[server]
pub async fn calculate_fiscal_result(period_start:NaiveDate,period_end:NaiveDate)->Result<FiscalResultItem,ServerFnError>{
    #[cfg(feature="server")]
    { if period_end<period_start{return Err(ServerFnError::new("Période fiscale invalide"));} let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id(); use sqlx::Row;
        let issued:i64=sqlx::query_scalar("SELECT COALESCE(SUM(CASE WHEN document_kind='CREDIT_NOTE' THEN -net_cents ELSE net_cents END),0)::bigint FROM invoices WHERE legal_entity_id=$1 AND issue_date BETWEEN $2 AND $3 AND status NOT IN ('DRAFT','CANCELLED')").bind(e).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let treasury:i64=sqlx::query_scalar("SELECT COALESCE(SUM(CASE WHEN direction='IN' THEN amount_cents ELSE -amount_cents END),0)::bigint FROM financial_transactions WHERE legal_entity_id=$1 AND occurred_at::date BETWEEN $2 AND $3 AND status<>'CANCELLED'").bind(e).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let outflows:i64=sqlx::query_scalar("SELECT COALESCE(SUM(amount_cents),0)::bigint FROM financial_transactions WHERE legal_entity_id=$1 AND direction='OUT' AND occurred_at::date BETWEEN $2 AND $3 AND status<>'CANCELLED'").bind(e).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let deductions:i64=sqlx::query_scalar("SELECT COALESCE(SUM(vat_cents),0)::bigint FROM vat_entries WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND entry_kind='DEDUCTIBLE'").bind(e).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let fiscal=issued-outflows; let anomalies=json!([{"code":"CLASSIFICATION","message":"Les sorties de trésorerie non qualifiées restent comptées dans la vue fiscale provisoire; une qualification comptable/fiscale détaillée peut être nécessaire."}]);
        sqlx::query(r#"INSERT INTO fiscal_results(legal_entity_id,period_start,period_end,management_income_cents,accounting_result_cents,fiscal_result_cents,treasury_result_cents,deductions_cents,reintegrations_cents,anomalies,calculation_snapshot) VALUES($1,$2,$3,$4,$5,$6,$7,$8,0,$9,$10) ON CONFLICT(legal_entity_id,period_start,period_end) DO UPDATE SET management_income_cents=EXCLUDED.management_income_cents,accounting_result_cents=EXCLUDED.accounting_result_cents,fiscal_result_cents=EXCLUDED.fiscal_result_cents,treasury_result_cents=EXCLUDED.treasury_result_cents,deductions_cents=EXCLUDED.deductions_cents,anomalies=EXCLUDED.anomalies,calculation_snapshot=EXCLUDED.calculation_snapshot"#).bind(e).bind(period_start).bind(period_end).bind(issued).bind(fiscal).bind(fiscal).bind(treasury).bind(deductions).bind(anomalies.clone()).bind(json!({"management":"factures émises HT","accounting":"HT facturé - sorties","fiscal":"vue provisoire à qualifier","treasury":"IN-OUT"})).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(FiscalResultItem{period_start,period_end,management_income_cents:issued,accounting_result_cents:fiscal,fiscal_result_cents:fiscal,treasury_result_cents:treasury,deductions_cents:deductions,reintegrations_cents:0,anomalies}) }
    #[cfg(not(feature="server"))] { let _=(period_start,period_end); Err(ServerFnError::new("calculate_fiscal_result est exécutée côté serveur")) }
}

#[server]
pub async fn generate_2072(tax_year:i32)->Result<Fiscal2072Item,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id(); use sqlx::Row; let form=sqlx::query("SELECT id,form_code,millesime,tax_year FROM fiscal_form_versions WHERE legal_entity_id=$1 AND form_code='2072-S' AND tax_year=$2 ORDER BY millesime DESC LIMIT 1").bind(e).bind(tax_year).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Aucune définition 2072-S versionnée pour l'entité"))?; let form_id:Uuid=form.get("id"); let form_code:String=form.get("form_code"); let form_millesime:i32=form.get("millesime");
        let start=NaiveDate::from_ymd_opt(tax_year,1,1).ok_or_else(||ServerFnError::new("Année invalide"))?; let end=NaiveDate::from_ymd_opt(tax_year,12,31).unwrap();
        let rents:i64=sqlx::query_scalar("SELECT COALESCE(SUM(CASE WHEN i.document_kind='CREDIT_NOTE' THEN -p.amount_cents ELSE p.amount_cents END),0)::bigint FROM payments p JOIN invoices i ON i.id=p.invoice_id AND i.legal_entity_id=p.legal_entity_id WHERE p.legal_entity_id=$1 AND p.received_at::date BETWEEN $2 AND $3 AND i.status NOT IN ('DRAFT','CANCELLED')").bind(e).bind(start).bind(end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let charges:i64=sqlx::query_scalar("SELECT COALESCE(SUM(amount_cents),0)::bigint FROM financial_transactions WHERE legal_entity_id=$1 AND direction='OUT' AND occurred_at::date BETWEEN $2 AND $3 AND status<>'CANCELLED'").bind(e).bind(start).bind(end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let net=rents-charges; let mut anomalies=Vec::<Value>::new(); if rents==0 {anomalies.push(json!({"code":"NO_RENT_RECEIPTS","message":"Aucun encaissement de loyer rattaché à une facture sur l'année."}));}
        let a1=json!({"1":rents,"2":0,"3":0,"4":0,"5":rents,"6":0,"7":0,"8":0,"9":0,"9bis":0,"10":0,"11":0,"12":charges,"13":0,"14":0,"15":0,"16":charges,"17":0,"18":net,"19":0,"20":0,"21":net,"22":0,"23":net});
        let data=json!({"tax_year":tax_year,"millesime":form_millesime,"form":"2072-S","R1":rents,"R2":0,"R3":charges,"R4":0,"R5":net,"A1":a1,"mapping_note":"Valeurs issues des flux présents; les lignes détaillées nécessitant une qualification spécifique restent à valider.","source":{"rent":"payments→invoices","charges":"financial_transactions"}});
        let anomalies_v=Value::Array(anomalies); let id:Uuid=sqlx::query_scalar("INSERT INTO fiscal_2072_runs(legal_entity_id,tax_year,form_version_id,status,data,anomalies) VALUES($1,$2,$3,'VALIDATION_REQUIRED',$4,$5) ON CONFLICT(legal_entity_id,tax_year,form_version_id) DO UPDATE SET data=EXCLUDED.data,anomalies=EXCLUDED.anomalies,status=CASE WHEN fiscal_2072_runs.status='FINAL' THEN 'FINAL' ELSE 'VALIDATION_REQUIRED' END,updated_at=now() RETURNING id").bind(e).bind(tax_year).bind(form_id).bind(data.clone()).bind(anomalies_v.clone()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        // Quote-parts: uses explicit share history rows covering each part of the year.
        sqlx::query("DELETE FROM fiscal_2072_allocations WHERE legal_entity_id=$1 AND run_id=$2").bind(e).bind(id).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(Fiscal2072Item{id,tax_year,form_code,status:"VALIDATION_REQUIRED".into(),data,anomalies:anomalies_v}) }
    #[cfg(not(feature="server"))] { let _=tax_year; Err(ServerFnError::new("generate_2072 est exécutée côté serveur")) }
}

#[server]
pub async fn calculate_2072_quote_parts(run_id:Uuid)->Result<Vec<QuotePartItem>,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id(); use sqlx::Row; let (tax_year,net):(i32,i64)=sqlx::query_as("SELECT tax_year,(data->>'R5')::bigint FROM fiscal_2072_runs WHERE id=$1 AND legal_entity_id=$2").bind(run_id).bind(e).fetch_one(pool).await.map_err(ServerFnError::new)?; let start=NaiveDate::from_ymd_opt(tax_year,1,1).unwrap(); let end=NaiveDate::from_ymd_opt(tax_year,12,31).unwrap(); let rows=sqlx::query("SELECT h.associate_id,a.display_name,h.ownership_pct,h.effective_from,COALESCE(h.effective_to,$3) AS effective_to FROM associate_share_history h JOIN associates a ON a.id=h.associate_id WHERE h.legal_entity_id=$1 AND h.effective_from <= $3 AND COALESCE(h.effective_to,$3) >= $2 ORDER BY a.display_name,h.effective_from").bind(e).bind(start).bind(end).fetch_all(pool).await.map_err(ServerFnError::new)?; let days_total=(end-start).num_days()+1; let mut out=Vec::new(); for r in rows { let s:chrono::NaiveDate=r.get("effective_from"); let ee:chrono::NaiveDate=r.get("effective_to"); let overlap_s=s.max(start); let overlap_e=ee.min(end); if overlap_e<overlap_s{continue;} let days=(overlap_e-overlap_s).num_days()+1; let pct:Decimal=r.get("ownership_pct"); let amount=((Decimal::from(net)*pct*Decimal::from(days))/(Decimal::from(100u32)*Decimal::from(days_total))).round_dp(0).to_i64().unwrap_or(0); let formula=format!("{} € × {} % × {}/{} jours",euro(net),pct,days,days_total); let assoc=r.get("associate_id"); sqlx::query("INSERT INTO fiscal_2072_allocations(legal_entity_id,run_id,associate_id,ownership_pct,amount_cents,formula,coverage_start,coverage_end) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(legal_entity_id,run_id,associate_id,coverage_start,coverage_end) DO UPDATE SET ownership_pct=EXCLUDED.ownership_pct,amount_cents=EXCLUDED.amount_cents,formula=EXCLUDED.formula").bind(e).bind(run_id).bind(assoc).bind(pct).bind(amount).bind(&formula).bind(overlap_s).bind(overlap_e).execute(pool).await.map_err(ServerFnError::new)?; out.push(QuotePartItem{associate_id:assoc,associate_name:r.get("display_name"),ownership_pct:pct,amount_cents:amount,formula}); } Ok(out) }
    #[cfg(not(feature="server"))] { let _=run_id; Err(ServerFnError::new("calculate_2072_quote_parts est exécutée côté serveur")) }
}

#[server]
pub async fn simulate_tax(label:String,taxable_base_cents:i64,rate_bp:i32)->Result<Value,ServerFnError>{
    #[cfg(feature="server")]
    { if taxable_base_cents<0||!(0..=10000).contains(&rate_bp){return Err(ServerFnError::new("Simulation fiscale invalide"));} let result=((Decimal::from(taxable_base_cents)*Decimal::from(rate_bp))/Decimal::from(10000u32)).round_dp(0).to_i64().unwrap_or(0); let payload=json!({"status":"SIMULATION","label":label.trim(),"base_cents":taxable_base_cents,"rate_bp":rate_bp,"estimated_tax_cents":result,"not_declarative":true}); let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; sqlx::query("INSERT INTO fiscal_simulations(legal_entity_id,simulation_code,label,as_of_date,inputs,result) VALUES($1,'GENERIC_TAX','SIMULATION: '||$2,CURRENT_DATE,$3,$4)").bind(current_legal_entity_id()).bind(label.trim()).bind(json!({"base_cents":taxable_base_cents,"rate_bp":rate_bp})).bind(payload.clone()).execute(pool).await.map_err(ServerFnError::new)?; Ok(payload) }
    #[cfg(not(feature="server"))] { let _=(label,taxable_base_cents,rate_bp); Err(ServerFnError::new("simulate_tax est exécutée côté serveur")) }
}

#[server]
pub async fn add_tax_obligation(tax_code:String,label:String,period_start:NaiveDate,period_end:NaiveDate,applicability:String,qualification_needed:bool,due_date:Option<NaiveDate>,amount_cents:Option<i64>,source_reference:String,notes:String)->Result<Uuid,ServerFnError>{
    #[cfg(feature="server")]
    { if period_end<period_start||tax_code.trim().is_empty()||label.trim().is_empty(){return Err(ServerFnError::new("Échéance fiscale invalide"));} let app=applicability.trim().to_uppercase(); if !matches!(app.as_str(),"APPLICABLE"|"POTENTIALLY_APPLICABLE"|"NOT_APPLICABLE"|"TO_QUALIFY"){return Err(ServerFnError::new("Qualification fiscale invalide"));} let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let id:Uuid=sqlx::query_scalar("INSERT INTO fiscal_tax_obligations(legal_entity_id,tax_code,label,period_start,period_end,applicability,qualification_needed,due_date,amount_cents,source_reference,notes) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id").bind(current_legal_entity_id()).bind(tax_code.trim().to_uppercase()).bind(label.trim()).bind(period_start).bind(period_end).bind(app).bind(qualification_needed).bind(due_date).bind(amount_cents).bind(source_reference.trim()).bind(notes.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?; Ok(id) }
    #[cfg(not(feature="server"))] { let _=(tax_code,label,period_start,period_end,applicability,qualification_needed,due_date,amount_cents,source_reference,notes); Err(ServerFnError::new("add_tax_obligation est exécutée côté serveur")) }
}

#[server]
pub async fn list_vat_declaration_fields(declaration_id:Uuid)->Result<Vec<VatDeclarationFieldItem>,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT field_code,label,value_cents,value_text,source_type,source_ids,formula,validation_status,validation_message FROM vat_declaration_fields WHERE legal_entity_id=$1 AND declaration_id=$2 ORDER BY field_code").bind(current_legal_entity_id()).bind(declaration_id).fetch_all(pool).await.map_err(ServerFnError::new)?; use sqlx::Row; Ok(rows.into_iter().map(|r|VatDeclarationFieldItem{field_code:r.get("field_code"),label:r.get("label"),value_cents:r.get("value_cents"),value_text:r.get("value_text"),source_type:r.get("source_type"),source_ids:r.get("source_ids"),formula:r.get("formula"),validation_status:r.get("validation_status"),validation_message:r.get("validation_message")}).collect()) }
    #[cfg(not(feature="server"))] { let _=declaration_id; Err(ServerFnError::new("list_vat_declaration_fields est exécutée côté serveur")) }
}

#[server]
pub async fn list_2072_runs()->Result<Vec<Fiscal2072Item>,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT id,tax_year,'2072-S' AS form_code,status,data,anomalies FROM fiscal_2072_runs WHERE legal_entity_id=$1 ORDER BY tax_year DESC,updated_at DESC LIMIT 20").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; use sqlx::Row; Ok(rows.into_iter().map(|r|Fiscal2072Item{id:r.get("id"),tax_year:r.get("tax_year"),form_code:r.get("form_code"),status:r.get("status"),data:r.get("data"),anomalies:r.get("anomalies")}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_2072_runs est exécutée côté serveur"))
}

#[server]
pub async fn build_fiscal_dossier(run_id:Uuid)->Result<FiscalDossierItem,ServerFnError>{
    #[cfg(feature="server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let e=current_legal_entity_id(); use sqlx::Row;
        let run=sqlx::query("SELECT tax_year,data,status FROM fiscal_2072_runs WHERE id=$1 AND legal_entity_id=$2").bind(run_id).bind(e).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let tax_year:i32=run.get("tax_year"); let data:Value=run.get("data"); let status:String=run.get("status");
        let quote=calculate_2072_quote_parts(run_id).await?; let quote_json=serde_json::to_value(&quote).map_err(ServerFnError::new)?;
        let docs:Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(id ORDER BY created_at), '[]'::jsonb) FROM documents WHERE legal_entity_id=$1 AND EXTRACT(YEAR FROM created_at)::int IN ($2,$3)").bind(e).bind(tax_year).bind(tax_year+1).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let history=json!([{"event":"2072_GENERATED","date":Utc::now().date_naive(),"run_id":run_id},{"event":"QUOTE_PARTS_CALCULATED","count":quote.len()}]);
        let revenues=data.get("R1").and_then(Value::as_i64).unwrap_or(0); let charges=data.get("R3").and_then(Value::as_i64).unwrap_or(0); let result=data.get("R5").and_then(Value::as_i64).unwrap_or(revenues-charges);
        let id:Uuid=sqlx::query_scalar("INSERT INTO fiscal_dossiers(legal_entity_id,run_id,tax_year,quote_parts,revenues_cents,charges_cents,result_cents,document_ids,history,status) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT(legal_entity_id,tax_year,run_id) DO UPDATE SET quote_parts=EXCLUDED.quote_parts,revenues_cents=EXCLUDED.revenues_cents,charges_cents=EXCLUDED.charges_cents,result_cents=EXCLUDED.result_cents,document_ids=EXCLUDED.document_ids,history=EXCLUDED.history,status=EXCLUDED.status,updated_at=now() RETURNING id").bind(e).bind(run_id).bind(tax_year).bind(&quote_json).bind(revenues).bind(charges).bind(result).bind(&docs).bind(&history).bind(&status).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(FiscalDossierItem{id,run_id,tax_year,revenues_cents:revenues,charges_cents:charges,result_cents:result,quote_parts:quote_json,document_ids:docs,history,status})
    }
    #[cfg(not(feature="server"))] { let _=run_id; Err(ServerFnError::new("build_fiscal_dossier est exécutée côté serveur")) }
}

#[server]
pub async fn list_fiscal_dossiers()->Result<Vec<FiscalDossierItem>,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; use sqlx::Row; let rows=sqlx::query("SELECT id,run_id,tax_year,revenues_cents,charges_cents,result_cents,quote_parts,document_ids,history,status FROM fiscal_dossiers WHERE legal_entity_id=$1 ORDER BY tax_year DESC,updated_at DESC LIMIT 20").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r|FiscalDossierItem{id:r.get("id"),run_id:r.get("run_id"),tax_year:r.get("tax_year"),revenues_cents:r.get("revenues_cents"),charges_cents:r.get("charges_cents"),result_cents:r.get("result_cents"),quote_parts:r.get("quote_parts"),document_ids:r.get("document_ids"),history:r.get("history"),status:r.get("status")}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_fiscal_dossiers est exécutée côté serveur"))
}

#[server]
pub async fn list_vat_declarations()->Result<Vec<VatDeclarationItem>,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT id,period_start,period_end,form_code,status,ca_ht_cents,collected_vat_cents,deductible_vat_cents,payable_vat_cents,credit_vat_cents,corrections_vat_cents,anomalies FROM vat_declarations WHERE legal_entity_id=$1 ORDER BY period_end DESC").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; use sqlx::Row; Ok(rows.into_iter().map(|r|VatDeclarationItem{id:r.get("id"),period_start:r.get("period_start"),period_end:r.get("period_end"),form_code:r.get("form_code"),status:r.get("status"),ca_ht_cents:r.get("ca_ht_cents"),collected_vat_cents:r.get("collected_vat_cents"),deductible_vat_cents:r.get("deductible_vat_cents"),payable_vat_cents:r.get("payable_vat_cents"),credit_vat_cents:r.get("credit_vat_cents"),corrections_vat_cents:r.get("corrections_vat_cents"),anomalies:r.get("anomalies")}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_vat_declarations est exécutée côté serveur"))
}

#[server]
pub async fn list_tax_obligations()->Result<Vec<TaxObligationItem>,ServerFnError>{
    #[cfg(feature="server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let rows=sqlx::query("SELECT id,tax_code,label,period_start,period_end,applicability,qualification_needed,due_date,amount_cents,notes FROM fiscal_tax_obligations WHERE legal_entity_id=$1 ORDER BY due_date NULLS LAST,period_end DESC").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; use sqlx::Row; Ok(rows.into_iter().map(|r|TaxObligationItem{id:r.get("id"),tax_code:r.get("tax_code"),label:r.get("label"),period_start:r.get("period_start"),period_end:r.get("period_end"),applicability:r.get("applicability"),qualification_needed:r.get("qualification_needed"),due_date:r.get("due_date"),amount_cents:r.get("amount_cents"),notes:r.get("notes")}).collect()) }
    #[cfg(not(feature="server"))] Err(ServerFnError::new("list_tax_obligations est exécutée côté serveur"))
}

#[component]
pub fn FiscalPage(mut refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let decls = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_vat_declarations().await.unwrap_or_default() }
    });
    let fields = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move {
            match decls.read().as_deref().and_then(|items| items.first()) {
                Some(declaration) => list_vat_declaration_fields(declaration.id).await.unwrap_or_default(),
                None => Vec::new(),
            }
        }
    });
    let runs = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_2072_runs().await.unwrap_or_default() }
    });
    let dossiers = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_fiscal_dossiers().await.unwrap_or_default() }
    });
    let obligations = use_resource(move || {
        let _ = refresh();
        async move { list_tax_obligations().await.unwrap_or_default() }
    });

    let mut p1 = use_signal(|| Utc::now().date_naive().with_day(1).unwrap().to_string());
    let mut p2 = use_signal(|| month_end(Utc::now().date_naive().with_day(1).unwrap()).to_string());
    let mut tax_year = use_signal(|| (Utc::now().year() - 1).to_string());
    let mut msg = use_signal(String::new);
    let mut sim_base = use_signal(String::new);
    let mut sim_rate = use_signal(|| "20".into());
    let mut tax_label = use_signal(String::new);

    rsx! {
        ModuleHeader {
            title: "Fiscalité",
            kicker: "TVA • 2072 • RÉSULTAT • QUOTE-PARTS • TAXES",
            detail: "Calculs sourcés, formulaires versionnés et simulations explicitement séparées des données déclaratives."
        }

        section {
            class: "two-col",

            div {
                class: "panel",
                h3 { "TVA — déclaration préparatoire" }
                FormField {
                    label: "Début",
                    value: p1(),
                    oninput: move |event: FormEvent| p1.set(event.value())
                }
                FormField {
                    label: "Fin",
                    value: p2(),
                    oninput: move |event: FormEvent| p2.set(event.value())
                }
                div {
                    class: "button-row",
                    button {
                        class: "primary",
                        onclick: move |_| {
                            let start = NaiveDate::parse_from_str(&p1(), "%Y-%m-%d");
                            let end = NaiveDate::parse_from_str(&p2(), "%Y-%m-%d");
                            async move {
                                match (start, end) {
                                    (Ok(start), Ok(end)) => match calculate_vat_declaration(start, end).await {
                                        Ok(result) => {
                                            msg.set(format!(
                                                "TVA {} — à payer {} / crédit {}",
                                                vat_status_fr(&result.status),
                                                euro(result.payable_vat_cents),
                                                euro(result.credit_vat_cents)
                                            ));
                                            bump += 1;
                                        }
                                        Err(error) => msg.set(error.to_string()),
                                    },
                                    _ => msg.set("Période invalide".into()),
                                }
                            }
                        },
                        "Préparer"
                    }
                    button {
                        class: "secondary",
                        onclick: move |_| async move {
                            if let Some(declaration) = decls.read().as_deref().and_then(|items| items.first()) {
                                match request_vat_validation(declaration.id).await {
                                    Ok(_) => msg.set("Validation humaine demandée".into()),
                                    Err(error) => msg.set(error.to_string()),
                                }
                            }
                        },
                        "Demander validation"
                    }
                    button {
                        class: "secondary",
                        onclick: move |_| async move {
                            if let Some(declaration) = decls.read().as_deref().and_then(|items| items.first()) {
                                match finalize_vat_declaration(declaration.id).await {
                                    Ok(_) => {
                                        msg.set("Déclaration finalisée après validation".into());
                                        bump += 1;
                                    }
                                    Err(error) => msg.set(error.to_string()),
                                }
                            }
                        },
                        "Finaliser"
                    }
                }
            }

            div {
                class: "panel",
                h3 { "Simulation fiscale — séparée" }
                FormField {
                    label: "Libellé",
                    value: tax_label(),
                    oninput: move |event: FormEvent| tax_label.set(event.value())
                }
                FormField {
                    label: "Base €",
                    value: sim_base(),
                    oninput: move |event: FormEvent| sim_base.set(event.value())
                }
                FormField {
                    label: "Taux %",
                    value: sim_rate(),
                    oninput: move |event: FormEvent| sim_rate.set(event.value())
                }
                button {
                    class: "secondary",
                    onclick: move |_| async move {
                        let base = sim_base()
                            .replace(',', ".")
                            .parse::<f64>()
                            .ok()
                            .map(|value| (value * 100.0).round() as i64);
                        let rate = sim_rate()
                            .replace(',', ".")
                            .parse::<f64>()
                            .ok()
                            .map(|value| (value * 100.0).round() as i32);
                        match (base, rate) {
                            (Some(base), Some(rate)) => match simulate_tax(tax_label(), base, rate).await {
                                Ok(value) => msg.set(format!(
                                    "{} — estimé {}",
                                    value.get("status").and_then(Value::as_str).unwrap_or("SIMULATION"),
                                    euro(value.get("estimated_tax_cents").and_then(Value::as_i64).unwrap_or(0))
                                )),
                                Err(error) => msg.set(error.to_string()),
                            },
                            _ => msg.set("Simulation invalide".into()),
                        }
                    },
                    "Simuler — SIMULATION"
                }
            }
        }

        span {
            class: "save-ok",
            "{msg()}"
        }

        section {
            class: "panel",
            h3 { "Champs TVA — valeur / source / pièces / formule / validation" }
            for field in fields.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        class: "data-title",
                        "{field.field_code} • {field.label}"
                    }
                    div {
                        class: "small",
                        {format!(
                            "Valeur {} • source {} • pièces {}",
                            field.value_cents
                                .map(euro)
                                .or_else(|| field.value_text.clone())
                                .unwrap_or_else(|| "—".into()),
                            field.source_type,
                            field.source_ids
                        )}
                    }
                    div {
                        class: "small",
                        {format!(
                            "Formule : {} • validation : {} — {}",
                            field.formula,
                            field.validation_status,
                            field.validation_message
                        )}
                    }
                }
            }
        }

        section {
            class: "two-col",
            div {
                class: "panel",
                h3 { "Résultat fiscal / gestion / trésorerie" }
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let start = NaiveDate::parse_from_str(&p1(), "%Y-%m-%d");
                        let end = NaiveDate::parse_from_str(&p2(), "%Y-%m-%d");
                        async move {
                            match (start, end) {
                                (Ok(start), Ok(end)) => match calculate_fiscal_result(start, end).await {
                                    Ok(result) => msg.set(format!(
                                        "Gestion {} • comptabilité {} • fiscalité {} • trésorerie {}",
                                        euro(result.management_income_cents),
                                        euro(result.accounting_result_cents),
                                        euro(result.fiscal_result_cents),
                                        euro(result.treasury_result_cents)
                                    )),
                                    Err(error) => msg.set(error.to_string()),
                                },
                                _ => msg.set("Période invalide".into()),
                            }
                        }
                    },
                    "Calculer la vue"
                }
            }

            div {
                class: "panel",
                h3 { "2072 — définition versionnée" }
                FormField {
                    label: "Année fiscale",
                    value: tax_year(),
                    oninput: move |event: FormEvent| tax_year.set(event.value())
                }
                button {
                    class: "primary",
                    onclick: move |_| async move {
                        match tax_year().parse::<i32>() {
                            Ok(year) => match generate_2072(year).await {
                                Ok(result) => {
                                    let count = calculate_2072_quote_parts(result.id).await.unwrap_or_default().len();
                                    let _ = build_fiscal_dossier(result.id).await;
                                    msg.set(format!(
                                        "2072 {} générée — {} quote-part(s) calculée(s)",
                                        result.status,
                                        count
                                    ));
                                    bump += 1;
                                }
                                Err(error) => msg.set(error.to_string()),
                            },
                            Err(_) => msg.set("Année invalide".into()),
                        }
                    },
                    "Générer 2072"
                }
                for run in runs.read().as_deref().unwrap_or(&[]).iter() {
                    div {
                        class: "data-row",
                        div {
                            class: "data-title",
                            "{run.tax_year} • {run.form_code} • {run.status}"
                        }
                        div {
                            class: "small",
                            {format!(
                                "R5 résultat : {}",
                                run.data
                                    .get("R5")
                                    .and_then(Value::as_i64)
                                    .map(euro)
                                    .unwrap_or_else(|| "—".into())
                            )}
                        }
                    }
                }
            }
        }

        section {
            class: "panel",
            h3 { "Dossiers fiscaux associés" }
            for dossier in dossiers.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        class: "data-title",
                        "2072 {dossier.tax_year} • {dossier.status}"
                    }
                    div {
                        class: "small",
                        {format!(
                            "Revenus {} • charges {} • résultat {}",
                            euro(dossier.revenues_cents),
                            euro(dossier.charges_cents),
                            euro(dossier.result_cents)
                        )}
                    }
                    div {
                        class: "small",
                        {format!(
                            "{} quote-part(s) • {} document(s) • historique {}",
                            dossier.quote_parts.as_array().map(|items| items.len()).unwrap_or(0),
                            dossier.document_ids.as_array().map(|items| items.len()).unwrap_or(0),
                            dossier.history.as_array().map(|items| items.len()).unwrap_or(0)
                        )}
                    }
                }
            }
        }

        section {
            class: "panel",
            h3 { "Taxes et échéances" }
            for tax in obligations.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        class: "data-title",
                        "{tax.tax_code} • {tax.label}"
                    }
                    div {
                        class: "small",
                        {format!(
                            "{} → {} • statut : {}{}",
                            tax.period_start,
                            tax.period_end,
                            tax.applicability,
                            if tax.qualification_needed { " • qualification requise" } else { "" }
                        )}
                    }
                    div {
                        class: "small",
                        {format!(
                            "Échéance : {} • {}",
                            tax.due_date.map(|date| date.to_string()).unwrap_or_else(|| "—".into()),
                            tax.notes
                        )}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn basis_rate_rounding_is_stable() {
        let base=Decimal::new(12345,2); let rate=Decimal::new(20,2);
        let vat=(base*rate).round_dp(2);
        assert_eq!(vat,Decimal::new(2469,2));
    }

    use super::*;
    #[test]
    fn vat_french_statuses_are_stable(){assert_eq!(vat_status_fr("DRAFT"),"Brouillon");assert_eq!(vat_status_fr("VALIDATION_REQUIRED"),"Validation requise");assert_eq!(vat_status_fr("FINAL"),"Finale");}
    #[test]
    fn month_end_handles_february(){let d=NaiveDate::from_ymd_opt(2026,2,1).unwrap();assert_eq!(month_end(d),NaiveDate::from_ymd_opt(2026,2,28).unwrap());}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VatCalculationPreview {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub ca_ht_cents: i64,
    pub collected_vat_cents: i64,
    pub deductible_vat_cents: i64,
    pub corrections_vat_cents: i64,
    pub payable_vat_cents: i64,
    pub credit_vat_cents: i64,
    pub basis_note: String,
}

/// Vue de calcul TVA sans créer ni modifier de déclaration.
#[server]
pub async fn calculate_vat_preview(period_start: NaiveDate, period_end: NaiveDate) -> Result<VatCalculationPreview, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if period_end < period_start { return Err(ServerFnError::new("Période TVA invalide")); }
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let ca: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(taxable_net_cents),0)::bigint FROM vat_entries WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND entry_kind='COLLECTED' AND exigibility_basis IN ('COLLECTION','ADJUSTMENT')")
            .bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let collected: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(vat_cents),0)::bigint FROM vat_entries WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND entry_kind='COLLECTED' AND exigibility_basis IN ('COLLECTION','ADJUSTMENT')")
            .bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let deductible: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(vat_cents),0)::bigint FROM vat_entries WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND entry_kind='DEDUCTIBLE'")
            .bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let corrections: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(vat_cents),0)::bigint FROM vat_entries WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND entry_kind='CORRECTION'")
            .bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let advances: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(vat_cents),0)::bigint FROM vat_advances WHERE legal_entity_id=$1 AND period_start=$2 AND period_end=$3 AND status<>'CANCELLED'")
            .bind(entity).bind(period_start).bind(period_end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let net = collected + corrections - deductible - advances;
        Ok(VatCalculationPreview { period_start, period_end, ca_ht_cents: ca, collected_vat_cents: collected, deductible_vat_cents: deductible, corrections_vat_cents: corrections, payable_vat_cents: net.max(0), credit_vat_cents: (-net).max(0), basis_note: "Calcul fondé sur les mouvements TVA enregistrés et les acomptes documentés; aucune déclaration n'est modifiée.".into() })
    }
    #[cfg(not(feature = "server"))]
    { let _=(period_start,period_end); Err(ServerFnError::new("calculate_vat_preview est exécutée côté serveur")) }
}
