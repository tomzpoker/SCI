use crate::domain::{
    LeaseChargeItem, LeaseClauseItem, LeaseDepositItem, LeaseDetailItem,
    LeaseGuaranteeItem, LeaseIndexItem, LeaseReductionItem, LeaseRentRevisionItem,
};
use crate::entity_scope::current_legal_entity_id;
use crate::ui::{FormField, ModuleHeader};
use chrono::{Datelike, Duration, NaiveDate};
use dioxus::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

#[cfg(feature = "server")]
use sqlx::Row;

fn parse_optional_date(value: &str) -> Result<Option<NaiveDate>, String> {
    if value.trim().is_empty() { return Ok(None); }
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map(Some)
        .map_err(|_| "Date invalide".to_owned())
}

fn valid_charge_mode(v: &str) -> bool {
    matches!(v, "NONE" | "FORFAIT" | "PROVISION" | "PROVISION_VARIABLE" | "REGULARISATION")
}

fn valid_frequency(v: &str) -> bool {
    matches!(v, "MONTHLY" | "QUARTERLY" | "ANNUAL")
}

fn valid_vat_mode(v: &str) -> bool {
    matches!(v, "FROM_UNIT" | "EXONERATED" | "OPTION" | "CUSTOM")
}

fn period_to_abs(label: &str) -> Option<i32> {
    let parts: Vec<&str> = label.split_whitespace().collect();
    if parts.len() != 2 { return None; }
    let q = parts[0].trim_start_matches('T').parse::<i32>().ok()?;
    let y = parts[1].parse::<i32>().ok()?;
    if !(1..=4).contains(&q) { return None; }
    Some(y * 4 + (q - 1))
}

fn date_to_period_abs(d: NaiveDate) -> i32 {
    d.year() * 4 + ((d.month() as i32 - 1) / 3)
}

fn quarter_end_date(period_label: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = period_label.split_whitespace().collect();
    if parts.len() != 2 { return None; }
    let q = parts[0].trim_start_matches('T').parse::<u32>().ok()?;
    let y = parts[1].parse::<i32>().ok()?;
    let (m, d): (u32, u32) = match q {
        1 => (3, 31), 2 => (6, 30), 3 => (9, 30), 4 => (12, 31),
        _ => return None,
    };
    NaiveDate::from_ymd_opt(y, m, d)
}

fn publication_date(period_label: &str, pub_day: u32, pub_month_offset: u32) -> Option<NaiveDate> {
    let end = quarter_end_date(period_label)?;
    let mut y = end.year();
    let mut m = end.month() + pub_month_offset;
    while m > 12 { m -= 12; y += 1; }
    let d = pub_day.min(28);
    NaiveDate::from_ymd_opt(y, m, d)
}

fn expected_period_for_date(date: NaiveDate, pub_day: u32, pub_offset: u32) -> Option<String> {
    let mut q = ((date.month() as i32 - 1) / 3) + 1;
    let mut y = date.year();
    for _ in 0..16 {
        let label = format!("T{} {}", q, y);
        if let Some(pd) = publication_date(&label, pub_day, pub_offset) {
            if pd <= date { return Some(label); }
        }
        q -= 1;
        if q < 1 { q = 4; y -= 1; }
    }
    None
}

fn next_anniversary(effect_date: NaiveDate, today: NaiveDate, period_months: u32) -> NaiveDate {
    if today <= effect_date { return effect_date; }
    let years = (period_months / 12).max(1);
    let mut n: i32 = 0;
    loop {
        n += years as i32;
        let y = effect_date.year() + n;
        let cand = NaiveDate::from_ymd_opt(y, effect_date.month(), effect_date.day())
            .or_else(|| NaiveDate::from_ymd_opt(y, effect_date.month(), 28));
        if let Some(c) = cand {
            if c > today { return c; }
        } else {
            return effect_date;
        }
        if n > 400 { return effect_date; }
    }
}

fn most_recent_anniversary(effect_date: NaiveDate, today: NaiveDate, period_months: u32) -> NaiveDate {
    if today <= effect_date { return effect_date; }
    let next = next_anniversary(effect_date, today, period_months);
    let years = (period_months / 12).max(1);
    let y = next.year() - years as i32;
    NaiveDate::from_ymd_opt(y, next.month(), next.day())
        .or_else(|| NaiveDate::from_ymd_opt(y, next.month(), 28))
        .unwrap_or(effect_date)
}

pub fn calculate_indexed_rent(
    old_rent_cents: i64,
    old_index: Decimal,
    new_index: Decimal,
    cap_bp: Option<i32>,
) -> Result<i64, String> {
    if old_rent_cents < 0 || old_index <= Decimal::ZERO || new_index <= Decimal::ZERO {
        return Err("Paramètres d'indexation invalides".to_owned());
    }
    let raw = Decimal::from(old_rent_cents) * new_index / old_index;
    let mut capped = raw;
    if let Some(max_bp) = cap_bp {
        if max_bp < 0 || max_bp > 10000 {
            return Err("Plafond invalide".to_owned());
        }
        let ceiling = Decimal::from(old_rent_cents) * Decimal::from(10000 + max_bp as i64)
            / Decimal::from(10000);
        if raw > ceiling { capped = ceiling; }
    }
    capped.round().to_i64().ok_or_else(|| "Résultat d'indexation invalide".to_owned())
}

// ============================================================
//  LECTURE BAUX
// ============================================================

#[server]
pub async fn list_lease_details() -> Result<Vec<LeaseDetailItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT l.id, l.reference, l.unit_id, u.label unit_label, p.name property_name,
                l.tenant_id, t.legal_name tenant_name, l.signature_date,
                l.start_date effect_date, l.end_date, l.lease_type, l.destination,
                COALESCE(NULLIF(l.current_rent_cents,0), l.rent_amount_cents) rent_amount_cents,
                l.rent_frequency, l.payment_day, l.vat_mode,
                COALESCE(l.index_code,'') index_code, l.index_base_value, l.index_base_date,
                l.index_cap_bp, l.charges_mode, l.charges_amount_cents,
                l.security_deposit_expected_cents, l.entry_fee_expected_cents,
                l.entry_fee_status, l.active,
                l.revision_period_months, l.index_publication_day,
                l.index_publication_month_offset, l.revision_application_mode
             FROM leases l
             JOIN units u ON u.id = l.unit_id
             JOIN properties p ON p.id = u.property_id
             JOIN tenants t ON t.id = l.tenant_id
             WHERE l.legal_entity_id = $1
             ORDER BY l.active DESC, l.start_date DESC",
        )
        .bind(current_legal_entity_id())
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| LeaseDetailItem {
            id: r.get("id"), reference: r.get("reference"),
            unit_id: r.get("unit_id"), unit_label: r.get("unit_label"),
            property_name: r.get("property_name"), tenant_id: r.get("tenant_id"),
            tenant_name: r.get("tenant_name"), signature_date: r.get("signature_date"),
            effect_date: r.get("effect_date"), end_date: r.get("end_date"),
            lease_type: r.get("lease_type"), destination: r.get("destination"),
            rent_amount_cents: r.get("rent_amount_cents"), rent_frequency: r.get("rent_frequency"),
            payment_day: r.get("payment_day"), vat_mode: r.get("vat_mode"),
            index_code: r.get("index_code"), index_base_value: r.get("index_base_value"),
            index_base_date: r.get("index_base_date"), index_cap_bp: r.get("index_cap_bp"),
            charges_mode: r.get("charges_mode"), charges_amount_cents: r.get("charges_amount_cents"),
            security_deposit_expected_cents: r.get("security_deposit_expected_cents"),
            entry_fee_expected_cents: r.get("entry_fee_expected_cents"),
            entry_fee_status: r.get("entry_fee_status"), active: r.get("active"),
            revision_period_months: r.get("revision_period_months"),
            index_publication_day: r.get("index_publication_day"),
            index_publication_month_offset: r.get("index_publication_month_offset"),
            revision_application_mode: r.get("revision_application_mode"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    { Err(ServerFnError::new("list_lease_details est exécutée côté serveur")) }
}

// ============================================================
//  CRÉATION BAUX
// ============================================================

#[server]
pub async fn create_structured_lease(
    unit_id: Uuid, tenant_id: Uuid, reference: String,
    signature_date: Option<NaiveDate>, effect_date: NaiveDate, end_date: Option<NaiveDate>,
    lease_type: String, destination: String, rent_amount_cents: i64,
    rent_frequency: String, payment_day: i16, vat_mode: String,
    index_code: String, index_base_value: Option<Decimal>, index_base_date: Option<NaiveDate>,
    index_cap_bp: Option<i32>, charges_mode: String, charges_amount_cents: i64,
    security_deposit_expected_cents: i64, entry_fee_expected_cents: i64,
    revision_period_months: i32,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();

        if reference.trim().is_empty() { return Err(ServerFnError::new("Référence requise")); }
        if end_date.map(|d| d < effect_date).unwrap_or(false) {
            return Err(ServerFnError::new("Échéance avant la date d'effet"));
        }
        if rent_amount_cents < 0 || charges_amount_cents < 0
            || security_deposit_expected_cents < 0 || entry_fee_expected_cents < 0 {
            return Err(ServerFnError::new("Montants invalides"));
        }
        if !valid_frequency(rent_frequency.trim()) || !valid_vat_mode(vat_mode.trim())
            || !valid_charge_mode(charges_mode.trim()) {
            return Err(ServerFnError::new("Paramètre de bail invalide"));
        }
        if payment_day < 1 || payment_day > 31 {
            return Err(ServerFnError::new("Jour de paiement invalide"));
        }
        if index_cap_bp.map(|v| v < 0 || v > 10000).unwrap_or(false) {
            return Err(ServerFnError::new("Plafond d'indexation invalide"));
        }
        if !matches!(revision_period_months, 12 | 24 | 36) {
            return Err(ServerFnError::new("Périodicité de révision invalide (12, 24 ou 36)"));
        }

        let valid: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM units u
             JOIN properties p ON p.id = u.property_id
             WHERE u.id = $1 AND p.legal_entity_id = $3
               AND EXISTS (SELECT 1 FROM tenants WHERE id = $2 AND legal_entity_id = $3)",
        )
        .bind(unit_id).bind(tenant_id).bind(entity)
        .fetch_one(pool).await.map_err(ServerFnError::new)?;
        if valid != 1 { return Err(ServerFnError::new("Lot ou locataire invalide")); }

        let rent = if rent_amount_cents == 0 {
            sqlx::query_scalar::<_, i64>("SELECT base_rent_cents FROM units WHERE id = $1")
                .bind(unit_id).fetch_one(pool).await.map_err(ServerFnError::new)?
        } else { rent_amount_cents };

        if rent <= 0 {
            return Err(ServerFnError::new(
                "Le loyer doit être supérieur à 0. Renseigne un montant dans le champ \
                 « Loyer € » ou configure le loyer de base du lot avant de créer le bail."
            ));
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO leases(
                legal_entity_id, unit_id, tenant_id, reference, start_date, end_date,
                notice_months, payment_day, annual_review_month, active, signature_date,
                lease_type, destination, rent_amount_cents, current_rent_cents, rent_frequency,
                vat_mode, index_code, index_base_value, index_base_date, index_cap_bp,
                charges_mode, charges_amount_cents, security_deposit_expected_cents,
                entry_fee_expected_cents, entry_fee_status, updated_at,
                revision_period_months, index_publication_day, index_publication_month_offset
             )
             VALUES($1,$2,$3,$4,$5,$6,3,$7,NULL,true,$8,$9,$10,$11,$11,$12,$13,
                NULLIF($14,''),$15,$16,$17,$18,$19,$20,$21,
                CASE WHEN $21 > 0 THEN 'UNCERTAIN' ELSE 'NOT_SET' END, now(),
                $22, 12, 3)
             RETURNING id",
        )
        .bind(entity).bind(unit_id).bind(tenant_id).bind(reference.trim())
        .bind(effect_date).bind(end_date).bind(payment_day).bind(signature_date)
        .bind(lease_type.trim()).bind(destination.trim()).bind(rent)
        .bind(rent_frequency.trim()).bind(vat_mode.trim()).bind(index_code.trim())
        .bind(index_base_value).bind(index_base_date).bind(index_cap_bp)
        .bind(charges_mode.trim()).bind(charges_amount_cents)
        .bind(security_deposit_expected_cents).bind(entry_fee_expected_cents)
        .bind(revision_period_months)
        .fetch_one(pool).await.map_err(ServerFnError::new)?;

        sqlx::query(
            "INSERT INTO lease_contract_events(legal_entity_id, lease_id, event_type,
                effective_date, before_payload, after_payload, reason)
             VALUES($1,$2,'LEASE_CREATED',$3,'{}'::jsonb,$4,'Création structurée du bail')",
        )
        .bind(entity).bind(id).bind(effect_date)
        .bind(json!({
            "reference": reference, "rent_amount_cents": rent,
            "rent_frequency": rent_frequency, "vat_mode": vat_mode,
            "charges_mode": charges_mode,
            "security_deposit_expected_cents": security_deposit_expected_cents,
            "entry_fee_expected_cents": entry_fee_expected_cents,
            "revision_period_months": revision_period_months
        }))
        .execute(pool).await.map_err(ServerFnError::new)?;

        if entry_fee_expected_cents > 0 {
            sqlx::query(
                "INSERT INTO lease_entry_fees(legal_entity_id, lease_id, expected_amount_cents,
                    qualification_status, justification, alert_required)
                 VALUES($1,$2,$3,'UNCERTAIN','Qualification à vérifier selon la situation et les pièces du dossier',true)",
            )
            .bind(entity).bind(id).bind(entry_fee_expected_cents)
            .execute(pool).await.map_err(ServerFnError::new)?;

            sqlx::query(
                "INSERT INTO tasks(legal_entity_id, code, title, description, due_at, state,
                    priority, source, blocking, entity_type, entity_id, occurrence_key)
                 VALUES($1,'LEASE_ENTRY_FEE_CHECK','Vérifier le droit d''entrée',
                    'Qualification incertaine : vérifier le cadre juridique et les pièces.',
                    now(),'READY',85,'LEASE_ENTRY_FEE',true,'LEASE',$2,$3)
                 ON CONFLICT(legal_entity_id,code,occurrence_key) DO NOTHING",
            )
            .bind(entity).bind(id).bind(format!("ENTRY_FEE:{}", id))
            .execute(pool).await.map_err(ServerFnError::new)?;
        }
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = (unit_id, tenant_id, reference, signature_date, effect_date, end_date,
            lease_type, destination, rent_amount_cents, rent_frequency, payment_day,
            vat_mode, index_code, index_base_value, index_base_date, index_cap_bp,
            charges_mode, charges_amount_cents, security_deposit_expected_cents,
            entry_fee_expected_cents, revision_period_months);
        Err(ServerFnError::new("create_structured_lease est exécutée côté serveur"))
    }
}

