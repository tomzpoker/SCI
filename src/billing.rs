use crate::domain::*;
use crate::entity_scope::current_legal_entity_id;
use crate::ui::{FormField, InfoTileOwned, ModuleHeader};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use dioxus::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BillingInvoiceItem {
    pub id: Uuid,
    pub lease_id: Option<Uuid>,
    pub invoice_number: String,
    pub document_kind: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub net_cents: i64,
    pub vat_cents: i64,
    pub gross_cents: i64,
    pub paid_cents: i64,
    pub status_code: String,
    pub status_fr: String,
    pub tenant_name: String,
    pub pdf_path: String,
    pub pdf_draft_watermark: bool,
    pub electronic_status: String,
    pub electronic_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BillingRunResult {
    pub generated: i64,
    pub already_present: i64,
    pub skipped: i64,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegularizationResult {
    pub id: Uuid,
    pub lease_id: Uuid,
    pub charge_code: String,
    pub provision_cents: i64,
    pub actual_cents: i64,
    pub difference_cents: i64,
    pub treatment: String,
    pub invoice_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EInvoiceConfigItem {
    pub id: Uuid,
    pub provider_code: String,
    pub provider_name: String,
    pub format_code: String,
    pub endpoint_reference: String,
    pub active: bool,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EInvoicePrepareResult {
    pub invoice_id: Uuid,
    pub provider_name: String,
    pub format_code: String,
    pub payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoverableTaxItem {
    pub id: Uuid,
    pub lease_id: Uuid,
    pub tax_code: String,
    pub label: String,
    pub legal_basis: String,
    pub recovery_rate_bp: i32,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub active: bool,
}

pub fn month_bounds(month: NaiveDate) -> (NaiveDate, NaiveDate) {
    let start = NaiveDate::from_ymd_opt(month.year(), month.month(), 1).expect("date");
    let next = if month.month() == 12 {
        NaiveDate::from_ymd_opt(month.year() + 1, 1, 1).expect("date")
    } else {
        NaiveDate::from_ymd_opt(month.year(), month.month() + 1, 1).expect("date")
    };
    (start, next - Duration::days(1))
}

pub fn prorated_amount(amount: i64, lease_start: NaiveDate, lease_end: Option<NaiveDate>, start: NaiveDate, end: NaiveDate) -> i64 {
    let active_start = lease_start.max(start);
    let active_end = lease_end.unwrap_or(end).min(end);
    if active_end < active_start {
        return 0;
    }
    let active_days = (active_end - active_start).num_days() + 1;
    let month_days = (end - start).num_days() + 1;
    if active_days >= month_days { amount } else { ((Decimal::from(amount) * Decimal::from(active_days)) / Decimal::from(month_days)).round_dp(0).to_i64().unwrap_or(0) }
}

pub fn due_date_for_month(start: NaiveDate, payment_day: i16) -> NaiveDate {
    let (_, end) = month_bounds(start);
    start.with_day((payment_day as u32).min(end.day())).unwrap_or(end)
}

pub fn invoice_status_fr(status: &str) -> &'static str {
    match status {
        "DRAFT" => "Brouillon",
        "VALIDATED" => "Validée",
        "ISSUED" => "Émise",
        "PAID_PARTIAL" => "Partiellement payée",
        "PAID" => "Payée",
        "OVERDUE" => "En retard",
        "CANCELLED" => "Annulée",
        "CREDITED" => "Créditée",
        _ => "État inconnu",
    }
}

#[server]
pub async fn list_billing_invoices() -> Result<Vec<BillingInvoiceItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let rows = sqlx::query(
            r#"SELECT i.id,i.lease_id,COALESCE(i.invoice_number,'BROUILLON') AS invoice_number,
                      i.document_kind,i.issue_date,i.due_date,i.billing_period_start,i.billing_period_end,
                      i.net_cents,i.vat_cents,i.gross_cents,
                      COALESCE((SELECT SUM(p.amount_cents) FROM payments p WHERE p.invoice_id=i.id AND p.legal_entity_id=i.legal_entity_id),0)::bigint AS paid_cents,
                      i.status,COALESCE(t.legal_name,'Sans locataire') AS tenant_name,
                      COALESCE(i.pdf_path,'') AS pdf_path,i.pdf_draft_watermark,
                      i.electronic_status,COALESCE(i.electronic_provider_code,'') AS electronic_provider_code
               FROM invoices i
               LEFT JOIN leases l ON l.id=i.lease_id
               LEFT JOIN tenants t ON t.id=l.tenant_id
               WHERE i.legal_entity_id=$1
               ORDER BY i.issue_date DESC,i.created_at DESC"#,
        )
        .bind(entity)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;
        use sqlx::Row;
        Ok(rows.into_iter().map(|r| {
            let status: String = r.get("status");
            BillingInvoiceItem {
                id: r.get("id"), lease_id: r.get("lease_id"), invoice_number: r.get("invoice_number"),
                document_kind: r.get("document_kind"), issue_date: r.get("issue_date"), due_date: r.get("due_date"),
                period_start: r.get("billing_period_start"), period_end: r.get("billing_period_end"),
                net_cents: r.get("net_cents"), vat_cents: r.get("vat_cents"), gross_cents: r.get("gross_cents"),
                paid_cents: r.get("paid_cents"), status_fr: invoice_status_fr(&status).into(), status_code: status,
                tenant_name: r.get("tenant_name"), pdf_path: r.get("pdf_path"),
                pdf_draft_watermark: r.get("pdf_draft_watermark"), electronic_status: r.get("electronic_status"),
                electronic_provider: r.get("electronic_provider_code"),
            }
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_billing_invoices est exécutée côté serveur"))
}

#[server]
pub async fn generate_monthly_billing(period_start: NaiveDate) -> Result<BillingRunResult, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let (start, end) = month_bounds(period_start);
        let leases = sqlx::query(r#"SELECT l.id,l.unit_id,l.tenant_id,l.reference,l.start_date,l.end_date,l.payment_day,
                    CASE WHEN l.rent_amount_cents > 0 THEN l.rent_amount_cents ELSE u.base_rent_cents END AS rent_amount_cents,
                    l.rent_frequency,l.vat_mode,u.vat_rate_bp
             FROM leases l JOIN units u ON u.id=l.unit_id JOIN properties p ON p.id=u.property_id
             WHERE p.legal_entity_id=$1 AND l.legal_entity_id=$1 AND l.active=true
               AND l.start_date <= $3 AND (l.end_date IS NULL OR l.end_date >= $2)
             ORDER BY l.reference"#)
            .bind(entity).bind(start).bind(end).fetch_all(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row;
        let mut result = BillingRunResult { generated: 0, already_present: 0, skipped: 0, messages: Vec::new() };
        for lease in leases {
            let lease_id: Uuid = lease.get("id");
            let payment_day: i16 = lease.get("payment_day");
            let frequency: String = lease.get("rent_frequency");
            let multiplier = match frequency.as_str() {
                "MONTHLY" => 1,
                "QUARTERLY" => if matches!(start.month(),1|4|7|10) { 3 } else { result.skipped += 1; continue },
                "ANNUAL" => if start.month() == 1 { 12 } else { result.skipped += 1; continue },
                _ => { result.messages.push(format!("{} : périodicité {} non automatisée", lease.get::<String,_>("reference"), frequency)); result.skipped += 1; continue }
            };
            let raw_rent: i64 = lease.get("rent_amount_cents");
            let rent = prorated_amount(raw_rent.saturating_mul(multiplier), lease.get("start_date"), lease.get("end_date"), start, end);
            if rent <= 0 { result.skipped += 1; continue; }
            let rate: i32 = lease.get("vat_rate_bp");
            let vat_mode: String = lease.get("vat_mode");
            let vat = if vat_mode == "NONE" { 0 } else { ((Decimal::from(rent) * Decimal::from(rate)) / Decimal::from(10000u32)).round_dp(0).to_i64().unwrap_or(0) };
            let generation_key = format!("S07:RENT:{}:{}", lease_id, start);
            let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM invoices WHERE legal_entity_id=$1 AND generation_key=$2").bind(entity).bind(&generation_key).fetch_optional(pool).await.map_err(ServerFnError::new)?;
            if exists.is_some() { result.already_present += 1; continue; }
            let invoice_id = create_draft_invoice(pool, entity, lease_id, start, end, due_date_for_month(start, payment_day), "INVOICE", &generation_key, rent, vat, raw_rent_line_description(&frequency, multiplier)).await.map_err(ServerFnError::new)?;
            append_fixed_charge_lines(pool, entity, invoice_id, lease_id, start, end).await.map_err(ServerFnError::new)?;
            append_recoverable_tax_lines(pool, entity, invoice_id, lease_id, start, end).await.map_err(ServerFnError::new)?;
            recompute_invoice_totals(pool, entity, invoice_id).await.map_err(ServerFnError::new)?;
            generate_invoice_pdf(pool, entity, invoice_id).await.map_err(ServerFnError::new)?;
            result.generated += 1;
        }
        Ok(result)
    }
    #[cfg(not(feature = "server"))]
    { let _ = period_start; Err(ServerFnError::new("generate_monthly_billing est exécutée côté serveur")) }
}

fn raw_rent_line_description(frequency: &str, multiplier: i64) -> String {
    match frequency {
        "MONTHLY" => "Loyer contractuel".to_string(),
        "QUARTERLY" => format!("Loyer contractuel — trimestre ({} mois)", multiplier),
        "ANNUAL" => format!("Loyer contractuel — annuel ({} mois)", multiplier),
        _ => "Loyer contractuel".to_string(),
    }
}

#[server]
pub async fn add_charge_actual(lease_id: Uuid, period_start: NaiveDate, charge_code: String, actual_cents: i64, source_reference: String) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        if actual_cents < 0 || charge_code.trim().is_empty() { return Err(ServerFnError::new("Montant ou code de charge invalide")); }
        let (start,end) = month_bounds(period_start);
        let id: Uuid = sqlx::query_scalar(r#"INSERT INTO billing_charge_actuals(legal_entity_id,lease_id,period_start,period_end,charge_code,actual_cents,source_reference,assessed_at)
            VALUES($1,$2,$3,$4,$5,$6,$7,CURRENT_DATE)
            ON CONFLICT(legal_entity_id,lease_id,period_start,period_end,charge_code)
            DO UPDATE SET actual_cents=EXCLUDED.actual_cents,source_reference=EXCLUDED.source_reference,assessed_at=CURRENT_DATE
            RETURNING id"#)
            .bind(current_legal_entity_id()).bind(lease_id).bind(start).bind(end).bind(charge_code.trim()).bind(actual_cents).bind(source_reference.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _=(lease_id,period_start,charge_code,actual_cents,source_reference); Err(ServerFnError::new("add_charge_actual est exécutée côté serveur")) }
}

#[server]
pub async fn calculate_charge_regularization(lease_id: Uuid, period_start: NaiveDate, charge_code: String) -> Result<RegularizationResult, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let (start,end) = month_bounds(period_start);
        let actual: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(actual_cents),0)::bigint FROM billing_charge_actuals WHERE legal_entity_id=$1 AND lease_id=$2 AND period_start=$3 AND period_end=$4 AND charge_code=$5")
            .bind(entity).bind(lease_id).bind(start).bind(end).bind(charge_code.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let provision: i64 = sqlx::query_scalar(r#"SELECT COALESCE(SUM(l.net_cents),0)::bigint
            FROM billing_invoice_lines l JOIN invoices i ON i.id=l.invoice_id AND i.legal_entity_id=l.legal_entity_id
            WHERE l.legal_entity_id=$1 AND i.lease_id=$2 AND l.line_type='CHARGE'
              AND l.service_period_start=$3 AND l.service_period_end=$4
              AND COALESCE(l.source_payload->>'charge_code','')=$5"#)
            .bind(entity).bind(lease_id).bind(start).bind(end).bind(charge_code.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let difference = actual - provision;
        let treatment = if difference == 0 { "NO_DIFFERENCE" } else if difference > 0 { "ADDITIONAL_INVOICE" } else { "CREDIT_NOTE" };
        let invoice_id = if difference == 0 { None } else {
            let due = due_date_for_month(end + Duration::days(1), 5);
            let kind = if difference > 0 { "INVOICE" } else { "CREDIT_NOTE" };
            let key = format!("S07:REG:{}:{}:{}", lease_id, start, charge_code.trim());
            let gross = difference.abs();
            let net = gross;
            let id = ensure_adjustment_invoice(pool, entity, Some(lease_id), start, end, due, kind, &key, net, 0).await.map_err(ServerFnError::new)?;
            let line_type = "REGULARISATION";
            let desc = format!("Régularisation {} — provisions {} / réel {}", charge_code.trim(), format_eur_sync(provision), format_eur_sync(actual));
            let line_no = next_line_no(pool, entity, id).await.map_err(ServerFnError::new)?;
            sqlx::query(r#"INSERT INTO billing_invoice_lines(legal_entity_id,invoice_id,line_no,line_type,description,quantity,unit_net_cents,net_cents,vat_rate_bp,vat_cents,gross_cents,source_type,source_id,source_payload,service_period_start,service_period_end)
                VALUES($1,$2,$3,$4,$5,1,$6,$6,0,0,$6,'REGULARIZATION',$7,$8,$9,$10)"#)
                .bind(entity).bind(id).bind(line_no).bind(line_type).bind(desc).bind(net).bind(Uuid::new_v4()).bind(serde_json::json!({"charge_code":charge_code.trim(),"provision_cents":provision,"actual_cents":actual,"difference_cents":difference})).bind(start).bind(end).execute(pool).await.map_err(ServerFnError::new)?;
            recompute_invoice_totals(pool, entity, id).await.map_err(ServerFnError::new)?;
            generate_invoice_pdf(pool, entity, id).await.map_err(ServerFnError::new)?;
            Some(id)
        };
        let id = Uuid::new_v4();
        sqlx::query(r#"INSERT INTO billing_regularizations(id,legal_entity_id,lease_id,period_start,period_end,charge_code,provision_cents,actual_cents,difference_cents,treatment,invoice_id)
            VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            ON CONFLICT(legal_entity_id,lease_id,period_start,period_end,charge_code)
            DO UPDATE SET provision_cents=EXCLUDED.provision_cents,actual_cents=EXCLUDED.actual_cents,difference_cents=EXCLUDED.difference_cents,treatment=EXCLUDED.treatment,invoice_id=EXCLUDED.invoice_id"#)
            .bind(id).bind(entity).bind(lease_id).bind(start).bind(end).bind(charge_code.trim()).bind(provision).bind(actual).bind(difference).bind(treatment).bind(invoice_id)
            .execute(pool).await.map_err(ServerFnError::new)?;
        let stored_id: Uuid = sqlx::query_scalar("SELECT id FROM billing_regularizations WHERE legal_entity_id=$1 AND lease_id=$2 AND period_start=$3 AND period_end=$4 AND charge_code=$5")
            .bind(entity).bind(lease_id).bind(start).bind(end).bind(charge_code.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(RegularizationResult{id:stored_id,lease_id,charge_code:charge_code.trim().to_owned(),provision_cents:provision,actual_cents:actual,difference_cents:difference,treatment:treatment.to_owned(),invoice_id})
    }
    #[cfg(not(feature = "server"))]
    { let _=(lease_id,period_start,charge_code); Err(ServerFnError::new("calculate_charge_regularization est exécutée côté serveur")) }
}

#[server]
pub async fn add_recoverable_tax_rule(lease_id: Uuid, tax_code: String, label: String, legal_basis: String, recovery_rate_bp: i32, effective_from: NaiveDate, effective_to: Option<NaiveDate>) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        if !(0..=10000).contains(&recovery_rate_bp) || tax_code.trim().is_empty() || label.trim().is_empty() { return Err(ServerFnError::new("Règle de taxe récupérable invalide")); }
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let id: Uuid = sqlx::query_scalar(r#"INSERT INTO lease_recoverable_taxes(legal_entity_id,lease_id,tax_code,label,legal_basis,recovery_rate_bp,effective_from,effective_to)
            SELECT $1,$2,$3,$4,$5,$6,$7,$8 WHERE EXISTS(SELECT 1 FROM leases WHERE id=$2 AND legal_entity_id=$1)
            RETURNING id"#)
            .bind(current_legal_entity_id()).bind(lease_id).bind(tax_code.trim().to_uppercase()).bind(label.trim()).bind(legal_basis.trim()).bind(recovery_rate_bp).bind(effective_from).bind(effective_to)
            .fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Bail introuvable"))?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _=(lease_id,tax_code,label,legal_basis,recovery_rate_bp,effective_from,effective_to); Err(ServerFnError::new("add_recoverable_tax_rule est exécutée côté serveur")) }
}

#[server]
pub async fn add_recoverable_tax_assessment(rule_id: Uuid, lease_id: Uuid, period_start: NaiveDate, assessed_cents: i64, source_reference: String) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        if assessed_cents < 0 { return Err(ServerFnError::new("Montant de taxe invalide")); }
        let (start,end)=month_bounds(period_start);
        let id:Uuid=sqlx::query_scalar(r#"INSERT INTO billing_tax_assessments(legal_entity_id,lease_id,tax_rule_id,period_start,period_end,assessed_cents,assessed_at,source_reference)
            VALUES($1,$2,$3,$4,$5,$6,CURRENT_DATE,$7)
            ON CONFLICT(legal_entity_id,tax_rule_id,period_start,period_end)
            DO UPDATE SET assessed_cents=EXCLUDED.assessed_cents,assessed_at=CURRENT_DATE,source_reference=EXCLUDED.source_reference RETURNING id"#)
            .bind(current_legal_entity_id()).bind(lease_id).bind(rule_id).bind(start).bind(end).bind(assessed_cents).bind(source_reference.trim()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _=(rule_id,lease_id,period_start,assessed_cents,source_reference); Err(ServerFnError::new("add_recoverable_tax_assessment est exécutée côté serveur")) }
}

#[server]
pub async fn list_recoverable_tax_rules(lease_id: Uuid) -> Result<Vec<RecoverableTaxItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT id,lease_id,tax_code,label,legal_basis,recovery_rate_bp,effective_from,effective_to,active FROM lease_recoverable_taxes WHERE lease_id=$1 AND legal_entity_id=$2 ORDER BY active DESC,effective_from DESC")
            .bind(lease_id).bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row;
        Ok(rows.into_iter().map(|r|RecoverableTaxItem{id:r.get("id"),lease_id:r.get("lease_id"),tax_code:r.get("tax_code"),label:r.get("label"),legal_basis:r.get("legal_basis"),recovery_rate_bp:r.get("recovery_rate_bp"),effective_from:r.get("effective_from"),effective_to:r.get("effective_to"),active:r.get("active")}).collect())
    }
    #[cfg(not(feature = "server"))]
    { let _=lease_id; Err(ServerFnError::new("list_recoverable_tax_rules est exécutée côté serveur")) }
}

#[server]
pub async fn validate_billing_invoice(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; change_invoice_status(pool,id,"DRAFT","VALIDATED","Validation interne sans émission externe").await.map_err(ServerFnError::new)?; generate_invoice_pdf(pool,current_legal_entity_id(),id).await.map_err(ServerFnError::new)?; Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=id; Err(ServerFnError::new("validate_billing_invoice est exécutée côté serveur")) }
}

#[server]
pub async fn issue_billing_invoice(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let row=sqlx::query("SELECT status,document_kind FROM invoices WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row;
        let Some(r)=row else { return Err(ServerFnError::new("Facture introuvable")); };
        let status:String=r.get("status");
        if status!="VALIDATED" { return Err(ServerFnError::new("Une facture doit être validée avant émission")); }
        change_invoice_status(pool,id,"VALIDATED","ISSUED","Émission de la facture").await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE invoices SET issued_at=now(),electronic_status=CASE WHEN electronic_provider_code IS NULL THEN 'NOT_READY' ELSE 'READY' END,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(current_legal_entity_id()).execute(pool).await.map_err(ServerFnError::new)?;
        generate_invoice_pdf(pool,current_legal_entity_id(),id).await.map_err(ServerFnError::new)?;
        if r.get::<String,_>("document_kind")=="CREDIT_NOTE" { refresh_source_credit_status(pool,id).await.map_err(ServerFnError::new)?; }
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=id; Err(ServerFnError::new("issue_billing_invoice est exécutée côté serveur")) }
}

#[server]
pub async fn cancel_billing_invoice(id: Uuid, reason: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        if reason.trim().is_empty(){return Err(ServerFnError::new("Motif d'annulation requis"));}
        let row=sqlx::query("SELECT status FROM invoices WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row; let Some(r)=row else{return Err(ServerFnError::new("Facture introuvable"));}; let status:String=r.get("status");
        if matches!(status.as_str(),"PAID"|"PAID_PARTIAL"|"CREDITED") { return Err(ServerFnError::new("Une facture déjà encaissée ne peut pas être annulée directement ; utilisez un avoir")); }
        change_invoice_status(pool,id,&status,"CANCELLED",reason.trim()).await.map_err(ServerFnError::new)?;
        generate_invoice_pdf(pool,current_legal_entity_id(),id).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=(id,reason); Err(ServerFnError::new("cancel_billing_invoice est exécutée côté serveur")) }
}

#[server]
pub async fn create_credit_note(id: Uuid, amount_gross_cents: Option<i64>, reason: String) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        if reason.trim().is_empty(){return Err(ServerFnError::new("Motif d'avoir requis"));}
        let original=sqlx::query("SELECT lease_id,invoice_number,issue_date,due_date,net_cents,vat_cents,gross_cents,billing_period_start,billing_period_end,status FROM invoices WHERE id=$1 AND legal_entity_id=$2 AND document_kind='INVOICE'").bind(id).bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        use sqlx::Row; let Some(o)=original else{return Err(ServerFnError::new("Facture source introuvable"));};
        let status:String=o.get("status"); if status=="CANCELLED" { return Err(ServerFnError::new("Une facture annulée ne doit pas être corrigée par un avoir")); }
        let original_gross:i64=o.get("gross_cents");
        let already_credited:i64=sqlx::query_scalar("SELECT COALESCE(SUM(gross_cents),0)::bigint FROM invoices WHERE legal_entity_id=$1 AND source_invoice_id=$2 AND document_kind='CREDIT_NOTE' AND status<>'CANCELLED'").bind(entity).bind(id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let remaining=original_gross.saturating_sub(already_credited);
        let credit_gross=amount_gross_cents.unwrap_or(remaining); if credit_gross<=0||credit_gross>remaining{return Err(ServerFnError::new("Montant d'avoir supérieur au montant encore corrigeable"));}
        let original_net:i64=o.get("net_cents");
        let credit_net=((Decimal::from(original_net)*Decimal::from(credit_gross))/Decimal::from(original_gross.max(1))).round_dp(0).to_i64().unwrap_or(0);
        let credit_vat=credit_gross-credit_net;
        let credit_vat_rate=if credit_net>0 { ((Decimal::from(credit_vat)*Decimal::from(10000u32))/Decimal::from(credit_net)).round_dp(0).to_i32().unwrap_or(0) } else { 0 };
        let source_period_start:Option<NaiveDate>=o.get("billing_period_start");
        let source_period_end:Option<NaiveDate>=o.get("billing_period_end");
        let source_lease:Option<Uuid>=o.get("lease_id");
        let issue_date:NaiveDate=o.get("issue_date");
        let due_date:NaiveDate=o.get("due_date");
        let credit_start=source_period_start.unwrap_or(issue_date);
        let credit_end=source_period_end.unwrap_or(issue_date);
        let credit_id=ensure_adjustment_invoice(pool,entity,source_lease,credit_start,credit_end,due_date,"CREDIT_NOTE",&format!("S07:CREDIT:{}",Uuid::new_v4()),credit_net,credit_vat).await.map_err(ServerFnError::new)?;
        let line_no=next_line_no(pool,entity,credit_id).await.map_err(ServerFnError::new)?;
        sqlx::query(r#"INSERT INTO billing_invoice_lines(legal_entity_id,invoice_id,line_no,line_type,description,quantity,unit_net_cents,net_cents,vat_rate_bp,vat_cents,gross_cents,source_type,source_id,source_payload,service_period_start,service_period_end)
            VALUES($1,$2,$3,'CREDIT',$4,1,$5,$5,$6,$7,$8,'CREDIT_NOTE',$9,$10,$11,$12)"#)
            .bind(entity).bind(credit_id).bind(line_no).bind(format!("Avoir — {}",reason.trim())).bind(credit_net).bind(credit_vat_rate).bind(credit_vat).bind(credit_gross).bind(id).bind(serde_json::json!({"source_invoice_id":id,"reason":reason.trim(),"credit_gross_cents":credit_gross})).bind(Some(credit_start)).bind(Some(credit_end)).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE invoices SET source_invoice_id=$3,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(credit_id).bind(entity).bind(id).execute(pool).await.map_err(ServerFnError::new)?;
        recompute_invoice_totals(pool,entity,credit_id).await.map_err(ServerFnError::new)?;
        generate_invoice_pdf(pool,entity,credit_id).await.map_err(ServerFnError::new)?;
        Ok(credit_id)
    }
    #[cfg(not(feature = "server"))]
    { let _=(id,amount_gross_cents,reason); Err(ServerFnError::new("create_credit_note est exécutée côté serveur")) }
}

#[server]
pub async fn set_einvoice_provider(provider_code:String, provider_name:String, format_code:String, endpoint_reference:String, effective_from:NaiveDate) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let format_code=format_code.to_uppercase(); if !matches!(format_code.as_str(),"UBL"|"CII"|"MIXED") { return Err(ServerFnError::new("Format e-facture invalide")); }
        if provider_code.trim().is_empty()||provider_name.trim().is_empty(){return Err(ServerFnError::new("Plateforme requise"));}
        if effective_from > Utc::now().date_naive(){return Err(ServerFnError::new("Le changement de plateforme doit prendre effet aujourd'hui ou à une date passée"));}
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id();
        sqlx::query("UPDATE billing_einvoice_connections SET active=false,effective_to=$2 WHERE legal_entity_id=$1 AND active=true").bind(entity).bind(effective_from - Duration::days(1)).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO billing_einvoice_connections(legal_entity_id,provider_code,provider_name,format_code,endpoint_reference,active,effective_from) VALUES($1,$2,$3,$4,$5,true,$6)").bind(entity).bind(provider_code.trim()).bind(provider_name.trim()).bind(&format_code).bind(endpoint_reference.trim()).bind(effective_from).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _=(provider_code,provider_name,format_code,endpoint_reference,effective_from); Err(ServerFnError::new("set_einvoice_provider est exécutée côté serveur")) }
}

#[server]
pub async fn list_einvoice_connections() -> Result<Vec<EInvoiceConfigItem>, ServerFnError> {
    #[cfg(feature = "server")]
    { let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; use sqlx::Row; let rows=sqlx::query("SELECT id,provider_code,provider_name,format_code,endpoint_reference,active,effective_from,effective_to FROM billing_einvoice_connections WHERE legal_entity_id=$1 ORDER BY effective_from DESC,id DESC").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?; Ok(rows.into_iter().map(|r|EInvoiceConfigItem{id:r.get("id"),provider_code:r.get("provider_code"),provider_name:r.get("provider_name"),format_code:r.get("format_code"),endpoint_reference:r.get("endpoint_reference"),active:r.get("active"),effective_from:r.get("effective_from"),effective_to:r.get("effective_to")}).collect()) }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_einvoice_connections est exécutée côté serveur"))
}

#[server]
pub async fn prepare_einvoice(id: Uuid) -> Result<EInvoicePrepareResult, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool=crate::infrastructure::db().await.map_err(ServerFnError::new)?; let entity=current_legal_entity_id(); use sqlx::Row;
        let config=sqlx::query("SELECT provider_name,format_code FROM billing_einvoice_connections WHERE legal_entity_id=$1 AND active=true ORDER BY effective_from DESC LIMIT 1").bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Aucune plateforme e-facture active n'est configurée"))?;
        let inv=sqlx::query("SELECT i.invoice_number,i.issue_date,i.due_date,i.net_cents,i.vat_cents,i.gross_cents,i.document_kind,COALESCE(t.legal_name,'') tenant_name,COALESCE(t.siret,'') tenant_siret,COALESCE(e.siren,'') entity_siren FROM invoices i LEFT JOIN leases l ON l.id=i.lease_id LEFT JOIN tenants t ON t.id=l.tenant_id JOIN legal_entities e ON e.id=i.legal_entity_id WHERE i.id=$1 AND i.legal_entity_id=$2").bind(id).bind(entity).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Facture introuvable"))?;
        let payload=serde_json::json!({"version":"S07-1","document_kind":inv.get::<String,_>("document_kind"),"invoice_number":inv.get::<Option<String>,_>("invoice_number"),"issue_date":inv.get::<NaiveDate,_>("issue_date"),"due_date":inv.get::<NaiveDate,_>("due_date"),"net_cents":inv.get::<i64,_>("net_cents"),"vat_cents":inv.get::<i64,_>("vat_cents"),"gross_cents":inv.get::<i64,_>("gross_cents"),"supplier":{"siren":inv.get::<String,_>("entity_siren")},"customer":{"name":inv.get::<String,_>("tenant_name"),"siret":inv.get::<String,_>("tenant_siret")}});
        let json=serde_json::to_string_pretty(&payload).map_err(ServerFnError::new)?;
        sqlx::query("UPDATE invoices SET electronic_status='READY',electronic_provider_code=(SELECT provider_code FROM billing_einvoice_connections WHERE legal_entity_id=$1 AND active=true ORDER BY effective_from DESC LIMIT 1),electronic_format=$3,updated_at=now() WHERE id=$2 AND legal_entity_id=$1").bind(entity).bind(id).bind(config.get::<String,_>("format_code")).execute(pool).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO billing_einvoice_events(legal_entity_id,invoice_id,connection_id,direction,event_type,provider_status,payload) SELECT $1,$2,id,'OUTBOUND','PREPARED','READY',$3 FROM billing_einvoice_connections WHERE legal_entity_id=$1 AND active=true ORDER BY effective_from DESC LIMIT 1").bind(entity).bind(id).bind(payload.clone()).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(EInvoicePrepareResult{invoice_id:id,provider_name:config.get("provider_name"),format_code:config.get("format_code"),payload_json:json})
    }
    #[cfg(not(feature = "server"))]
    { let _=id; Err(ServerFnError::new("prepare_einvoice est exécutée côté serveur")) }
}

#[cfg(feature="server")]
async fn create_draft_invoice(pool:&sqlx::PgPool,entity:Uuid,lease_id:Uuid,start:NaiveDate,end:NaiveDate,due:NaiveDate,kind:&str,generation_key:&str,rent_net:i64,rent_vat:i64,rent_desc:String)->Result<Uuid,sqlx::Error>{
    let number=next_invoice_sequence_s07(pool,entity,start).await?;
    let invoice_number=format!("{}-{:04}",start.format("%Y%m"),number);
    let id:Uuid=sqlx::query_scalar(r#"INSERT INTO invoices(legal_entity_id,lease_id,invoice_number,issue_date,due_date,service_period_start,service_period_end,billing_period_start,billing_period_end,net_cents,vat_cents,gross_cents,status,document_kind,generation_key,pdf_draft_watermark)
        VALUES($1,$2,$3,$4,$5,$4,$6,$4,$6,$7,$8,$7+$8,'DRAFT',$9,$10,true) ON CONFLICT(legal_entity_id,generation_key) WHERE generation_key IS NOT NULL AND btrim(generation_key)<>'' DO NOTHING RETURNING id"#)
        .bind(entity).bind(lease_id).bind(invoice_number).bind(start).bind(due).bind(end).bind(rent_net).bind(rent_vat).bind(kind).bind(generation_key).fetch_one(pool).await?;
    sqlx::query("INSERT INTO billing_invoice_state_history(legal_entity_id,invoice_id,from_status,to_status,reason) VALUES($1,$2,NULL,'DRAFT','Création de brouillon S07')").bind(entity).bind(id).execute(pool).await?;
    let line_id=Uuid::new_v4(); let line_no=1;
    sqlx::query(r#"INSERT INTO billing_invoice_lines( id,legal_entity_id,invoice_id,line_no,line_type,description,quantity,unit_net_cents,net_cents,vat_rate_bp,vat_cents,gross_cents,source_type,source_id,source_payload,service_period_start,service_period_end)
        VALUES($1,$2,$3,$4,'RENT',$5,1,$6,$6,$7,$8,$6+$8,'LEASE',$9,$10,$11,$12)"#)
        .bind(line_id).bind(entity).bind(id).bind(line_no).bind(rent_desc).bind(rent_net).bind(if rent_net>0 { ((Decimal::from(rent_vat)*Decimal::from(10000u32))/Decimal::from(rent_net)).round_dp(0).to_i32().unwrap_or(0) } else {0}).bind(rent_vat).bind(lease_id).bind(serde_json::json!({"kind":"RENT"})).bind(start).bind(end).execute(pool).await?;
    let _=line_id; Ok(id)
}

#[cfg(feature="server")]
async fn append_fixed_charge_lines(pool:&sqlx::PgPool,entity:Uuid,invoice_id:Uuid,lease_id:Uuid,start:NaiveDate,end:NaiveDate)->Result<(),sqlx::Error>{
    use sqlx::Row;
    let rules=sqlx::query("SELECT id,charge_type,mode,amount_cents,variable_formula FROM lease_charge_rules WHERE legal_entity_id=$1 AND lease_id=$2 AND active=true AND effective_from <= $3 AND (effective_to IS NULL OR effective_to >= $3) ORDER BY effective_from,id").bind(entity).bind(lease_id).bind(end).fetch_all(pool).await?;
    for rule in rules { let amount:i64=rule.get("amount_cents"); if amount<=0 {continue;} let line_no=next_line_no(pool,entity,invoice_id).await?; let code:String=rule.get("charge_type"); let mode:String=rule.get("mode"); let formula:String=rule.get("variable_formula"); sqlx::query("INSERT INTO billing_invoice_lines(legal_entity_id,invoice_id,line_no,line_type,description,quantity,unit_net_cents,net_cents,vat_rate_bp,vat_cents,gross_cents,source_type,source_id,source_payload,service_period_start,service_period_end) VALUES($1,$2,$3,'CHARGE',$4,1,$5,$5,0,0,$5,'LEASE_CHARGE',$6,$7,$8,$9)").bind(entity).bind(invoice_id).bind(line_no).bind(if formula.trim().is_empty(){format!("Charge — {} ({})",code,mode)}else{format!("Charge — {} ({})",code,formula)}).bind(amount).bind(rule.get::<Uuid,_>("id")).bind(serde_json::json!({"charge_code":code,"mode":mode,"formula":formula,"rule_id":rule.get::<Uuid,_>("id")})).bind(start).bind(end).execute(pool).await?; }
    Ok(())
}

#[cfg(feature="server")]
async fn append_recoverable_tax_lines(pool:&sqlx::PgPool,entity:Uuid,invoice_id:Uuid,lease_id:Uuid,start:NaiveDate,end:NaiveDate)->Result<(),sqlx::Error>{
    use sqlx::Row;
    let rows=sqlx::query("SELECT a.id,a.assessed_cents,r.id rule_id,r.tax_code,r.label,r.legal_basis,r.recovery_rate_bp FROM billing_tax_assessments a JOIN lease_recoverable_taxes r ON r.id=a.tax_rule_id AND r.legal_entity_id=a.legal_entity_id WHERE a.legal_entity_id=$1 AND a.lease_id=$2 AND a.period_start=$3 AND a.period_end=$4 AND r.active=true AND r.effective_from <= $4 AND (r.effective_to IS NULL OR r.effective_to >= $3)").bind(entity).bind(lease_id).bind(start).bind(end).fetch_all(pool).await?;
    for r in rows { let assessed:i64=r.get("assessed_cents"); let recovery:i32=r.get("recovery_rate_bp"); let amount=((Decimal::from(assessed)*Decimal::from(recovery))/Decimal::from(10000u32)).round_dp(0).to_i64().unwrap_or(0); if amount<=0{continue;} let line_no=next_line_no(pool,entity,invoice_id).await?; let code:String=r.get("tax_code"); let label:String=r.get("label"); sqlx::query("INSERT INTO billing_invoice_lines(legal_entity_id,invoice_id,line_no,line_type,description,quantity,unit_net_cents,net_cents,vat_rate_bp,vat_cents,gross_cents,recoverable_tax,source_type,source_id,source_payload,service_period_start,service_period_end) VALUES($1,$2,$3,'RECOVERABLE_TAX',$4,1,$5,$5,0,0,$5,true,'TAX_ASSESSMENT',$6,$7,$8,$9)").bind(entity).bind(invoice_id).bind(line_no).bind(format!("Taxe récupérable — {} ({})",label,code)).bind(amount).bind(r.get::<Uuid,_>("id")).bind(serde_json::json!({"tax_code":code,"legal_basis":r.get::<String,_>("legal_basis"),"recovery_rate_bp":recovery,"rule_id":r.get::<Uuid,_>("rule_id")})).bind(start).bind(end).execute(pool).await?; }
    Ok(())
}

#[cfg(feature="server")]
async fn ensure_adjustment_invoice(pool:&sqlx::PgPool,entity:Uuid,lease_id:Option<Uuid>,start:NaiveDate,end:NaiveDate,due:NaiveDate,kind:&str,key:&str,net:i64,vat:i64)->Result<Uuid,sqlx::Error>{
    if let Some(id)=sqlx::query_scalar::<_,Uuid>("SELECT id FROM invoices WHERE legal_entity_id=$1 AND generation_key=$2").bind(entity).bind(key).fetch_optional(pool).await? {return Ok(id);}
    let number=next_invoice_sequence_s07(pool,entity,start).await?;
    let invoice_number=format!("{}-{:04}",start.format("%Y%m"),number);
    let id:Uuid=sqlx::query_scalar(r#"INSERT INTO invoices(legal_entity_id,lease_id,invoice_number,issue_date,due_date,service_period_start,service_period_end,billing_period_start,billing_period_end,net_cents,vat_cents,gross_cents,status,document_kind,generation_key,pdf_draft_watermark)
        VALUES($1,$2,$3,$4,$5,$4,$6,$4,$6,$7,$8,$7+$8,'DRAFT',$9,$10,true) RETURNING id"#)
        .bind(entity).bind(lease_id).bind(invoice_number).bind(start).bind(due).bind(end).bind(net).bind(vat).bind(kind).bind(key).fetch_one(pool).await?;
    sqlx::query("INSERT INTO billing_invoice_state_history(legal_entity_id,invoice_id,from_status,to_status,reason) VALUES($1,$2,NULL,'DRAFT','Création d''un brouillon d''ajustement')").bind(entity).bind(id).execute(pool).await?;
    Ok(id)
}

#[cfg(feature="server")]
async fn next_line_no(pool:&sqlx::PgPool,entity:Uuid,invoice_id:Uuid)->Result<i32,sqlx::Error>{sqlx::query_scalar("SELECT COALESCE(MAX(line_no),0)::int+1 FROM billing_invoice_lines WHERE legal_entity_id=$1 AND invoice_id=$2").bind(entity).bind(invoice_id).fetch_one(pool).await}

#[cfg(feature="server")]
async fn recompute_invoice_totals(pool:&sqlx::PgPool,entity:Uuid,invoice_id:Uuid)->Result<(),sqlx::Error>{
    let (net,vat):(i64,i64)=sqlx::query_as("SELECT COALESCE(SUM(net_cents),0)::bigint,COALESCE(SUM(vat_cents),0)::bigint FROM billing_invoice_lines WHERE legal_entity_id=$1 AND invoice_id=$2").bind(entity).bind(invoice_id).fetch_one(pool).await?;
    sqlx::query("UPDATE invoices SET net_cents=$3,vat_cents=$4,gross_cents=$3+$4,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(invoice_id).bind(entity).bind(net).bind(vat).execute(pool).await?; Ok(())
}

#[cfg(feature="server")]
async fn change_invoice_status(pool:&sqlx::PgPool,id:Uuid,from:&str,to:&str,reason:&str)->Result<(),sqlx::Error>{
    let entity=current_legal_entity_id(); let changed=sqlx::query("UPDATE invoices SET status=$3,updated_at=now() WHERE id=$1 AND legal_entity_id=$2 AND status=$4").bind(id).bind(entity).bind(to).bind(from).execute(pool).await?; if changed.rows_affected()==0{return Err(sqlx::Error::Protocol(format!("État de facture inattendu : {} → {}",from,to)));}
    sqlx::query("INSERT INTO billing_invoice_state_history(legal_entity_id,invoice_id,from_status,to_status,reason) VALUES($1,$2,$3,$4,$5)").bind(entity).bind(id).bind(from).bind(to).bind(reason).execute(pool).await?; Ok(())
}

#[cfg(feature="server")]
async fn refresh_source_credit_status(pool:&sqlx::PgPool,credit_id:Uuid)->Result<(),sqlx::Error>{
    use sqlx::Row; let row=sqlx::query("SELECT source_invoice_id,document_kind FROM invoices WHERE id=$1 AND legal_entity_id=$2").bind(credit_id).bind(current_legal_entity_id()).fetch_optional(pool).await?; let Some(r)=row else{return Ok(());}; let source:Option<Uuid>=r.get("source_invoice_id"); if let Some(source_id)=source{let issued:i64=sqlx::query_scalar("SELECT COALESCE(SUM(gross_cents),0)::bigint FROM invoices WHERE legal_entity_id=$1 AND source_invoice_id=$2 AND document_kind='CREDIT_NOTE' AND status IN ('ISSUED','PAID_PARTIAL','PAID')").bind(current_legal_entity_id()).bind(source_id).fetch_one(pool).await?; let original:i64=sqlx::query_scalar("SELECT gross_cents FROM invoices WHERE id=$1 AND legal_entity_id=$2").bind(source_id).bind(current_legal_entity_id()).fetch_one(pool).await?; if issued>=original {sqlx::query("UPDATE invoices SET status='CREDITED',updated_at=now() WHERE id=$1 AND legal_entity_id=$2 AND status NOT IN ('CANCELLED','CREDITED')").bind(source_id).bind(current_legal_entity_id()).execute(pool).await?;} } Ok(())
}

#[cfg(feature="server")]
async fn next_invoice_sequence_s07(pool:&sqlx::PgPool,entity:Uuid,date:NaiveDate)->Result<i64,sqlx::Error>{sqlx::query_scalar("SELECT COUNT(*)::bigint+1 FROM invoices WHERE legal_entity_id=$1 AND issue_date>=date_trunc('month',$2::date) AND issue_date<date_trunc('month',$2::date)+interval '1 month'").bind(entity).bind(date).fetch_one(pool).await}

#[cfg(feature="server")]
async fn generate_invoice_pdf(pool:&sqlx::PgPool,entity:Uuid,id:Uuid)->Result<(),String>{
    use sqlx::Row;
    let row=sqlx::query("SELECT COALESCE(i.invoice_number,'BROUILLON') invoice_number,i.issue_date,i.due_date,i.net_cents,i.vat_cents,i.gross_cents,i.status,i.document_kind,i.pdf_path,COALESCE(e.legal_name,'') entity_name,COALESCE(e.registered_office,'') registered_office,COALESCE(t.legal_name,'Sans locataire') tenant_name,COALESCE(t.billing_address,'') tenant_address,COALESCE(t.siret,'') tenant_siret FROM invoices i LEFT JOIN legal_entities e ON e.id=i.legal_entity_id LEFT JOIN leases l ON l.id=i.lease_id LEFT JOIN tenants t ON t.id=l.tenant_id WHERE i.id=$1 AND i.legal_entity_id=$2").bind(id).bind(entity).fetch_optional(pool).await.map_err(|e|e.to_string())?.ok_or_else(||"Facture introuvable".to_string())?;
    let mut lines=Vec::new();
    lines.push(format!("{} — {}",if row.get::<String,_>("document_kind")=="CREDIT_NOTE" {"AVOIR"} else {"FACTURE"},row.get::<String,_>("invoice_number")));
    if row.get::<String,_>("status")=="DRAFT" {lines.push("BROUILLON — SANS EFFET EXTERNE".to_string());}
    lines.push(format!("Émetteur : {}",row.get::<String,_>("entity_name")));
    lines.push(format!("Siège : {}",row.get::<String,_>("registered_office")));
    lines.push(format!("Client : {}",row.get::<String,_>("tenant_name")));
    if !row.get::<String,_>("tenant_siret").is_empty(){lines.push(format!("SIRET client : {}",row.get::<String,_>("tenant_siret")));}
    if !row.get::<String,_>("tenant_address").is_empty(){lines.push(format!("Adresse client : {}",row.get::<String,_>("tenant_address")));}
    lines.push(format!("Date : {} — Échéance : {}",row.get::<NaiveDate,_>("issue_date").format("%d/%m/%Y"),row.get::<NaiveDate,_>("due_date").format("%d/%m/%Y")));
    lines.push("----------------------------------------".to_string());
    let detail_rows=sqlx::query("SELECT description,net_cents,vat_cents,gross_cents FROM billing_invoice_lines WHERE legal_entity_id=$1 AND invoice_id=$2 ORDER BY line_no").bind(entity).bind(id).fetch_all(pool).await.map_err(|e|e.to_string())?;
    for d in detail_rows {lines.push(format!("{} | HT {} | TVA {} | TTC {}",d.get::<String,_>("description"),format_eur_sync(d.get("net_cents")),format_eur_sync(d.get("vat_cents")),format_eur_sync(d.get("gross_cents"))));}
    lines.push("----------------------------------------".to_string());
    lines.push(format!("Total HT : {}",format_eur_sync(row.get("net_cents"))));
    lines.push(format!("TVA : {}",format_eur_sync(row.get("vat_cents"))));
    lines.push(format!("Total TTC : {}",format_eur_sync(row.get("gross_cents"))));
    let path=if let Some(existing)=row.get::<Option<String>,_>("pdf_path"){PathBuf::from(existing)}else{let root=std::env::var("SCI_BILLING_PDF_ROOT").unwrap_or_else(|_|".sci-billing".into());PathBuf::from(root).join(entity.to_string()).join(format!("{}.pdf",sanitize_pdf_name(&row.get::<String,_>("invoice_number"))))};
    if let Some(parent)=path.parent(){std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    let pdf_lines:Vec<String>=lines.into_iter().map(|v|pdf_ascii(&v)).collect();
    write_simple_pdf(&path,&pdf_lines).map_err(|e|e.to_string())?;
    let draft=row.get::<String,_>("status")=="DRAFT";
    sqlx::query("UPDATE invoices SET pdf_path=$3,pdf_generated_at=now(),pdf_draft_watermark=$4,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).bind(path.to_string_lossy().to_string()).bind(draft).execute(pool).await.map_err(|e|e.to_string())?;
    Ok(())
}

fn format_eur_sync(cents:i64)->String{format!("{}.{:02} EUR",cents/100,(cents.abs()%100))}
fn pdf_ascii(value:&str)->String{value.replace('é',"e").replace('è',"e").replace('ê',"e").replace('ë',"e").replace('à',"a").replace('â',"a").replace('ä',"a").replace('î',"i").replace('ï',"i").replace('ô',"o").replace('ö',"o").replace('ù',"u").replace('û',"u").replace('ü',"u").replace('ç',"c").replace('É',"E").replace('—',"-").replace('€',"EUR")}
fn sanitize_pdf_name(name:&str)->String{name.chars().map(|c|if c.is_ascii_alphanumeric()||matches!(c,'-'|'_'){c}else{'_'}).collect()}

#[cfg(feature="server")]
fn write_simple_pdf(path:&Path,lines:&[String])->std::io::Result<()>{
    let mut content=String::new(); content.push_str("BT /F1 9 Tf 40 790 Td 0 -16 Td\n");
    for (idx,line) in lines.iter().enumerate(){let safe=line.replace('\\',"\\\\").replace('(',"\\(").replace(')',"\\)"); if idx==1 && line.starts_with("BROUILLON") {content.push_str("/F1 16 Tf ");} else {content.push_str("/F1 9 Tf ");} let _=write!(&mut content,"({}) Tj 0 -16 Td\n",safe);}
    content.push_str("/F1 8 Tf (Page 1/1) Tj ET\n");
    let objects=vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>".to_string(),
        format!("<< /Length {} >>\\nstream\\n{}endstream",content.len(),content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
    ];
    let mut pdf=b"%PDF-1.4\n".to_vec(); let mut offsets=vec![0usize];
    for (i,obj) in objects.iter().enumerate(){offsets.push(pdf.len());pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n",i+1,obj).replace("\\n","\n").as_bytes());}
    let xref=pdf.len(); pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",objects.len()+1).as_bytes()); for off in offsets.iter().skip(1){pdf.extend_from_slice(format!("{:010} 00000 n \n",off).as_bytes());} pdf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",objects.len()+1,xref).as_bytes()); std::fs::write(path,pdf)
}

fn format_short_month(date: NaiveDate)->String{format!("{:02}/{}",date.month(),date.year())}
fn format_effective_to(date: Option<NaiveDate>) -> String {
    date.map(|d| d.to_string()).unwrap_or_else(|| "active".to_owned())
}
fn format_period_start(date: Option<NaiveDate>) -> String {
    date.map(format_short_month).unwrap_or_else(|| "-".to_owned())
}

#[component]
pub fn BillingManagementPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let invoices = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_billing_invoices().await.unwrap_or_default() } });
    let leases = use_resource(move || { let _ = refresh(); async move { crate::leases::list_lease_details().await.unwrap_or_default() } });
    let configs = use_resource(move || { let _ = refresh(); let _ = bump(); async move { list_einvoice_connections().await.unwrap_or_default() } });
    let mut period = use_signal(|| Utc::now().date_naive().with_day(1).unwrap().to_string());
    let mut lease = use_signal(String::new);
    let mut charge_code = use_signal(|| "CHARGES".to_string());
    let mut actual = use_signal(String::new);
    let mut tax_lease = use_signal(String::new);
    let mut tax_code = use_signal(|| "TAXE_FONCIERE".to_string());
    let mut tax_label = use_signal(|| "Taxe récupérable".to_string());
    let mut tax_basis = use_signal(String::new);
    let mut tax_rate = use_signal(|| "100".to_string());
    let mut tax_amount = use_signal(String::new);
    let mut credit_invoice = use_signal(String::new);
    let mut credit_amount = use_signal(String::new);
    let mut credit_reason = use_signal(String::new);
    let mut msg = use_signal(String::new);
    let mut provider = use_signal(String::new);
    let mut provider_name = use_signal(String::new);
    let mut provider_format = use_signal(|| "MIXED".to_string());
    let mut endpoint = use_signal(String::new);

    rsx! {
        ModuleHeader {
            title: "Facturation",
            kicker: "LOYERS • CHARGES • RÉGULARISATIONS • AVOIRS",
            detail: "La génération part du bail et reste en brouillon jusqu’à validation. Aucun brouillon n’a d’effet externe."
        }
        section {
            class: "two-col",
            div {
                class: "panel",
                h3 { "Génération automatique" }
                FormField { label: "Mois à facturer (AAAA-MM-JJ)", value: period(), oninput: move |e: FormEvent| period.set(e.value()) }
                button {
                    class: "primary",
                    onclick: move |_| {
                        let value = period();
                        async move {
                            match NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
                                Ok(date) => match generate_monthly_billing(date).await {
                                    Ok(r) => { msg.set(format!("Génération : {} créée, {} déjà présentes, {} ignorées.", r.generated, r.already_present, r.skipped)); bump += 1; }
                                    Err(e) => msg.set(e.to_string()),
                                },
                                Err(_) => msg.set("Date de mois invalide".into()),
                            }
                        }
                    },
                    "Générer les loyers + charges + taxes du mois"
                }
            }
            div {
                class: "panel",
                h3 { "Régularisation de charges" }
                label {
                    class: "field",
                    span { "Bail" }
                    select {
                        value: lease(),
                        onchange: move |e: FormEvent| lease.set(e.value()),
                        option { value: "", "Sélectionner" }
                        for l in leases.read().as_deref().unwrap_or(&[]).iter() {
                            option { value: l.id.to_string(), "{l.reference} • {l.tenant_name}" }
                        }
                    }
                }
                FormField { label: "Période", value: period(), oninput: move |e: FormEvent| period.set(e.value()) }
                FormField { label: "Code charge", value: charge_code(), oninput: move |e: FormEvent| charge_code.set(e.value()) }
                FormField { label: "Réel €", value: actual(), oninput: move |e: FormEvent| actual.set(e.value()) }
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let lease_raw = lease();
                        let period_raw = period();
                        let code = charge_code();
                        let actual_raw = actual();
                        async move {
                            let parsed = Uuid::parse_str(&lease_raw);
                            let date = NaiveDate::parse_from_str(&period_raw, "%Y-%m-%d");
                            let amount = actual_raw.replace(',', ".").parse::<f64>().ok().map(|v| (v * 100.0).round() as i64);
                            match (parsed, date, amount) {
                                (Ok(lease_id), Ok(date), Some(amount)) => match add_charge_actual(lease_id, date, code, amount, "Saisie utilisateur".into()).await {
                                    Ok(_) => match calculate_charge_regularization(lease_id, date, charge_code()).await {
                                        Ok(r) => { msg.set(format!("Régularisation : écart {} € → {}", euro_local(r.difference_cents), status_regularization(&r.treatment))); bump += 1; }
                                        Err(e) => msg.set(e.to_string()),
                                    },
                                    Err(e) => msg.set(e.to_string()),
                                },
                                _ => msg.set("Bail, période ou réel invalide".into()),
                            }
                        }
                    },
                    "Calculer la régularisation"
                }
            }
        }
        section {
            class: "two-col",
            div {
                class: "panel",
                h3 { "Taxes récupérables" }
                label {
                    class: "field",
                    span { "Bail" }
                    select {
                        value: tax_lease(),
                        onchange: move |e: FormEvent| tax_lease.set(e.value()),
                        option { value: "", "Sélectionner" }
                        for l in leases.read().as_deref().unwrap_or(&[]).iter() {
                            option { value: l.id.to_string(), "{l.reference} • {l.tenant_name}" }
                        }
                    }
                }
                FormField { label: "Code fiscal", value: tax_code(), oninput: move |e: FormEvent| tax_code.set(e.value()) }
                FormField { label: "Libellé", value: tax_label(), oninput: move |e: FormEvent| tax_label.set(e.value()) }
                FormField { label: "Base juridique", value: tax_basis(), oninput: move |e: FormEvent| tax_basis.set(e.value()) }
                FormField { label: "Récupérable %", value: tax_rate(), oninput: move |e: FormEvent| tax_rate.set(e.value()) }
                FormField { label: "Réel de taxe € (facultatif)", value: tax_amount(), oninput: move |e: FormEvent| tax_amount.set(e.value()) }
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let lease_raw = tax_lease();
                        let rate_raw = tax_rate();
                        let code = tax_code();
                        let label = tax_label();
                        let basis = tax_basis();
                        let period_raw = period();
                        let amount_raw = tax_amount();
                        async move {
                            let lease_id = Uuid::parse_str(&lease_raw);
                            let rate = rate_raw.replace(',', ".").parse::<f64>().ok().map(|v| (v * 100.0).round() as i32);
                            let date = NaiveDate::parse_from_str(&period_raw, "%Y-%m-%d").unwrap_or_else(|_| Utc::now().date_naive());
                            match (lease_id, rate) {
                                (Ok(lease_id), Some(rate)) => match add_recoverable_tax_rule(lease_id, code, label, basis, rate, date, None).await {
                                    Ok(rule) => {
                                        if let Some(value) = amount_raw.replace(',', ".").parse::<f64>().ok().map(|v| (v * 100.0).round() as i64) {
                                            let _ = add_recoverable_tax_assessment(rule, lease_id, date, value, "Saisie utilisateur".into()).await;
                                        }
                                        msg.set("Règle de taxe enregistrée".into()); bump += 1;
                                    }
                                    Err(e) => msg.set(e.to_string()),
                                },
                                _ => msg.set("Bail ou taux invalide".into()),
                            }
                        }
                    },
                    "Enregistrer la règle + réel"
                }
            }
            div {
                class: "panel",
                h3 { "Avoir / correction traçable" }
                label {
                    class: "field",
                    span { "Facture source" }
                    select {
                        value: credit_invoice(),
                        onchange: move |e: FormEvent| credit_invoice.set(e.value()),
                        option { value: "", "Sélectionner" }
                        for i in invoices.read().as_deref().unwrap_or(&[]).iter().filter(|i| i.document_kind == "INVOICE" && i.status_code != "CANCELLED" && i.status_code != "CREDITED") {
                            option { value: i.id.to_string(), "{i.invoice_number} • {i.tenant_name} • {i.status_fr}" }
                        }
                    }
                }
                FormField { label: "Montant TTC € (vide = total)", value: credit_amount(), oninput: move |e: FormEvent| credit_amount.set(e.value()) }
                FormField { label: "Motif", value: credit_reason(), oninput: move |e: FormEvent| credit_reason.set(e.value()) }
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let invoice = credit_invoice();
                        let amount_raw = credit_amount();
                        let reason = credit_reason();
                        async move {
                            let id = Uuid::parse_str(&invoice);
                            let amount = if amount_raw.trim().is_empty() { None } else { amount_raw.replace(',', ".").parse::<f64>().ok().map(|v| (v * 100.0).round() as i64) };
                            match id {
                                Ok(id) => match create_credit_note(id, amount, reason).await {
                                    Ok(cid) => { msg.set(format!("Avoir brouillon créé : {cid}")); bump += 1; }
                                    Err(e) => msg.set(e.to_string()),
                                },
                                Err(_) => msg.set("Facture source invalide".into()),
                            }
                        }
                    },
                    "Créer l’avoir (brouillon)"
                }
            }
        }
        section {
            class: "panel",
            h3 { "Nouvelle facturation électronique" }
            div {
                class: "facts-row",
                InfoTileOwned { label: "Modèle", value: "Plateforme interchangeable".to_string() }
                InfoTileOwned { label: "Réception", value: "Depuis 01/09/2026".to_string() }
                InfoTileOwned { label: "Émission PME/TPE", value: "Au plus tard 01/09/2027".to_string() }
            }
            div {
                class: "form-grid",
                FormField { label: "Code plateforme", value: provider(), oninput: move |e: FormEvent| provider.set(e.value()) }
                FormField { label: "Nom plateforme", value: provider_name(), oninput: move |e: FormEvent| provider_name.set(e.value()) }
                FormField { label: "Format", value: provider_format(), oninput: move |e: FormEvent| provider_format.set(e.value()) }
                FormField { label: "Référence endpoint", value: endpoint(), oninput: move |e: FormEvent| endpoint.set(e.value()) }
            }
            button {
                class: "secondary",
                onclick: move |_| async move {
                    match set_einvoice_provider(provider(), provider_name(), provider_format(), endpoint(), Utc::now().date_naive()).await {
                        Ok(_) => { msg.set("Plateforme activée. L’ancienne reste historisée.".into()); bump += 1; }
                        Err(e) => msg.set(e.to_string()),
                    }
                },
                "Activer cette plateforme"
            }
            for c in configs.read().as_deref().unwrap_or(&[]).iter() {
                div {
                    class: "data-row",
                    div {
                        div { class: "data-title", "{c.provider_name} • {c.format_code}" }
                        div { class: "small", "{c.effective_from} → {format_effective_to(c.effective_to)}" }
                    }
                    div { class: "row-value", if c.active { "ACTIVE" } else { "HISTORIQUE" } }
                }
            }
        }
        span { class: "save-ok", "{msg}" }
        section {
            class: "panel",
            h3 { "Factures" }
            for i in invoices.read().as_deref().unwrap_or(&[]).iter() {
                BillingInvoiceRow { item: i.clone(), bump }
            }
        }
    }
}