// ============================================================
//  CLAUSES
// ============================================================

#[server]
pub async fn list_lease_clauses(lease_id: Uuid) -> Result<Vec<LeaseClauseItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT id, lease_id, code, title, clause_type, body, effective_from,
                    effective_to, version_no, active
             FROM lease_clauses
             WHERE lease_id = $1 AND legal_entity_id = $2
             ORDER BY active DESC, code, version_no DESC",
        )
        .bind(lease_id).bind(current_legal_entity_id())
        .fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| LeaseClauseItem {
            id: r.get("id"), lease_id: r.get("lease_id"), code: r.get("code"),
            title: r.get("title"), clause_type: r.get("clause_type"), body: r.get("body"),
            effective_from: r.get("effective_from"), effective_to: r.get("effective_to"),
            version_no: r.get("version_no"), active: r.get("active"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    { Err(ServerFnError::new("list_lease_clauses est exécutée côté serveur")) }
}

#[server]
pub async fn save_lease_clause(
    lease_id: Uuid, code: String, title: String, clause_type: String,
    body: String, effective_from: NaiveDate, effective_to: Option<NaiveDate>,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        if code.trim().is_empty() || title.trim().is_empty() || body.trim().is_empty() {
            return Err(ServerFnError::new("Code, titre et clause requis"));
        }
        if effective_to.map(|d| d < effective_from).unwrap_or(false) {
            return Err(ServerFnError::new("Fin de clause invalide"));
        }
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM leases WHERE id = $1 AND legal_entity_id = $2)",
        ).bind(lease_id).bind(entity).fetch_one(pool).await.map_err(ServerFnError::new)?;
        if !exists { return Err(ServerFnError::new("Bail introuvable")); }

        let version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_no),0)+1 FROM lease_clauses
             WHERE lease_id = $1 AND code = $2 AND legal_entity_id = $3",
        ).bind(lease_id).bind(code.trim()).bind(entity)
        .fetch_one(pool).await.map_err(ServerFnError::new)?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_clauses(legal_entity_id, lease_id, code, title, clause_type,
                body, effective_from, effective_to, version_no, active)
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,true) RETURNING id",
        )
        .bind(entity).bind(lease_id).bind(code.trim()).bind(title.trim())
        .bind(clause_type.trim()).bind(body.trim()).bind(effective_from)
        .bind(effective_to).bind(version)
        .fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = (lease_id, code, title, clause_type, body, effective_from, effective_to);
        Err(ServerFnError::new("save_lease_clause est exécutée côté serveur"))
    }
}

// ============================================================
//  INDICES
// ============================================================

#[server]
pub async fn list_lease_indices(index_code: String) -> Result<Vec<LeaseIndexItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT id, index_code, period_label, value, source_reference, verified_at
             FROM lease_index_values
             WHERE legal_entity_id = $1 AND ($2 = '' OR index_code = $2)
             ORDER BY index_code,
                      CAST(SUBSTRING(period_label FROM '\\d{4}') AS int) DESC,
                      SUBSTRING(period_label FROM 'T\\d') DESC",
        )
        .bind(current_legal_entity_id()).bind(index_code.trim())
        .fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| LeaseIndexItem {
            id: r.get("id"), index_code: r.get("index_code"),
            period_label: r.get("period_label"), value: r.get("value"),
            source_reference: r.get("source_reference"), verified_at: r.get("verified_at"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    { Err(ServerFnError::new("list_lease_indices est exécutée côté serveur")) }
}

#[server]
pub async fn save_lease_index(
    index_code: String, period_label: String, value: Decimal,
    source_reference: String, verified_at: Option<NaiveDate>,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        if index_code.trim().is_empty() || period_label.trim().is_empty() {
            return Err(ServerFnError::new("Indice et période requis"));
        }
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_index_values(legal_entity_id, index_code, period_label,
                value, source_reference, verified_at)
             VALUES($1,$2,$3,$4,$5,$6)
             ON CONFLICT(legal_entity_id,index_code,period_label)
             DO UPDATE SET value = EXCLUDED.value,
                source_reference = EXCLUDED.source_reference,
                verified_at = EXCLUDED.verified_at
             RETURNING id",
        )
        .bind(current_legal_entity_id()).bind(index_code.trim().to_uppercase())
        .bind(period_label.trim()).bind(value).bind(source_reference.trim())
        .bind(verified_at).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    {
        let _ = (index_code, period_label, value, source_reference, verified_at);
        Err(ServerFnError::new("save_lease_index est exécutée côté serveur"))
    }
}

// ============================================================
//  RÉVISION — helper interne
// ============================================================

#[cfg(feature = "server")]
struct RevisionCalc {
    old_rent: i64,
    new_rent: i64,
    old_index: Decimal,
    new_index: Decimal,
    code: String,
    cap: Option<i32>,
    formula: String,
    rule_text: String,
    vat_rate_bp: i32,
}

#[cfg(feature = "server")]
async fn compute_revision_internal(
    lease_id: Uuid,
    clause_code: &str,
    index_period: &str,
) -> Result<RevisionCalc, String> {
    let pool = crate::infrastructure::db().await.map_err(|e| e.to_string())?;
    let entity = current_legal_entity_id();

    let lease = sqlx::query(
        "SELECT COALESCE(NULLIF(l.current_rent_cents,0), l.rent_amount_cents) AS rent,
                COALESCE(l.index_code,'') AS index_code, l.index_base_value,
                l.index_base_date, l.start_date AS effect_date,
                l.index_cap_bp, u.vat_rate_bp,
                l.index_publication_day, l.index_publication_month_offset
         FROM leases l JOIN units u ON u.id = l.unit_id
         WHERE l.id = $1 AND l.legal_entity_id = $2",
    )
    .bind(lease_id).bind(entity)
    .fetch_optional(pool).await.map_err(|e| e.to_string())?
    .ok_or_else(|| "Bail introuvable".to_string())?;

    let old_rent: i64 = lease.get("rent");
    let lease_index_code: String = lease.get("index_code");
    let index_base_date_opt: Option<NaiveDate> = lease.try_get("index_base_date").ok().flatten();
    let effect_date: NaiveDate = lease.get("effect_date");
    let cap: Option<i32> = lease.get("index_cap_bp");
    let vat_rate_bp: i32 = lease.get("vat_rate_bp");
    let pub_day: i32 = lease.get("index_publication_day");
    let pub_offset: i32 = lease.get("index_publication_month_offset");
    let pub_day_u = pub_day.max(1) as u32;
    let pub_offset_u = pub_offset.max(1) as u32;

    let period_clean = index_period.trim();
    if period_clean.is_empty() {
        return Err("Sélectionne une période d'indice avant de calculer.".into());
    }

    let code: String = if !lease_index_code.is_empty() {
        lease_index_code.clone()
    } else {
        let r = sqlx::query(
            "SELECT index_code FROM lease_index_values
             WHERE legal_entity_id = $1 AND period_label = $2
             ORDER BY index_code LIMIT 1",
        )
        .bind(entity).bind(period_clean)
        .fetch_optional(pool).await.map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Aucun indice trouvé pour la période {period_clean}."))?;
        r.get("index_code")
    };

    let stored_base: Option<Decimal> = lease.try_get("index_base_value").ok().flatten();
    let old_index: Decimal = match stored_base {
        Some(v) if v > Decimal::ZERO => v,
        _ => {
            let ref_date = index_base_date_opt.unwrap_or(effect_date);
            let rows = sqlx::query(
                "SELECT period_label, value FROM lease_index_values
                 WHERE legal_entity_id = $1 AND index_code = $2",
            )
            .bind(entity).bind(&code)
            .fetch_all(pool).await.map_err(|e| e.to_string())?;

            let mut best: Option<(NaiveDate, Decimal)> = None;
            for r in rows {
                let pl: String = r.get("period_label");
                let v: Decimal = r.get("value");
                if let Some(pd) = publication_date(&pl, pub_day_u, pub_offset_u) {
                    if pd <= ref_date {
                        match best {
                            None => best = Some((pd, v)),
                            Some((bd, _)) if pd > bd => best = Some((pd, v)),
                            _ => {}
                        }
                    }
                }
            }
            best.map(|(_, v)| v).ok_or_else(|| format!(
                "Impossible de déterminer l'indice de base : aucune valeur {} \
                 publiée avant le {}. Importe les valeurs d'indice ou renseigne \
                 l'indice de base manuellement sur le bail.",
                code, ref_date
            ))?
        }
    };

    let row = sqlx::query(
        "SELECT value FROM lease_index_values
         WHERE legal_entity_id = $1 AND index_code = $2 AND period_label = $3",
    )
    .bind(entity).bind(&code).bind(period_clean)
    .fetch_optional(pool).await.map_err(|e| e.to_string())?
    .ok_or_else(|| format!("Indice {} introuvable pour la période {}.", code, period_clean))?;

    let new_index: Decimal = row.get("value");
    if old_index <= Decimal::ZERO || new_index <= Decimal::ZERO {
        return Err("Indices invalides".into());
    }

    let new_rent = calculate_indexed_rent(old_rent, old_index, new_index, cap)?;
    let formula = if cap.is_some() {
        format!("max(0, min({old_rent} × {new_index} / {old_index}, plafond {cap:?}))")
    } else {
        format!("{old_rent} × {new_index} / {old_index}")
    };

    let rule_text = if clause_code.trim().is_empty() {
        "Indexation contractuelle".to_owned()
    } else {
        sqlx::query_scalar::<_, String>(
            "SELECT body FROM lease_clauses
             WHERE legal_entity_id = $1 AND lease_id = $2 AND code = $3 AND active
             ORDER BY version_no DESC LIMIT 1",
        )
        .bind(entity).bind(lease_id).bind(clause_code.trim())
        .fetch_optional(pool).await.map_err(|e| e.to_string())?
        .unwrap_or_else(|| "Clause sélectionnée".to_owned())
    };

    Ok(RevisionCalc {
        old_rent, new_rent, old_index, new_index, code, cap, formula, rule_text, vat_rate_bp,
    })
}

// ============================================================
//  RÉVISION — preview / apply / refuse
// ============================================================

#[server]
pub async fn preview_lease_rent_revision(
    lease_id: Uuid, clause_code: String, index_period: String, effective_date: NaiveDate,
) -> Result<LeaseRentRevisionItem, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let c = compute_revision_internal(lease_id, &clause_code, &index_period)
            .await.map_err(ServerFnError::new)?;
        Ok(LeaseRentRevisionItem {
            id: Uuid::nil(), lease_id,
            calculation_date: chrono::Utc::now().date_naive(),
            effective_date, rule_text: c.rule_text, index_code: c.code,
            index_period: index_period.trim().to_owned(),
            old_rent_cents: c.old_rent, index_old: Some(c.old_index),
            index_new: Some(c.new_index), cap_bp: c.cap,
            new_rent_cents: c.new_rent, formula: c.formula,
            result_status: "PREVIEW".into(),
        })
    }
    #[cfg(not(feature = "server"))]
    { let _ = (lease_id, clause_code, index_period, effective_date);
      Err(ServerFnError::new("preview_lease_rent_revision est exécutée côté serveur")) }
}

#[server]
pub async fn apply_lease_rent_revision(
    lease_id: Uuid, clause_code: String, index_period: String,
    effective_date: NaiveDate, override_rent_cents: Option<i64>,
    note: String, with_catching: bool,
) -> Result<LeaseRentRevisionItem, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let c = compute_revision_internal(lease_id, &clause_code, &index_period)
            .await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;

        let final_rent = override_rent_cents.unwrap_or(c.new_rent);
        if final_rent < 0 { return Err(ServerFnError::new("Loyer révisé invalide")); }
        let is_adjusted = override_rent_cents.is_some()
            && override_rent_cents.map(|v| v != c.new_rent).unwrap_or(false);
        let status = if is_adjusted { "ADJUSTED" } else { "VALIDATED" };

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_rent_revisions(
                legal_entity_id, lease_id, calculation_date, effective_date,
                rule_text, index_code, index_period, old_rent_cents,
                index_old, index_new, cap_bp, new_rent_cents, formula,
                result_status, status, note, validated_at, override_rent_cents)
             VALUES($1,$2,CURRENT_DATE,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,
                $13,$13,$14,now(),$15)
             RETURNING id",
        )
        .bind(entity).bind(lease_id).bind(effective_date).bind(&c.rule_text)
        .bind(&c.code).bind(index_period.trim()).bind(c.old_rent)
        .bind(c.old_index).bind(c.new_index).bind(c.cap).bind(final_rent)
        .bind(&c.formula).bind(status).bind(note.trim()).bind(override_rent_cents)
        .fetch_one(pool).await.map_err(ServerFnError::new)?;

        sqlx::query(
            "UPDATE leases SET current_rent_cents = $3, updated_at = now()
             WHERE id = $1 AND legal_entity_id = $2",
        ).bind(lease_id).bind(entity).bind(final_rent)
        .execute(pool).await.map_err(ServerFnError::new)?;

        if with_catching && final_rent > c.old_rent {
            let today = chrono::Utc::now().date_naive();
            let days = (today - effective_date).num_days().max(0);
            if days > 0 {
                let delta_cents = final_rent - c.old_rent;
                let catching_cents = ((delta_cents as f64) * (days as f64) / 30.0).round() as i64;
                if catching_cents > 0 {
                    let net_cents = if c.vat_rate_bp > 0 {
                        (catching_cents as f64 * 10000.0 / (10000.0 + c.vat_rate_bp as f64)).round() as i64
                    } else { catching_cents };
                    let vat_cents = catching_cents - net_cents;

                    let seq: i64 = sqlx::query_scalar(
                        "SELECT COALESCE(MAX(CAST(SUBSTRING(invoice_number FROM '[0-9]+$') AS bigint)),0)+1
                         FROM invoices WHERE legal_entity_id = $1 AND invoice_number LIKE 'RATT-%'",
                    ).bind(entity).fetch_one(pool).await.unwrap_or(1);

                    let invoice_number = format!("RATT-{}-{:04}", effective_date.format("%Y%m"), seq);

                    let invoice_id: Option<Uuid> = sqlx::query_scalar(
                        "INSERT INTO invoices(legal_entity_id, lease_id, invoice_number,
                            issue_date, due_date, service_period_start, service_period_end,
                            net_cents, vat_cents, gross_cents, status)
                         VALUES($1,$2,$3,$4,$4,$5,$6,$7,$8,$9,'DRAFT') RETURNING id",
                    )
                    .bind(entity).bind(lease_id).bind(&invoice_number).bind(today)
                    .bind(effective_date).bind(today).bind(net_cents).bind(vat_cents)
                    .bind(catching_cents)
                    .fetch_optional(pool).await.map_err(ServerFnError::new)?;

                    if let Some(inv_id) = invoice_id {
                        sqlx::query(
                            "UPDATE lease_rent_revisions SET catching_invoice_id = $3
                             WHERE id = $1 AND legal_entity_id = $2",
                        ).bind(id).bind(entity).bind(inv_id)
                        .execute(pool).await.map_err(ServerFnError::new)?;
                    }
                }
            }
        }

        Ok(LeaseRentRevisionItem {
            id, lease_id,
            calculation_date: chrono::Utc::now().date_naive(),
            effective_date, rule_text: c.rule_text, index_code: c.code,
            index_period: index_period.trim().to_owned(),
            old_rent_cents: c.old_rent, index_old: Some(c.old_index),
            index_new: Some(c.new_index), cap_bp: c.cap,
            new_rent_cents: final_rent, formula: c.formula,
            result_status: status.into(),
        })
    }
    #[cfg(not(feature = "server"))]
    { let _ = (lease_id, clause_code, index_period, effective_date,
        override_rent_cents, note, with_catching);
      Err(ServerFnError::new("apply_lease_rent_revision est exécutée côté serveur")) }
}

#[server]
pub async fn refuse_lease_rent_revision(
    lease_id: Uuid, clause_code: String, index_period: String,
    effective_date: NaiveDate, note: String,
) -> Result<LeaseRentRevisionItem, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let c = compute_revision_internal(lease_id, &clause_code, &index_period)
            .await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_rent_revisions(
                legal_entity_id, lease_id, calculation_date, effective_date,
                rule_text, index_code, index_period, old_rent_cents,
                index_old, index_new, cap_bp, new_rent_cents, formula,
                result_status, status, note)
             VALUES($1,$2,CURRENT_DATE,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,
                'REFUSED','REFUSED',$13) RETURNING id",
        )
        .bind(entity).bind(lease_id).bind(effective_date).bind(&c.rule_text)
        .bind(&c.code).bind(index_period.trim()).bind(c.old_rent)
        .bind(c.old_index).bind(c.new_index).bind(c.cap).bind(c.new_rent)
        .bind(&c.formula).bind(note.trim())
        .fetch_one(pool).await.map_err(ServerFnError::new)?;

        Ok(LeaseRentRevisionItem {
            id, lease_id,
            calculation_date: chrono::Utc::now().date_naive(),
            effective_date, rule_text: c.rule_text, index_code: c.code,
            index_period: index_period.trim().to_owned(),
            old_rent_cents: c.old_rent, index_old: Some(c.old_index),
            index_new: Some(c.new_index), cap_bp: c.cap,
            new_rent_cents: c.new_rent, formula: c.formula,
            result_status: "REFUSED".into(),
        })
    }
    #[cfg(not(feature = "server"))]
    { let _ = (lease_id, clause_code, index_period, effective_date, note);
      Err(ServerFnError::new("refuse_lease_rent_revision est exécutée côté serveur")) }
}

// ============================================================
//  HISTORIQUE RÉVISIONS
// ============================================================