fn invoice_title(is_credit: bool, number: &str) -> String {
    if is_credit { format!("AVOIR {number}") } else { number.to_owned() }
}

#[component]
fn BillingInvoiceRow(item: BillingInvoiceItem, mut bump: Signal<u64>) -> Element {
    let id = item.id;
    let is_draft = item.status_code == "DRAFT";
    let is_validated = item.status_code == "VALIDATED";
    let is_credit = item.document_kind == "CREDIT_NOTE";
    rsx! {
        div {
            class: "data-row",
            div {
                div { class: "data-title", "{invoice_title(is_credit, &item.invoice_number)}" }
                div { class: "small", "{item.tenant_name} • {item.status_fr} • période {format_period_start(item.period_start)}" }
                div { class: "small", "HT {euro_local(item.net_cents)} • TVA {euro_local(item.vat_cents)} • TTC {euro_local(item.gross_cents)} • payé {euro_local(item.paid_cents)}" }
                if !item.pdf_path.is_empty() { div { class: "small", "PDF : {item.pdf_path}" } }
                if !item.electronic_status.is_empty() { div { class: "small", "E-facture : {item.electronic_status} {item.electronic_provider}" } }
            }
            div {
                class: "row-actions",
                if is_draft {
                    button {
                        class: "secondary",
                        onclick: move |_| async move { let _ = validate_billing_invoice(id).await; bump += 1; },
                        "Valider"
                    }
                }
                if is_validated {
                    button {
                        class: "secondary",
                        onclick: move |_| async move { let _ = issue_billing_invoice(id).await; bump += 1; },
                        "Émettre"
                    }
                }
                if !matches!(item.status_code.as_str(), "CANCELLED" | "CREDITED") {
                    button {
                        class: "secondary",
                        onclick: move |_| async move { let _ = cancel_billing_invoice(id, "Annulation demandée dans la gestion des factures".into()).await; bump += 1; },
                        "Annuler"
                    }
                }
                if !is_credit && item.status_code != "DRAFT" {
                    button {
                        class: "secondary",
                        onclick: move |_| async move { let _ = prepare_einvoice(id).await; bump += 1; },
                        "Préparer e-facture"
                    }
                }
            }
        }
    }
}