#[server]
pub async fn list_lease_revisions(lease_id: Uuid) -> Result<Vec<LeaseRentRevisionItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT id, lease_id, calculation_date, effective_date, rule_text,
                    index_code, index_period, old_rent_cents, index_old, index_new,
                    cap_bp, new_rent_cents, formula,
                    COALESCE(status, result_status, 'CALCULATED') AS status
             FROM lease_rent_revisions
             WHERE lease_id = $1 AND legal_entity_id = $2
             ORDER BY calculation_date DESC, created_at DESC",
        )
        .bind(lease_id).bind(current_legal_entity_id())
        .fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| LeaseRentRevisionItem {
            id: r.get("id"), lease_id: r.get("lease_id"),
            calculation_date: r.get("calculation_date"), effective_date: r.get("effective_date"),
            rule_text: r.get("rule_text"), index_code: r.get("index_code"),
            index_period: r.get("index_period"), old_rent_cents: r.get("old_rent_cents"),
            index_old: r.get("index_old"), index_new: r.get("index_new"),
            cap_bp: r.get("cap_bp"), new_rent_cents: r.get("new_rent_cents"),
            formula: r.get("formula"), result_status: r.get("status"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    { Err(ServerFnError::new("list_lease_revisions est exécutée côté serveur")) }
}

// ============================================================
//  CHARGES / RÉDUCTIONS / DÉPÔTS / GARANTIES / DROIT D'ENTRÉE
// ============================================================

#[server]
pub async fn add_lease_charge(
    lease_id: Uuid, charge_type: String, mode: String, amount_cents: i64,
    variable_formula: String, effective_from: NaiveDate, effective_to: Option<NaiveDate>,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        if !valid_charge_mode(mode.trim()) || amount_cents < 0 {
            return Err(ServerFnError::new("Charge invalide"));
        }
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_charge_rules(legal_entity_id, lease_id, charge_type, mode,
                amount_cents, variable_formula, effective_from, effective_to)
             SELECT $1,$2,$3,$4,$5,$6,$7,$8
             WHERE EXISTS(SELECT 1 FROM leases WHERE id = $2 AND legal_entity_id = $1)
             RETURNING id",
        )
        .bind(current_legal_entity_id()).bind(lease_id).bind(charge_type.trim())
        .bind(mode.trim()).bind(amount_cents).bind(variable_formula.trim())
        .bind(effective_from).bind(effective_to)
        .fetch_optional(pool).await.map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Bail introuvable"))?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _ = (lease_id, charge_type, mode, amount_cents, variable_formula, effective_from, effective_to);
      Err(ServerFnError::new("add_lease_charge est exécutée côté serveur")) }
}

#[server]
pub async fn list_lease_charges(lease_id: Uuid) -> Result<Vec<LeaseChargeItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT id, lease_id, charge_type, mode, amount_cents, variable_formula,
                    effective_from, effective_to, active
             FROM lease_charge_rules WHERE lease_id = $1 AND legal_entity_id = $2
             ORDER BY active DESC, effective_from DESC",
        )
        .bind(lease_id).bind(current_legal_entity_id())
        .fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| LeaseChargeItem {
            id: r.get("id"), lease_id: r.get("lease_id"),
            charge_type: r.get("charge_type"), mode: r.get("mode"),
            amount_cents: r.get("amount_cents"), variable_formula: r.get("variable_formula"),
            effective_from: r.get("effective_from"), effective_to: r.get("effective_to"),
            active: r.get("active"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    { Err(ServerFnError::new("list_lease_charges est exécutée côté serveur")) }
}

#[server]
pub async fn add_lease_reduction(
    lease_id: Uuid, start_date: NaiveDate, end_date: NaiveDate,
    amount_cents: Option<i64>, percentage_bp: Option<i32>, reason: String,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        if end_date < start_date || reason.trim().is_empty() {
            return Err(ServerFnError::new("Réduction invalide"));
        }
        if amount_cents.is_none() && percentage_bp.is_none() {
            return Err(ServerFnError::new("Montant ou pourcentage requis"));
        }
        if percentage_bp.map(|v| v < 0 || v > 10000).unwrap_or(false) {
            return Err(ServerFnError::new("Pourcentage invalide"));
        }
        let old: i64 = sqlx::query_scalar(
            "SELECT COALESCE(NULLIF(current_rent_cents,0), rent_amount_cents)
             FROM leases WHERE id = $1 AND legal_entity_id = $2",
        ).bind(lease_id).bind(entity)
        .fetch_optional(pool).await.map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Bail introuvable"))?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_rent_reductions(legal_entity_id, lease_id, start_date,
                end_date, amount_cents, percentage_bp, reason, original_rent_cents)
             VALUES($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
        )
        .bind(entity).bind(lease_id).bind(start_date).bind(end_date)
        .bind(amount_cents).bind(percentage_bp).bind(reason.trim()).bind(old)
        .fetch_one(pool).await.map_err(ServerFnError::new)?;

        sqlx::query(
            "INSERT INTO lease_contract_events(legal_entity_id, lease_id, event_type,
                effective_date, before_payload, after_payload, reason)
             VALUES($1,$2,'TEMPORARY_RENT_REDUCTION',$3,$4,$5,$6)",
        )
        .bind(entity).bind(lease_id).bind(start_date)
        .bind(json!({ "rent_amount_cents": old }))
        .bind(json!({
            "reduction_id": id, "start_date": start_date, "end_date": end_date,
            "amount_cents": amount_cents, "percentage_bp": percentage_bp
        }))
        .bind(reason.trim())
        .execute(pool).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _ = (lease_id, start_date, end_date, amount_cents, percentage_bp, reason);
      Err(ServerFnError::new("add_lease_reduction est exécutée côté serveur")) }
}

#[server]
pub async fn list_lease_reductions(lease_id: Uuid) -> Result<Vec<LeaseReductionItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT id, lease_id, start_date, end_date, amount_cents, percentage_bp,
                    reason, original_rent_cents
             FROM lease_rent_reductions
             WHERE lease_id = $1 AND legal_entity_id = $2
             ORDER BY start_date DESC",
        )
        .bind(lease_id).bind(current_legal_entity_id())
        .fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| LeaseReductionItem {
            id: r.get("id"), lease_id: r.get("lease_id"),
            start_date: r.get("start_date"), end_date: r.get("end_date"),
            amount_cents: r.get("amount_cents"), percentage_bp: r.get("percentage_bp"),
            reason: r.get("reason"), original_rent_cents: r.get("original_rent_cents"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    { Err(ServerFnError::new("list_lease_reductions est exécutée côté serveur")) }
}

#[server]
pub async fn add_lease_deposit(
    lease_id: Uuid, movement_type: String, amount_cents: i64,
    movement_date: NaiveDate, justification: String,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        if amount_cents < 0 || !matches!(movement_type.trim(),
            "EXPECTED" | "RECEIVED" | "RESTITUTION" | "RETAINED" | "ADJUSTMENT") {
            return Err(ServerFnError::new("Mouvement de dépôt invalide"));
        }
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_deposits(legal_entity_id, lease_id, movement_type,
                amount_cents, movement_date, justification)
             SELECT $1,$2,$3,$4,$5,$6
             WHERE EXISTS(SELECT 1 FROM leases WHERE id = $2 AND legal_entity_id = $1)
             RETURNING id",
        )
        .bind(current_legal_entity_id()).bind(lease_id).bind(movement_type.trim())
        .bind(amount_cents).bind(movement_date).bind(justification.trim())
        .fetch_optional(pool).await.map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Bail introuvable"))?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _ = (lease_id, movement_type, amount_cents, movement_date, justification);
      Err(ServerFnError::new("add_lease_deposit est exécutée côté serveur")) }
}

#[server]
pub async fn list_lease_deposits(lease_id: Uuid) -> Result<Vec<LeaseDepositItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT id, lease_id, movement_type, amount_cents, movement_date,
                    justification, bank_transaction_id
             FROM lease_deposits WHERE lease_id = $1 AND legal_entity_id = $2
             ORDER BY movement_date DESC, created_at DESC",
        )
        .bind(lease_id).bind(current_legal_entity_id())
        .fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| LeaseDepositItem {
            id: r.get("id"), lease_id: r.get("lease_id"),
            movement_type: r.get("movement_type"), amount_cents: r.get("amount_cents"),
            movement_date: r.get("movement_date"), justification: r.get("justification"),
            bank_transaction_id: r.get("bank_transaction_id"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    { Err(ServerFnError::new("list_lease_deposits est exécutée côté serveur")) }
}

#[server]
pub async fn add_lease_guarantee(
    lease_id: Uuid, guarantee_type: String, guarantor_name: String,
    amount_cents: Option<i64>, start_date: Option<NaiveDate>, end_date: Option<NaiveDate>,
    document_id: Option<Uuid>, notes: String,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        if guarantor_name.trim().is_empty() {
            return Err(ServerFnError::new("Garant requis"));
        }
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_guarantees(legal_entity_id, lease_id, guarantee_type,
                guarantor_name, amount_cents, start_date, end_date, document_id, notes)
             SELECT $1,$2,$3,$4,$5,$6,$7,$8,$9
             WHERE EXISTS(SELECT 1 FROM leases WHERE id = $2 AND legal_entity_id = $1)
             RETURNING id",
        )
        .bind(current_legal_entity_id()).bind(lease_id).bind(guarantee_type.trim())
        .bind(guarantor_name.trim()).bind(amount_cents).bind(start_date)
        .bind(end_date).bind(document_id).bind(notes.trim())
        .fetch_optional(pool).await.map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Bail introuvable"))?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    { let _ = (lease_id, guarantee_type, guarantor_name, amount_cents,
        start_date, end_date, document_id, notes);
      Err(ServerFnError::new("add_lease_guarantee est exécutée côté serveur")) }
}

#[server]
pub async fn list_lease_guarantees(lease_id: Uuid) -> Result<Vec<LeaseGuaranteeItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT id, lease_id, guarantee_type, guarantor_name, amount_cents,
                    start_date, end_date, document_id, notes, active
             FROM lease_guarantees WHERE lease_id = $1 AND legal_entity_id = $2
             ORDER BY active DESC, created_at DESC",
        )
        .bind(lease_id).bind(current_legal_entity_id())
        .fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| LeaseGuaranteeItem {
            id: r.get("id"), lease_id: r.get("lease_id"),
            guarantee_type: r.get("guarantee_type"), guarantor_name: r.get("guarantor_name"),
            amount_cents: r.get("amount_cents"), start_date: r.get("start_date"),
            end_date: r.get("end_date"), document_id: r.get("document_id"),
            notes: r.get("notes"), active: r.get("active"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    { Err(ServerFnError::new("list_lease_guarantees est exécutée côté serveur")) }
}

#[server]
pub async fn set_lease_entry_fee(
    lease_id: Uuid, expected_amount_cents: i64,
    qualification_status: String, justification: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        if expected_amount_cents < 0
            || !matches!(qualification_status.trim(),
                "CONFIRMED" | "PENDING" | "UNCERTAIN" | "NOT_APPLICABLE") {
            return Err(ServerFnError::new("Qualification du droit d'entrée invalide"));
        }
        let amount = if expected_amount_cents == 0 {
            sqlx::query_scalar::<_, i64>(
                "SELECT entry_fee_expected_cents FROM leases
                 WHERE id = $1 AND legal_entity_id = $2",
            ).bind(lease_id).bind(entity)
            .fetch_optional(pool).await.map_err(ServerFnError::new)?.unwrap_or(0)
        } else { expected_amount_cents };

        sqlx::query(
            "INSERT INTO lease_entry_fees(legal_entity_id, lease_id, expected_amount_cents,
                qualification_status, justification, alert_required)
             VALUES($1,$2,$3,$4,$5,$4='UNCERTAIN')
             ON CONFLICT(legal_entity_id,lease_id)
             DO UPDATE SET expected_amount_cents = EXCLUDED.expected_amount_cents,
                qualification_status = EXCLUDED.qualification_status,
                justification = EXCLUDED.justification,
                alert_required = EXCLUDED.alert_required, updated_at = now()",
        )
        .bind(entity).bind(lease_id).bind(amount)
        .bind(qualification_status.trim()).bind(justification.trim())
        .execute(pool).await.map_err(ServerFnError::new)?;

        sqlx::query(
            "UPDATE leases SET entry_fee_expected_cents = $3, entry_fee_status = $4,
                updated_at = now() WHERE id = $1 AND legal_entity_id = $2",
        ).bind(lease_id).bind(entity).bind(amount).bind(qualification_status.trim())
        .execute(pool).await.map_err(ServerFnError::new)?;

        sqlx::query(
            "INSERT INTO lease_contract_events(legal_entity_id, lease_id, event_type,
                effective_date, after_payload, reason)
             VALUES($1,$2,'ENTRY_FEE_QUALIFICATION',CURRENT_DATE,$3,$4)",
        )
        .bind(entity).bind(lease_id)
        .bind(json!({ "expected_amount_cents": amount, "qualification_status": qualification_status }))
        .bind(justification.trim())
        .execute(pool).await.map_err(ServerFnError::new)?;

        if qualification_status.trim() != "UNCERTAIN" {
            sqlx::query(
                "UPDATE tasks SET state = 'DONE', completed_at = now()
                 WHERE legal_entity_id = $1 AND code = 'LEASE_ENTRY_FEE_CHECK'
                   AND entity_id = $2 AND state <> 'DONE'",
            ).bind(entity).bind(lease_id)
            .execute(pool).await.map_err(ServerFnError::new)?;
        }
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    { let _ = (lease_id, expected_amount_cents, qualification_status, justification);
      Err(ServerFnError::new("set_lease_entry_fee est exécutée côté serveur")) }
}

// ============================================================
//  UI — MODALE RÉVISION
// ============================================================

#[component]
fn RevisionModal(
    lease_id: Uuid, clause_code: String, index_period: String,
    effective_date: NaiveDate, on_close: EventHandler<bool>,
) -> Element {
    let cc_for_effect = clause_code.clone();
    let ip_for_effect = index_period.clone();

    let mut preview = use_signal(|| None::<LeaseRentRevisionItem>);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut override_rent = use_signal(String::new);
    let mut note = use_signal(String::new);
    let mut with_catching = use_signal(|| true);
    let mut submitting = use_signal(|| false);

    use_effect(move || {
        let cc = cc_for_effect.clone();
        let ip = ip_for_effect.clone();
        spawn(async move {
            match preview_lease_rent_revision(lease_id, cc, ip, effective_date).await {
                Ok(p) => {
                    override_rent.set(format!("{:.2}", p.new_rent_cents as f64 / 100.0));
                    preview.set(Some(p));
                }
                Err(e) => error.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    });

    let cc_refuse = clause_code.clone();
    let ip_refuse = index_period.clone();
    let cc_validate = clause_code.clone();
    let ip_validate = index_period.clone();

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0,0,0,0.75); display: flex; align-items: center; justify-content: center; z-index: 2147483647;",
            onclick: move |_| on_close.call(false),
            div {
                onclick: move |e| e.stop_propagation(),
                style: "background: #1e293b; border-radius: 12px; padding: 24px; max-width: 720px; width: 90%; max-height: 85vh; overflow-y: auto; box-shadow: 0 10px 40px rgba(0,0,0,0.6);",
                div { style: "height: 4px; background: #38bdf8; margin: -24px -24px 20px -24px; border-radius: 12px 12px 0 0;" }
                div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                    div {
                        div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;", "Révision de loyer" }
                        h3 { style: "margin: 4px 0 0 0; color: #f8fafc;", "Aperçu avant validation" }
                    }
                    button {
                        style: "background: transparent; border: none; color: #94a3b8; font-size: 1.5rem; cursor: pointer; line-height: 1;",
                        onclick: move |_| on_close.call(false), "×"
                    }
                }
                if loading() {
                    div { style: "padding: 20px; text-align: center; color: #94a3b8;", "Calcul..." }
                } else if let Some(err) = error() {
                    div { style: "padding: 14px; background: rgba(248,113,113,0.10); border: 1px solid rgba(248,113,113,0.3); border-radius: 6px; color: #f87171; font-size: 0.8rem;", "{err}" }
                } else if let Some(p) = preview() {
                    div { style: "display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin-bottom: 16px;",
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase;", "Loyer actuel" }
                            div { style: "font-size: 1rem; font-weight: 600; color: #e2e8f0; margin-top: 4px;",
                                {format!("{:.2} EUR", p.old_rent_cents as f64 / 100.0)} }
                        }
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase;", "Indice base" }
                            div { style: "font-size: 1rem; font-weight: 600; color: #94a3b8; margin-top: 4px;",
                                {p.index_old.map(|i| format!("{:.4}", i)).unwrap_or_else(|| "—".into())} }
                        }
                        div { style: "padding: 12px; background: #0f172a; border-radius: 8px;",
                            div { style: "font-size: 0.65rem; color: #64748b; text-transform: uppercase;", "Nouvel indice" }
                            div { style: "font-size: 1rem; font-weight: 600; color: #94a3b8; margin-top: 4px;",
                                {p.index_new.map(|i| format!("{:.4}", i)).unwrap_or_else(|| "—".into())} }
                        }
                    }
                    div { style: "padding: 12px 14px; background: rgba(56,189,248,0.06); border: 1px solid rgba(56,189,248,0.25); border-radius: 8px; margin-bottom: 16px;",
                        div { style: "font-size: 0.7rem; color: #38bdf8; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600; margin-bottom: 6px;", "Calcul" }
                        div { style: "font-size: 0.78rem; color: #cbd5e1; font-family: monospace;", "{p.formula}" }
                        div { style: "font-size: 1.1rem; font-weight: 700; color: #38bdf8; margin-top: 8px;",
                            {format!("Nouveau loyer calculé : {:.2} EUR", p.new_rent_cents as f64 / 100.0)} }
                    }
                    label { class: "field",
                        span { "Loyer à appliquer (modifiable) €" }
                        input { r#type: "text", value: "{override_rent}",
                            oninput: move |e| override_rent.set(e.value()) }
                    }
                    label { class: "field", style: "margin-top: 12px;",
                        span { "Note (optionnel)" }
                        input { r#type: "text", value: "{note}",
                            oninput: move |e| note.set(e.value()),
                            placeholder: "Ex : accord amiable, rattrapage négocié..." }
                    }
                    label { style: "display: flex; align-items: center; gap: 8px; font-size: 0.8rem; color: #e2e8f0; margin-top: 12px;",
                        input { r#type: "checkbox", checked: with_catching(),
                            onchange: move |e| with_catching.set(e.value() == "true") }
                        "Créer une facture de rattrapage (brouillon)"
                    }
                    div { style: "display: flex; gap: 8px; margin-top: 20px; justify-content: flex-end;",
                        button { class: "secondary", disabled: submitting(),
                            onclick: move |_| on_close.call(false), "Annuler" }
                        button {
                            style: "padding: 8px 16px; background: transparent; color: #f87171; border: 1px solid #f87171; border-radius: 6px; cursor: pointer; font-weight: 600; font-size: 0.82rem;",
                            disabled: submitting(),
                            onclick: {
                                let cc = cc_refuse.clone(); let ip = ip_refuse.clone();
                                move |_| {
                                    let cc = cc.clone(); let ip = ip.clone();
                                    submitting.set(true);
                                    spawn(async move {
                                        let _ = refuse_lease_rent_revision(lease_id, cc, ip, effective_date, note()).await;
                                        submitting.set(false); on_close.call(true);
                                    });
                                }
                            }, "Refuser"
                        }
                        button {
                            class: "primary", disabled: submitting(),
                            onclick: {
                                let cc = cc_validate.clone(); let ip = ip_validate.clone();
                                move |_| {
                                    let cc = cc.clone(); let ip = ip.clone();
                                    let ovr = override_rent().replace(',', ".").parse::<f64>().ok()
                                        .map(|v| (v * 100.0).round() as i64);
                                    let p_opt = preview();
                                    let is_override = p_opt
                                        .map(|p| ovr.map(|v| v != p.new_rent_cents).unwrap_or(false))
                                        .unwrap_or(false);
                                    submitting.set(true);
                                    spawn(async move {
                                        let _ = apply_lease_rent_revision(
                                            lease_id, cc, ip, effective_date,
                                            if is_override { ovr } else { None },
                                            note(), with_catching()).await;
                                        submitting.set(false); on_close.call(true);
                                    });
                                }
                            },
                            if submitting() { "Enregistrement..." } else { "Valider" }
                        }
                    }
                }
            }
        }
    }
}

// ============================================================
//  UI — PAGE BAUX
// ============================================================