fn euro_local(cents: i64) -> String { format!("{}.{:02} €", cents / 100, cents.abs() % 100) }
fn status_regularization(v: &str) -> String { match v { "ADDITIONAL_INVOICE" => "facture complémentaire".into(), "CREDIT_NOTE" => "avoir".into(), "NO_DIFFERENCE" => "aucun écart".into(), _ => "traitement à préciser".into() } }


#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn french_statuses(){assert_eq!(invoice_status_fr("DRAFT"),"Brouillon");assert_eq!(invoice_status_fr("PAID_PARTIAL"),"Partiellement payée");assert_eq!(invoice_status_fr("CREDITED"),"Créditée");}
    #[test]fn prorata_month(){let s=NaiveDate::from_ymd_opt(2026,9,1).unwrap();let e=NaiveDate::from_ymd_opt(2026,9,30).unwrap();assert_eq!(prorated_amount(30000,s,None,s,e),30000);assert_eq!(prorated_amount(30000,NaiveDate::from_ymd_opt(2026,9,16).unwrap(),None,s,e),15000);}
    #[test]fn due_date_clamped(){assert_eq!(due_date_for_month(NaiveDate::from_ymd_opt(2026,2,1).unwrap(),31),NaiveDate::from_ymd_opt(2026,2,28).unwrap());}
    #[test]fn prorata_never_exceeds_full_rent(){let s=NaiveDate::from_ymd_opt(2026,4,1).unwrap();let e=NaiveDate::from_ymd_opt(2026,4,30).unwrap();for day in 1..=30{let d=NaiveDate::from_ymd_opt(2026,4,day).unwrap();assert!(prorated_amount(10000,d,None,s,e)<=10000);}}
}