#[component]
pub fn LeasesPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);

    let leases = use_resource(move || {
        let _ = refresh(); let _ = bump();
        async move { list_lease_details().await.unwrap_or_default() }
    });
    let units = use_resource(move || {
        let _ = refresh();
        async move { crate::server::list_units().await.unwrap_or_default() }
    });
    let tenants = use_resource(move || {
        let _ = refresh();
        async move { crate::server::list_tenants().await.unwrap_or_default() }
    });

    let mut selected = use_signal(|| None::<Uuid>);
    let mut reference = use_signal(String::new);
    let mut effect = use_signal(|| chrono::Utc::now().date_naive().to_string());
    let mut end = use_signal(String::new);
    let mut signature = use_signal(String::new);
    let mut rent = use_signal(String::new);
    let mut destination = use_signal(|| "Commerce".to_owned());
    let mut lease_type = use_signal(|| "BAIL_COMMERCIAL".to_owned());
    let mut rent_frequency = use_signal(|| "MONTHLY".to_owned());
    let mut vat_mode = use_signal(|| "FROM_UNIT".to_owned());
    let mut index_code = use_signal(|| "ICC".to_owned());
    let mut index_base = use_signal(String::new);
    let mut index_base_date = use_signal(String::new);
    let mut index_base_period = use_signal(String::new);
    let mut index_cap = use_signal(String::new);
    let mut entry_fee = use_signal(String::new);
    let mut security_deposit = use_signal(String::new);
    let mut revision_period = use_signal(|| "12".to_owned());
    let mut msg = use_signal(String::new);

    let mut unit = use_signal(String::new);
    let mut tenant = use_signal(String::new);

    let clauses = use_resource(move || {
        let id = selected();
        async move { match id { Some(i) => list_lease_clauses(i).await.unwrap_or_default(), None => Vec::new() } }
    });
    let revisions = use_resource(move || {
        let id = selected();
        async move { match id { Some(i) => list_lease_revisions(i).await.unwrap_or_default(), None => Vec::new() } }
    });
    let charges = use_resource(move || {
        let id = selected();
        async move { match id { Some(i) => list_lease_charges(i).await.unwrap_or_default(), None => Vec::new() } }
    });
    let reductions = use_resource(move || {
        let id = selected();
        async move { match id { Some(i) => list_lease_reductions(i).await.unwrap_or_default(), None => Vec::new() } }
    });
    let deposits = use_resource(move || {
        let id = selected();
        async move { match id { Some(i) => list_lease_deposits(i).await.unwrap_or_default(), None => Vec::new() } }
    });
    let guarantees = use_resource(move || {
        let id = selected();
        async move { match id { Some(i) => list_lease_guarantees(i).await.unwrap_or_default(), None => Vec::new() } }
    });
    let indices = use_resource(move || {
        async move { list_lease_indices("ICC".into()).await.unwrap_or_default() }
    });

    let mut clause_code = use_signal(|| "INDEXATION".to_owned());
    let mut clause_body = use_signal(String::new);
    let mut index_period = use_signal(String::new);
    let mut revision_date = use_signal(|| chrono::Utc::now().date_naive().to_string());

    let mut charge_type = use_signal(|| "COPROPRIETE".to_owned());
    let mut charge_mode = use_signal(|| "PROVISION".to_owned());
    let mut charge_amount = use_signal(String::new);
    let mut charge_formula = use_signal(String::new);

    let mut reduction_start = use_signal(|| chrono::Utc::now().date_naive().to_string());
    let mut reduction_end = use_signal(|| {
        (chrono::Utc::now().date_naive() + Duration::days(30)).to_string()
    });
    let mut reduction_amount = use_signal(String::new);
    let mut reduction_pct = use_signal(String::new);
    let mut reduction_reason = use_signal(String::new);

    let mut deposit_type = use_signal(|| "RECEIVED".to_owned());
    let mut deposit_amount = use_signal(String::new);
    let mut deposit_date = use_signal(|| chrono::Utc::now().date_naive().to_string());
    let mut deposit_justification = use_signal(String::new);

    let mut guarantee_type = use_signal(|| "CAUTION".to_owned());
    let mut guarantor = use_signal(String::new);
    let mut guarantee_amount = use_signal(String::new);
    let mut guarantee_notes = use_signal(String::new);

    let mut entry_status = use_signal(|| "UNCERTAIN".to_owned());
    let mut entry_justification = use_signal(String::new);

    let mut show_revision_modal = use_signal(|| false);

    let selected_lease = selected().and_then(|id| {
        leases.read().as_deref().unwrap_or(&[]).iter().find(|l| l.id == id).cloned()
    });

    let revision_lookup: std::collections::HashMap<String, i64> = revisions.read()
        .as_deref().unwrap_or(&[])
        .iter()
        .filter(|r| matches!(r.result_status.as_str(), "VALIDATED" | "ADJUSTED"))
        .map(|r| (r.index_period.clone(), r.new_rent_cents))
        .collect();

    let all_icc: Vec<LeaseIndexItem> = indices.read().as_deref().unwrap_or(&[])
        .iter()
        .filter(|i| i.index_code == "ICC")
        .filter(|i| period_to_abs(&i.period_label).is_some())
        .cloned().collect();

    let start_abs = selected_lease.as_ref()
        .map(|l| date_to_period_abs(l.effect_date)).unwrap_or(0);

    let relevant_indices: Vec<LeaseIndexItem> = all_icc.iter()
        .filter(|i| period_to_abs(&i.period_label).map(|a| a >= start_abs - 8).unwrap_or(false))
        .cloned().collect();

    let rev_months: u32 = selected_lease.as_ref()
        .map(|l| l.revision_period_months.max(12) as u32).unwrap_or(12);
    let pub_day: u32 = selected_lease.as_ref()
        .map(|l| l.index_publication_day.max(1) as u32).unwrap_or(12);
    let pub_offset: u32 = selected_lease.as_ref()
        .map(|l| l.index_publication_month_offset.max(1) as u32).unwrap_or(3);

    let today = chrono::Utc::now().date_naive();
    let current_anniversary: Option<NaiveDate> = selected_lease.as_ref()
        .map(|l| most_recent_anniversary(l.effect_date, today, rev_months));
    let next_anniv: Option<NaiveDate> = selected_lease.as_ref()
        .map(|l| next_anniversary(l.effect_date, today, rev_months));
    let reference_period_start: Option<NaiveDate> = current_anniversary
        .map(|a| a - Duration::days(365 * (rev_months as i64) / 12));

    let _applicable_period: Option<String> = current_anniversary
        .and_then(|a| expected_period_for_date(a, pub_day, pub_offset));

    let next_expected_period: Option<String> = next_anniv
        .and_then(|a| expected_period_for_date(a, pub_day, pub_offset));

    let next_expected_in_db: bool = next_expected_period.as_deref()
        .map(|p| all_icc.iter().any(|i| i.period_label == p))
        .unwrap_or(false);

    let modal_id = selected();
    let modal_show = show_revision_modal();
    let modal_cc = clause_code();
    let modal_ip = index_period();
    let modal_date = NaiveDate::parse_from_str(&revision_date(), "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Utc::now().date_naive());

    rsx! {
        if modal_show {
            if let Some(mid) = modal_id {
                RevisionModal {
                    lease_id: mid,
                    clause_code: modal_cc.clone(),
                    index_period: modal_ip.clone(),
                    effective_date: modal_date,
                    on_close: move |refresh_needed: bool| {
                        show_revision_modal.set(false);
                        if refresh_needed { bump += 1; }
                    },
                }
            }
        }

        ModuleHeader {
            title: "Baux & locations",
            kicker: "CONTRATS • INDEXATION • CHARGES • DÉPÔTS • GARANTIES",
            detail: "Le bail structuré conserve l'historique et traite les modifications comme des événements contractuels."
        }

        section { class: "panel",
            h2 { "Créer un bail structuré" }
            div { class: "form-grid",
                label { class: "field",
                    span { "Lot" }
                    select { value: unit(), onchange: move |e: FormEvent| unit.set(e.value()),
                        option { value: "", "Sélectionner" }
                        for u in units.read().as_deref().unwrap_or(&[]).iter() {
                            option { value: u.id.to_string(), {format!("{} • {}", u.property_name, u.label)} }
                        }
                    }
                }
                label { class: "field",
                    span { "Locataire" }
                    select { value: tenant(), onchange: move |e: FormEvent| tenant.set(e.value()),
                        option { value: "", "Sélectionner" }
                        for t in tenants.read().as_deref().unwrap_or(&[]).iter() {
                            option { value: t.id.to_string(), "{t.legal_name}" }
                        }
                    }
                }
                FormField { label: "Référence", value: reference(), oninput: move |e: FormEvent| reference.set(e.value()) }
                FormField { label: "Signature AAAA-MM-JJ", value: signature(), oninput: move |e: FormEvent| signature.set(e.value()) }
                FormField { label: "Effet AAAA-MM-JJ", value: effect(), oninput: move |e: FormEvent| effect.set(e.value()) }
                FormField { label: "Échéance AAAA-MM-JJ", value: end(), oninput: move |e: FormEvent| end.set(e.value()) }

                label { class: "field",
                    span { "Type de bail" }
                    select { value: lease_type(), onchange: move |e: FormEvent| lease_type.set(e.value()),
                        option { value: "BAIL_COMMERCIAL", "Bail commercial" }
                        option { value: "BAIL_PROFESSIONNEL", "Bail professionnel" }
                        option { value: "BAIL_HABITATION", "Bail d'habitation" }
                        option { value: "BAIL_MIXTE", "Bail mixte" }
                        option { value: "BAIL_MEUBLE", "Bail meublé" }
                    }
                }

                label { class: "field",
                    span { "Usage du local" }
                    select { value: destination(), onchange: move |e: FormEvent| destination.set(e.value()),
                        option { value: "Commerce", "Commerce" }
                        option { value: "Bureau", "Bureau" }
                        option { value: "Habitation", "Habitation" }
                        option { value: "Entrepôt", "Entrepôt" }
                        option { value: "Local d'activité", "Local d'activité" }
                        option { value: "Industriel", "Industriel" }
                        option { value: "Mixte", "Mixte (habitation + activité)" }
                        option { value: "Parking", "Parking / Garage" }
                    }
                }

                FormField { label: "Loyer €", value: rent(), oninput: move |e: FormEvent| rent.set(e.value()) }

                label { class: "field",
                    span { "Périodicité du loyer" }
                    select { value: rent_frequency(), onchange: move |e: FormEvent| rent_frequency.set(e.value()),
                        option { value: "MONTHLY", "Mensuel" }
                        option { value: "QUARTERLY", "Trimestriel" }
                        option { value: "ANNUAL", "Annuel" }
                    }
                }

                label { class: "field",
                    span { "TVA" }
                    select { value: vat_mode(), onchange: move |e: FormEvent| vat_mode.set(e.value()),
                        option { value: "FROM_UNIT", "Taux du lot" }
                        option { value: "EXONERATED", "Exonéré de TVA" }
                        option { value: "OPTION", "Option TVA (assujetti)" }
                        option { value: "CUSTOM", "Personnalisé" }
                    }
                }

                label { class: "field",
                    span { "Indice de référence" }
                    select { value: index_code(), onchange: move |e: FormEvent| index_code.set(e.value()),
                        option { value: "ICC", "ICC — Indice du coût de la construction" }
                        option { value: "ILC", "ILC — Indice des loyers commerciaux" }
                        option { value: "ILAT", "ILAT — Indice des loyers des activités tertiaires" }
                    }
                }

                label { class: "field",
                    span { "Indice de base" }
                    select {
                        value: index_base_period(),
                        onchange: move |e: FormEvent| {
                            let period = e.value();
                            index_base_period.set(period.clone());
                            if period.is_empty() {
                                index_base.set(String::new());
                                index_base_date.set(String::new());
                            } else {
                                let picked = indices.read().as_deref().unwrap_or(&[])
                                    .iter()
                                    .find(|i| i.period_label == period)
                                    .cloned();
                                if let Some(idx) = picked {
                                    index_base.set(format!("{:.4}", idx.value));
                                    if let Some(pd) = publication_date(&period, 12, 3) {
                                        index_base_date.set(pd.to_string());
                                    }
                                }
                            }
                        },
                        option { value: "", "— Sélectionner l'indice de base —" }
                        for idx in indices.read().as_deref().unwrap_or(&[]).iter() {
                            option { value: "{idx.period_label}",
                                {format!("{} — {:.4}", idx.period_label, idx.value)} }
                        }
                    }
                }

                label { class: "field",
                    span { "Date de référence (calculée)" }
                    input {
                        r#type: "text",
                        readonly: true,
                        value: "{index_base_date}",
                        placeholder: "Automatique",
                        style: "opacity: 0.7; cursor: not-allowed;",
                    }
                }

                FormField { label: "Plafond % (optionnel)", value: index_cap(), oninput: move |e: FormEvent| index_cap.set(e.value()) }

                label { class: "field",
                    span { "Périodicité de révision" }
                    select { value: revision_period(), onchange: move |e: FormEvent| revision_period.set(e.value()),
                        option { value: "12", "Annuelle (12 mois)" }
                        option { value: "24", "Biennal(e) (24 mois)" }
                        option { value: "36", "Triennale (36 mois)" }
                    }
                }

                FormField { label: "Dépôt prévu €", value: security_deposit(), oninput: move |e: FormEvent| security_deposit.set(e.value()) }
                FormField { label: "Droit d'entrée €", value: entry_fee(), oninput: move |e: FormEvent| entry_fee.set(e.value()) }
            }
            button {
                class: "primary",
                onclick: move |_| {
                    let u = Uuid::parse_str(&unit());
                    let t = Uuid::parse_str(&tenant());
                    let e = NaiveDate::parse_from_str(&effect(), "%Y-%m-%d");
                    let rp = revision_period().parse::<i32>().unwrap_or(12);
                    async move {
                        match (u, t, e) {
                            (Ok(u), Ok(t), Ok(eff)) => {
                                match (parse_optional_date(&signature()), parse_optional_date(&end())) {
                                    (Ok(sig), Ok(en)) => {
                                        let r = (rent().replace(',', ".").parse::<f64>().unwrap_or(0.0) * 100.0).round() as i64;
                                        let b = index_base().replace(',', ".").parse::<Decimal>().ok();
                                        let cap = index_cap().trim().parse::<i32>().ok();
                                        let deposit = (security_deposit().replace(',', ".").parse::<f64>().unwrap_or(0.0) * 100.0).round() as i64;
                                        let entry = (entry_fee().replace(',', ".").parse::<f64>().unwrap_or(0.0) * 100.0).round() as i64;
                                        let base_date = parse_optional_date(&index_base_date()).ok().flatten();
                                        match create_structured_lease(
                                            u, t, reference(), sig, eff, en, lease_type(), destination(),
                                            r, rent_frequency(), 5, vat_mode(), index_code(), b, base_date,
                                            cap, "NONE".into(), 0, deposit, entry, rp,
                                        ).await {
                                            Ok(id) => { selected.set(Some(id)); msg.set("Bail structuré créé".into()); bump += 1; }
                                            Err(e) => { msg.set(e.to_string()); }
                                        }
                                    }
                                    _ => { msg.set("Date invalide".into()); }
                                }
                            }
                            _ => { msg.set("Lot, locataire ou effet invalide".into()); }
                        }
                    }
                },
                "Créer"
            }
            span { class: "save-ok", "{msg}" }
        }

        for l in leases.read().as_deref().unwrap_or(&[]).iter().cloned() {
            div { class: "data-row",
                div {
                    div { class: "data-title", "{l.reference} • {l.tenant_name}" }
                    div { class: "small", "{l.property_name} / {l.unit_label} • loyer actuel {l.rent_amount_cents as f64 / 100.0} • {l.entry_fee_status} • révision {l.revision_period_months} mois" }
                }
                div { class: "row-actions",
                    button { class: "secondary", onclick: move |_| selected.set(Some(l.id)), "Détails" }
                }
            }
        }

        if let Some(id) = selected() {
            section { class: "panel",
                h2 { "Clause exploitable" }
                div { class: "form-grid",
                    FormField { label: "Code", value: clause_code(), oninput: move |e: FormEvent| clause_code.set(e.value()) }
                    FormField { label: "Texte", value: clause_body(), oninput: move |e: FormEvent| clause_body.set(e.value()) }
                }
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let c = clause_code(); let b = clause_body();
                        async move {
                            match save_lease_clause(id, c, "Clause contractuelle".into(), "GENERAL".into(), b, chrono::Utc::now().date_naive(), None).await {
                                Ok(_) => bump += 1,
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    },
                    "Ajouter une version de clause"
                }
                div {
                    for c in clauses.read().as_deref().unwrap_or(&[]).iter() {
                        div { class: "data-row",
                            div { {format!("{} v{}", c.code, c.version_no)} }
                            div { class: "small", "{c.body}" }
                        }
                    }
                }
            }

            section { class: "panel",
                h2 { "Révision de loyer" }
                div { class: "form-grid",
                    FormField { label: "Code clause", value: clause_code(), oninput: move |e: FormEvent| clause_code.set(e.value()) }
                    FormField { label: "Période indice", value: index_period(), oninput: move |e: FormEvent| index_period.set(e.value()) }
                    FormField { label: "Date d'effet", value: revision_date(), oninput: move |e: FormEvent| revision_date.set(e.value()) }
                }
                button { class: "primary",
                    onclick: move |_| show_revision_modal.set(true),
                    "Calculer la révision"
                }
                div {
                    for r in revisions.read().as_deref().unwrap_or(&[]).iter() {
                        {
                            let status_color = match r.result_status.as_str() {
                                "VALIDATED" => "#22c55e",
                                "ADJUSTED" => "#38bdf8",
                                "REFUSED" => "#f87171",
                                "POSTPONED" => "#fbbf24",
                                "PREVIEW" | "CALCULATED" => "#94a3b8",
                                _ => "#94a3b8",
                            };
                            rsx! {
                                div { class: "data-row",
                                    div {
                                        div { {format!("{} • {} → {:.2} EUR", r.index_period, r.result_status, r.new_rent_cents as f64 / 100.0)} }
                                        div { class: "small", "{r.formula}" }
                                    }
                                    div { style: "color: {status_color}; font-size: 0.7rem; font-weight: 600;", "{r.result_status}" }
                                }
                            }
                        }
                    }
                }
            }

            section { class: "panel",
                h2 { "Indices ICC disponibles" }

                if let Some(anniv) = current_anniversary {
                    div {
                        style: "display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; margin-bottom: 12px;",
                        div {
                            style: "padding: 8px 12px; background: rgba(56,189,248,0.06); border: 1px solid rgba(56,189,248,0.2); border-radius: 6px;",
                            div { style: "font-size: 0.62rem; color: #38bdf8; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;", "Dernier anniversaire" }
                            div { style: "font-size: 0.85rem; color: #e2e8f0; font-weight: 600; margin-top: 2px;", "{anniv}" }
                        }
                        if let Some(ref_start) = reference_period_start {
                            div {
                                style: "padding: 8px 12px; background: rgba(148,163,184,0.06); border: 1px solid rgba(148,163,184,0.2); border-radius: 6px;",
                                div { style: "font-size: 0.62rem; color: #94a3b8; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;", "Période de référence ({rev_months} mois)" }
                                div { style: "font-size: 0.85rem; color: #e2e8f0; font-weight: 600; margin-top: 2px;", "{ref_start} → {anniv}" }
                            }
                        }
                    }
                }

                if let Some(next_p) = next_expected_period.clone() {
                    {
                        let np = next_p.clone();
                        let in_db = next_expected_in_db;
                        let is_done = revision_lookup.contains_key(&np);
                        let (banner_color, banner_bg, banner_border, banner_label) = if is_done {
                            ("#22c55e", "rgba(34,197,94,0.10)", "rgba(34,197,94,0.4)",
                             "Révision déjà appliquée".to_string())
                        } else if in_db {
                            ("#fbbf24", "rgba(251,191,36,0.10)", "rgba(251,191,36,0.4)",
                             match next_anniv {
                                 Some(a) => format!("Prochaine révision à appliquer ({})", a),
                                 None => "Prochaine révision à appliquer".to_string(),
                             })
                        } else {
                            ("#94a3b8", "rgba(148,163,184,0.08)", "rgba(148,163,184,0.3)",
                             match next_anniv {
                                 Some(a) => format!("Prochaine révision ({}) — indice non encore publié", a),
                                 None => "Prochaine révision — indice non encore publié".to_string(),
                             })
                        };
                        rsx! {
                            div {
                                style: "padding: 10px 14px; background: {banner_bg}; border: 1px solid {banner_border}; border-radius: 6px; margin-bottom: 12px; display: flex; align-items: center; justify-content: space-between;",
                                div {
                                    div { style: "font-size: 0.62rem; color: {banner_color}; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600;", "{banner_label}" }
                                    div { style: "font-size: 1rem; color: {banner_color}; font-weight: 700; margin-top: 2px;", "{next_p}" }
                                    if !in_db && !is_done {
                                        div { style: "font-size: 0.68rem; color: #94a3b8; margin-top: 4px;",
                                            "Importe l'indice depuis l'INSEE pour pouvoir appliquer la révision." }
                                    }
                                }
                                if in_db && !is_done {
                                    button {
                                        style: "background: {banner_bg}; border: 1px solid {banner_border}; color: {banner_color}; font-size: 0.72rem; font-weight: 600; padding: 5px 10px; border-radius: 4px; cursor: pointer;",
                                        onclick: move |_| index_period.set(np.clone()),
                                        "Sélectionner"
                                    }
                                }
                            }
                        }
                    }
                }

                div { style: "font-size: 0.72rem; color: #94a3b8; margin-bottom: 10px;",
                    "Clique sur un indice pour le sélectionner. Indices affichés : {relevant_indices.len()}"
                }

                div { style: "display: flex; flex-direction: column; gap: 6px; max-height: 400px; overflow-y: auto;",
                    for i in relevant_indices.iter() {
                        {
                            let period = i.period_label.clone();
                            let new_rent_opt: Option<i64> = revision_lookup.get(&period).copied();
                            let is_used = new_rent_opt.is_some();
                            let is_next = !is_used
                                && next_expected_period.as_deref().map(|p| p == period.as_str()).unwrap_or(false);
                            let period_abs = period_to_abs(&period).unwrap_or(0);
                            let before_lease = period_abs < start_abs;

                            let (color, bg, border, opacity): (&str, &str, &str, &str) = if is_used {
                                ("#22c55e", "rgba(34,197,94,0.10)", "rgba(34,197,94,0.45)", "1")
                            } else if is_next {
                                ("#fbbf24", "rgba(251,191,36,0.12)", "rgba(251,191,36,0.5)", "1")
                            } else if before_lease {
                                ("#64748b", "rgba(100,116,139,0.04)", "rgba(100,116,139,0.15)", "0.4")
                            } else {
                                ("#475569", "rgba(71,85,105,0.05)", "rgba(71,85,105,0.18)", "0.5")
                            };

                            let is_selected = index_period() == period;
                            let value = i.value;
                            let src = i.source_reference.clone();
                            let outline = if is_selected { "2px solid #38bdf8" } else { "none" };
                            rsx! {
                                div {
                                    key: "{i.id}",
                                    style: "display: flex; align-items: center; gap: 12px; padding: 8px 12px; background: {bg}; border: 1px solid {border}; border-left: 3px solid {color}; border-radius: 6px; cursor: pointer; outline: {outline}; opacity: {opacity};",
                                    onclick: move |_| index_period.set(period.clone()),
                                    span { style: "font-weight: 700; color: {color}; min-width: 70px;", "{i.period_label}" }
                                    span { style: "font-variant-numeric: tabular-nums; color: #e2e8f0; font-family: monospace; font-size: 0.8rem;",
                                        {format!("{:.4}", value)} }
                                    span { style: "font-size: 0.68rem; color: #64748b; flex: 1; text-align: right;", "{src}" }
                                    if let Some(new_rent) = new_rent_opt {
                                        span { style: "font-size: 0.75rem; color: #22c55e; font-weight: 700; font-family: monospace; white-space: nowrap;",
                                            {format!("→ {:.2} €", new_rent as f64 / 100.0)} }
                                    }
                                    if is_next {
                                        span { style: "font-size: 0.7rem; color: #fbbf24; font-weight: 600;", "Prochaine révision" }
                                    }
                                    if before_lease && !is_used {
                                        span { style: "font-size: 0.65rem; color: #64748b; font-style: italic;", "Avant bail" }
                                    }
                                    if is_selected && !is_used && !is_next {
                                        span { style: "font-size: 0.7rem; color: #38bdf8; font-weight: 600;", "Sélectionné" }
                                    }
                                }
                            }
                        }
                    }
                    if relevant_indices.is_empty() {
                        div { style: "padding: 14px; text-align: center; color: #64748b; font-size: 0.8rem;",
                            "Aucun indice disponible pour ce bail. Renseigne l'indice de base." }
                    }
                }

                div {
                    style: "display: flex; flex-wrap: wrap; gap: 12px; margin-top: 12px; padding-top: 10px; border-top: 1px solid rgba(148,163,184,0.1); font-size: 0.68rem; color: #94a3b8;",
                    div { style: "display: flex; align-items: center; gap: 5px;",
                        span { style: "display: inline-block; width: 10px; height: 10px; background: #22c55e; border-radius: 2px;" }
                        "Révision appliquée (nouveau loyer affiché)" }
                    div { style: "display: flex; align-items: center; gap: 5px;",
                        span { style: "display: inline-block; width: 10px; height: 10px; background: #fbbf24; border-radius: 2px;" }
                        "Prochaine révision" }
                    div { style: "display: flex; align-items: center; gap: 5px;",
                        span { style: "display: inline-block; width: 10px; height: 10px; background: #475569; border-radius: 2px;" }
                        "Hors périmètre" }
                    div { style: "display: flex; align-items: center; gap: 5px;",
                        span { style: "display: inline-block; width: 10px; height: 10px; background: #64748b; border-radius: 2px; opacity: 0.5;" }
                        "Antérieur au bail" }
                }
            }

            section { class: "panel",
                h2 { "Charges" }
                div { class: "form-grid",
                    FormField { label: "Type", value: charge_type(), oninput: move |e: FormEvent| charge_type.set(e.value()) }
                    FormField { label: "Mode", value: charge_mode(), oninput: move |e: FormEvent| charge_mode.set(e.value()) }
                    FormField { label: "Montant €", value: charge_amount(), oninput: move |e: FormEvent| charge_amount.set(e.value()) }
                    FormField { label: "Formule variable", value: charge_formula(), oninput: move |e: FormEvent| charge_formula.set(e.value()) }
                }
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let a = (charge_amount().replace(',', ".").parse::<f64>().unwrap_or(0.0) * 100.0).round() as i64;
                        let c = charge_type(); let m = charge_mode(); let f = charge_formula();
                        async move {
                            match add_lease_charge(id, c, m, a, f, chrono::Utc::now().date_naive(), None).await {
                                Ok(_) => bump += 1,
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    }, "Ajouter une charge"
                }
                div {
                    for c in charges.read().as_deref().unwrap_or(&[]).iter() {
                        div { class: "data-row",
                            div { {format!("{} / {} / {} €", c.charge_type, c.mode, c.amount_cents as f64 / 100.0)} }
                            div { class: "small", "{c.variable_formula}" }
                        }
                    }
                }
            }

            section { class: "panel",
                h2 { "Réductions temporaires" }
                div { class: "form-grid",
                    FormField { label: "Début", value: reduction_start(), oninput: move |e: FormEvent| reduction_start.set(e.value()) }
                    FormField { label: "Fin", value: reduction_end(), oninput: move |e: FormEvent| reduction_end.set(e.value()) }
                    FormField { label: "Montant €", value: reduction_amount(), oninput: move |e: FormEvent| reduction_amount.set(e.value()) }
                    FormField { label: "Pourcentage %", value: reduction_pct(), oninput: move |e: FormEvent| reduction_pct.set(e.value()) }
                    FormField { label: "Motif", value: reduction_reason(), oninput: move |e: FormEvent| reduction_reason.set(e.value()) }
                }
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let s = NaiveDate::parse_from_str(&reduction_start(), "%Y-%m-%d");
                        let e = NaiveDate::parse_from_str(&reduction_end(), "%Y-%m-%d");
                        async move {
                            if let (Ok(s), Ok(e)) = (s, e) {
                                let a = if reduction_amount().trim().is_empty() { None } else {
                                    Some((reduction_amount().replace(',', ".").parse::<f64>().unwrap_or(0.0) * 100.0).round() as i64)
                                };
                                let pct = if reduction_pct().trim().is_empty() { None } else {
                                    Some((reduction_pct().replace(',', ".").parse::<f64>().unwrap_or(0.0) * 100.0).round() as i32)
                                };
                                match add_lease_reduction(id, s, e, a, pct, reduction_reason()).await {
                                    Ok(_) => bump += 1,
                                    Err(e) => msg.set(e.to_string()),
                                }
                            } else { msg.set("Périodes invalides".into()); }
                        }
                    }, "Enregistrer la réduction"
                }
                div {
                    for r in reductions.read().as_deref().unwrap_or(&[]).iter() {
                        div { class: "data-row",
                            div { {format!("{} → {}", r.start_date, r.end_date)} }
                            div { class: "small", "{r.reason}" }
                        }
                    }
                }
            }

            section { class: "panel",
                h2 { "Dépôt de garantie" }
                div { class: "form-grid",
                    FormField { label: "Mouvement", value: deposit_type(), oninput: move |e: FormEvent| deposit_type.set(e.value()) }
                    FormField { label: "Montant €", value: deposit_amount(), oninput: move |e: FormEvent| deposit_amount.set(e.value()) }
                    FormField { label: "Date", value: deposit_date(), oninput: move |e: FormEvent| deposit_date.set(e.value()) }
                    FormField { label: "Justification", value: deposit_justification(), oninput: move |e: FormEvent| deposit_justification.set(e.value()) }
                }
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let d = NaiveDate::parse_from_str(&deposit_date(), "%Y-%m-%d");
                        async move {
                            if let Ok(d) = d {
                                let a = (deposit_amount().replace(',', ".").parse::<f64>().unwrap_or(0.0) * 100.0).round() as i64;
                                match add_lease_deposit(id, deposit_type(), a, d, deposit_justification()).await {
                                    Ok(_) => bump += 1,
                                    Err(e) => msg.set(e.to_string()),
                                }
                            } else { msg.set("Date invalide".into()); }
                        }
                    }, "Enregistrer le mouvement"
                }
                div {
                    for d in deposits.read().as_deref().unwrap_or(&[]).iter() {
                        div { class: "data-row",
                            div { {format!("{} {} €", d.movement_type, d.amount_cents as f64 / 100.0)} }
                            div { class: "small", "{d.justification}" }
                        }
                    }
                }
            }

            section { class: "panel",
                h2 { "Garanties" }
                div { class: "form-grid",
                    FormField { label: "Type", value: guarantee_type(), oninput: move |e: FormEvent| guarantee_type.set(e.value()) }
                    FormField { label: "Garant", value: guarantor(), oninput: move |e: FormEvent| guarantor.set(e.value()) }
                    FormField { label: "Montant €", value: guarantee_amount(), oninput: move |e: FormEvent| guarantee_amount.set(e.value()) }
                    FormField { label: "Notes", value: guarantee_notes(), oninput: move |e: FormEvent| guarantee_notes.set(e.value()) }
                }
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let a = guarantee_amount().trim().parse::<f64>().ok().map(|v| (v * 100.0).round() as i64);
                        async move {
                            match add_lease_guarantee(id, guarantee_type(), guarantor(), a, None, None, None, guarantee_notes()).await {
                                Ok(_) => bump += 1,
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    }, "Ajouter une garantie"
                }
                div {
                    for g in guarantees.read().as_deref().unwrap_or(&[]).iter() {
                        div { class: "data-row",
                            div { {format!("{} • {}", g.guarantee_type, g.guarantor_name)} }
                            div { class: "small", "{g.notes}" }
                        }
                    }
                }
            }

            section { class: "panel",
                h2 { "Droit d'entrée" }
                div { class: "form-grid",
                    FormField { label: "Qualification", value: entry_status(), oninput: move |e: FormEvent| entry_status.set(e.value()) }
                    FormField { label: "Justification", value: entry_justification(), oninput: move |e: FormEvent| entry_justification.set(e.value()) }
                }
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let st = entry_status(); let j = entry_justification();
                        async move {
                            match set_lease_entry_fee(id, 0, st, j).await {
                                Ok(_) => bump += 1,
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    }, "Enregistrer la qualification"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{calculate_indexed_rent, next_anniversary, most_recent_anniversary,
                publication_date, quarter_end_date, expected_period_for_date};
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn indexed_rent_is_reproducible() {
        assert_eq!(
            calculate_indexed_rent(100_000, Decimal::new(1000, 0), Decimal::new(1050, 0), None).unwrap(),
            105_000
        );
    }

    #[test]
    fn indexed_rent_respects_positive_cap() {
        assert_eq!(
            calculate_indexed_rent(100_000, Decimal::new(1000, 0), Decimal::new(1200, 0), Some(500)).unwrap(),
            105_000
        );
    }

    #[test]
    fn icc_publication_date_uses_insee_lag() {
        assert_eq!(
            publication_date("T1 2026", 12, 3).unwrap(),
            NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()
        );
        assert_eq!(
            publication_date("T4 2025", 12, 3).unwrap(),
            NaiveDate::from_ymd_opt(2026, 3, 12).unwrap()
        );
    }

    #[test]
    fn expected_period_at_annual_anniversary() {
        assert_eq!(
            expected_period_for_date(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(), 12, 3).unwrap(),
            "T4 2025".to_string()
        );
    }

    #[test]
    fn expected_period_at_far_future_anniversary() {
        assert_eq!(
            expected_period_for_date(NaiveDate::from_ymd_opt(2029, 6, 1).unwrap(), 12, 3).unwrap(),
            "T4 2028".to_string()
        );
    }

    #[test]
    fn next_anniversary_annual() {
        let start = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        assert_eq!(
            next_anniversary(start, NaiveDate::from_ymd_opt(2026, 9, 30).unwrap(), 12),
            NaiveDate::from_ymd_opt(2027, 6, 1).unwrap()
        );
    }

    #[test]
    fn next_anniversary_triennial() {
        let start = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
        assert_eq!(
            next_anniversary(start, NaiveDate::from_ymd_opt(2026, 9, 30).unwrap(), 36),
            NaiveDate::from_ymd_opt(2027, 6, 1).unwrap()
        );
        assert_eq!(
            next_anniversary(start, NaiveDate::from_ymd_opt(2027, 9, 30).unwrap(), 36),
            NaiveDate::from_ymd_opt(2030, 6, 1).unwrap()
        );
    }

    #[test]
    fn most_recent_anniversary_for_triennial() {
        let start = NaiveDate::from_ymd_opt(2023, 6, 1).unwrap();
        assert_eq!(
            most_recent_anniversary(start, NaiveDate::from_ymd_opt(2026, 9, 30).unwrap(), 36),
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()
        );
    }

    #[test]
    fn quarter_end_is_last_day_of_quarter() {
        assert_eq!(quarter_end_date("T1 2025").unwrap(), NaiveDate::from_ymd_opt(2025, 3, 31).unwrap());
        assert_eq!(quarter_end_date("T2 2025").unwrap(), NaiveDate::from_ymd_opt(2025, 6, 30).unwrap());
        assert_eq!(quarter_end_date("T3 2025").unwrap(), NaiveDate::from_ymd_opt(2025, 9, 30).unwrap());
        assert_eq!(quarter_end_date("T4 2025").unwrap(), NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
    }
}