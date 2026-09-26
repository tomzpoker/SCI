use crate::domain::{
    LeaseChargeItem, LeaseClauseItem, LeaseDepositItem, LeaseDetailItem,
    LeaseGuaranteeItem, LeaseIndexItem, LeaseReductionItem, LeaseRentRevisionItem,
};
use crate::entity_scope::current_legal_entity_id;
use crate::ui::{FormField, ModuleHeader};
use chrono::NaiveDate;
use dioxus::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

#[cfg(feature = "server")]
use sqlx::Row;

fn parse_optional_date(value: &str) -> Result<Option<NaiveDate>, String> {
    if value.trim().is_empty() {
        return Ok(None);
    }

    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map(Some)
        .map_err(|_| "Date invalide".to_owned())
}

fn valid_charge_mode(v: &str) -> bool {
    matches!(
        v,
        "NONE" | "FORFAIT" | "PROVISION" | "PROVISION_VARIABLE" | "REGULARISATION"
    )
}

fn valid_frequency(v: &str) -> bool {
    matches!(v, "MONTHLY" | "QUARTERLY" | "ANNUAL")
}

fn valid_vat_mode(v: &str) -> bool {
    matches!(v, "FROM_UNIT" | "EXONERATED" | "OPTION" | "CUSTOM")
}

pub fn calculate_indexed_rent(
    old_rent_cents: i64,
    old_index: Decimal,
    new_index: Decimal,
    cap_bp: Option<i32>,
) -> Result<i64, String> {
    if old_rent_cents < 0
        || old_index <= Decimal::ZERO
        || new_index <= Decimal::ZERO
    {
        return Err("Paramètres d'indexation invalides".to_owned());
    }

    let raw = Decimal::from(old_rent_cents) * new_index / old_index;
    let mut capped = raw;

    if let Some(max_bp) = cap_bp {
        if max_bp < 0 || max_bp > 10000 {
            return Err("Plafond invalide".to_owned());
        }

        let ceiling = Decimal::from(old_rent_cents)
            * Decimal::from(10000 + max_bp as i64)
            / Decimal::from(10000);

        if raw > ceiling {
            capped = ceiling;
        }
    }

    capped
        .round()
        .to_i64()
        .ok_or_else(|| "Résultat d'indexation invalide".to_owned())
}

#[server]
pub async fn list_lease_details() -> Result<Vec<LeaseDetailItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let rows = sqlx::query(
            "SELECT
                l.id,
                l.reference,
                l.unit_id,
                u.label unit_label,
                p.name property_name,
                l.tenant_id,
                t.legal_name tenant_name,
                l.signature_date,
                l.start_date effect_date,
                l.end_date,
                l.lease_type,
                l.destination,
                l.rent_amount_cents,
                l.rent_frequency,
                l.payment_day,
                l.vat_mode,
                COALESCE(l.index_code,'') index_code,
                l.index_base_value,
                l.index_base_date,
                l.index_cap_bp,
                l.charges_mode,
                l.charges_amount_cents,
                l.security_deposit_expected_cents,
                l.entry_fee_expected_cents,
                l.entry_fee_status,
                l.active
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

        Ok(rows
            .into_iter()
            .map(|r| LeaseDetailItem {
                id: r.get("id"),
                reference: r.get("reference"),
                unit_id: r.get("unit_id"),
                unit_label: r.get("unit_label"),
                property_name: r.get("property_name"),
                tenant_id: r.get("tenant_id"),
                tenant_name: r.get("tenant_name"),
                signature_date: r.get("signature_date"),
                effect_date: r.get("effect_date"),
                end_date: r.get("end_date"),
                lease_type: r.get("lease_type"),
                destination: r.get("destination"),
                rent_amount_cents: r.get("rent_amount_cents"),
                rent_frequency: r.get("rent_frequency"),
                payment_day: r.get("payment_day"),
                vat_mode: r.get("vat_mode"),
                index_code: r.get("index_code"),
                index_base_value: r.get("index_base_value"),
                index_base_date: r.get("index_base_date"),
                index_cap_bp: r.get("index_cap_bp"),
                charges_mode: r.get("charges_mode"),
                charges_amount_cents: r.get("charges_amount_cents"),
                security_deposit_expected_cents: r.get("security_deposit_expected_cents"),
                entry_fee_expected_cents: r.get("entry_fee_expected_cents"),
                entry_fee_status: r.get("entry_fee_status"),
                active: r.get("active"),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "list_lease_details est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn create_structured_lease(
    unit_id: Uuid,
    tenant_id: Uuid,
    reference: String,
    signature_date: Option<NaiveDate>,
    effect_date: NaiveDate,
    end_date: Option<NaiveDate>,
    lease_type: String,
    destination: String,
    rent_amount_cents: i64,
    rent_frequency: String,
    payment_day: i16,
    vat_mode: String,
    index_code: String,
    index_base_value: Option<Decimal>,
    index_base_date: Option<NaiveDate>,
    index_cap_bp: Option<i32>,
    charges_mode: String,
    charges_amount_cents: i64,
    security_deposit_expected_cents: i64,
    entry_fee_expected_cents: i64,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let entity = current_legal_entity_id();

        if reference.trim().is_empty() {
            return Err(ServerFnError::new("Référence requise"));
        }

        if end_date.map(|d| d < effect_date).unwrap_or(false) {
            return Err(ServerFnError::new(
                "Échéance avant la date d'effet",
            ));
        }

        if rent_amount_cents < 0
            || charges_amount_cents < 0
            || security_deposit_expected_cents < 0
            || entry_fee_expected_cents < 0
        {
            return Err(ServerFnError::new("Montants invalides"));
        }

        if !valid_frequency(rent_frequency.trim())
            || !valid_vat_mode(vat_mode.trim())
            || !valid_charge_mode(charges_mode.trim())
        {
            return Err(ServerFnError::new("Paramètre de bail invalide"));
        }

        if payment_day < 1 || payment_day > 31 {
            return Err(ServerFnError::new("Jour de paiement invalide"));
        }

        if index_cap_bp
            .map(|v| v < 0 || v > 10000)
            .unwrap_or(false)
        {
            return Err(ServerFnError::new(
                "Plafond d'indexation invalide",
            ));
        }

        let valid: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint
             FROM units u
             JOIN properties p ON p.id = u.property_id
             WHERE u.id = $1
               AND p.legal_entity_id = $3
               AND EXISTS (
                   SELECT 1
                   FROM tenants
                   WHERE id = $2
                     AND legal_entity_id = $3
               )",
        )
        .bind(unit_id)
        .bind(tenant_id)
        .bind(entity)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;

        if valid != 1 {
            return Err(ServerFnError::new(
                "Lot ou locataire invalide",
            ));
        }

        let rent = if rent_amount_cents == 0 {
            sqlx::query_scalar::<_, i64>(
                "SELECT base_rent_cents
                 FROM units
                 WHERE id = $1",
            )
            .bind(unit_id)
            .fetch_one(pool)
            .await
            .map_err(ServerFnError::new)?
        } else {
            rent_amount_cents
        };

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO leases(
                legal_entity_id,
                unit_id,
                tenant_id,
                reference,
                start_date,
                end_date,
                notice_months,
                payment_day,
                annual_review_month,
                active,
                signature_date,
                lease_type,
                destination,
                rent_amount_cents,
                rent_frequency,
                vat_mode,
                index_code,
                index_base_value,
                index_base_date,
                index_cap_bp,
                charges_mode,
                charges_amount_cents,
                security_deposit_expected_cents,
                entry_fee_expected_cents,
                entry_fee_status,
                updated_at
             )
             VALUES(
                $1,$2,$3,$4,$5,$6,3,$7,NULL,true,$8,$9,$10,$11,$12,$13,
                NULLIF($14,''),
                $15,$16,$17,$18,$19,$20,$21,
                CASE WHEN $21 > 0 THEN 'UNCERTAIN' ELSE 'NOT_SET' END,
                now()
             )
             RETURNING id",
        )
        .bind(entity)
        .bind(unit_id)
        .bind(tenant_id)
        .bind(reference.trim())
        .bind(effect_date)
        .bind(end_date)
        .bind(payment_day)
        .bind(signature_date)
        .bind(lease_type.trim())
        .bind(destination.trim())
        .bind(rent)
        .bind(rent_frequency.trim())
        .bind(vat_mode.trim())
        .bind(index_code.trim())
        .bind(index_base_value)
        .bind(index_base_date)
        .bind(index_cap_bp)
        .bind(charges_mode.trim())
        .bind(charges_amount_cents)
        .bind(security_deposit_expected_cents)
        .bind(entry_fee_expected_cents)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;

        sqlx::query(
            "INSERT INTO lease_contract_events(
                legal_entity_id,
                lease_id,
                event_type,
                effective_date,
                before_payload,
                after_payload,
                reason
             )
             VALUES(
                $1,
                $2,
                'LEASE_CREATED',
                $3,
                '{}'::jsonb,
                $4,
                'Création structurée du bail'
             )",
        )
        .bind(entity)
        .bind(id)
        .bind(effect_date)
        .bind(json!({
            "reference": reference,
            "rent_amount_cents": rent,
            "rent_frequency": rent_frequency,
            "vat_mode": vat_mode,
            "charges_mode": charges_mode,
            "security_deposit_expected_cents": security_deposit_expected_cents,
            "entry_fee_expected_cents": entry_fee_expected_cents
        }))
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        if entry_fee_expected_cents > 0 {
            sqlx::query(
                "INSERT INTO lease_entry_fees(
                    legal_entity_id,
                    lease_id,
                    expected_amount_cents,
                    qualification_status,
                    justification,
                    alert_required
                 )
                 VALUES(
                    $1,
                    $2,
                    $3,
                    'UNCERTAIN',
                    'Qualification à vérifier selon la situation et les pièces du dossier',
                    true
                 )",
            )
            .bind(entity)
            .bind(id)
            .bind(entry_fee_expected_cents)
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;

            sqlx::query(
                "INSERT INTO tasks(
                    legal_entity_id,
                    code,
                    title,
                    description,
                    due_at,
                    state,
                    priority,
                    source,
                    blocking,
                    entity_type,
                    entity_id,
                    occurrence_key
                 )
                 VALUES(
                    $1,
                    'LEASE_ENTRY_FEE_CHECK',
                    'Vérifier le droit d''entrée',
                    'Qualification incertaine : vérifier le cadre juridique et les pièces.',
                    now(),
                    'READY',
                    85,
                    'LEASE_ENTRY_FEE',
                    true,
                    'LEASE',
                    $2,
                    $3
                 )
                 ON CONFLICT(legal_entity_id,code,occurrence_key)
                 DO NOTHING",
            )
            .bind(entity)
            .bind(id)
            .bind(format!("ENTRY_FEE:{}", id))
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        }

        Ok(id)
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (
            unit_id,
            tenant_id,
            reference,
            signature_date,
            effect_date,
            end_date,
            lease_type,
            destination,
            rent_amount_cents,
            rent_frequency,
            payment_day,
            vat_mode,
            index_code,
            index_base_value,
            index_base_date,
            index_cap_bp,
            charges_mode,
            charges_amount_cents,
            security_deposit_expected_cents,
            entry_fee_expected_cents,
        );

        Err(ServerFnError::new(
            "create_structured_lease est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn list_lease_clauses(
    lease_id: Uuid,
) -> Result<Vec<LeaseClauseItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let rows = sqlx::query(
            "SELECT
                id,
                lease_id,
                code,
                title,
                clause_type,
                body,
                effective_from,
                effective_to,
                version_no,
                active
             FROM lease_clauses
             WHERE lease_id = $1
               AND legal_entity_id = $2
             ORDER BY active DESC, code, version_no DESC",
        )
        .bind(lease_id)
        .bind(current_legal_entity_id())
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| LeaseClauseItem {
                id: r.get("id"),
                lease_id: r.get("lease_id"),
                code: r.get("code"),
                title: r.get("title"),
                clause_type: r.get("clause_type"),
                body: r.get("body"),
                effective_from: r.get("effective_from"),
                effective_to: r.get("effective_to"),
                version_no: r.get("version_no"),
                active: r.get("active"),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "list_lease_clauses est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn save_lease_clause(
    lease_id: Uuid,
    code: String,
    title: String,
    clause_type: String,
    body: String,
    effective_from: NaiveDate,
    effective_to: Option<NaiveDate>,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let entity = current_legal_entity_id();

        if code.trim().is_empty()
            || title.trim().is_empty()
            || body.trim().is_empty()
        {
            return Err(ServerFnError::new(
                "Code, titre et clause requis",
            ));
        }

        if effective_to
            .map(|d| d < effective_from)
            .unwrap_or(false)
        {
            return Err(ServerFnError::new("Fin de clause invalide"));
        }

        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1
                FROM leases
                WHERE id = $1
                  AND legal_entity_id = $2
            )",
        )
        .bind(lease_id)
        .bind(entity)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;

        if !exists {
            return Err(ServerFnError::new("Bail introuvable"));
        }

        let version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_no),0)+1
             FROM lease_clauses
             WHERE lease_id = $1
               AND code = $2
               AND legal_entity_id = $3",
        )
        .bind(lease_id)
        .bind(code.trim())
        .bind(entity)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_clauses(
                legal_entity_id,
                lease_id,
                code,
                title,
                clause_type,
                body,
                effective_from,
                effective_to,
                version_no,
                active
             )
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,true)
             RETURNING id",
        )
        .bind(entity)
        .bind(lease_id)
        .bind(code.trim())
        .bind(title.trim())
        .bind(clause_type.trim())
        .bind(body.trim())
        .bind(effective_from)
        .bind(effective_to)
        .bind(version)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(id)
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (
            lease_id,
            code,
            title,
            clause_type,
            body,
            effective_from,
            effective_to,
        );

        Err(ServerFnError::new(
            "save_lease_clause est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn list_lease_indices(
    index_code: String,
) -> Result<Vec<LeaseIndexItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let rows = sqlx::query(
            "SELECT
                id,
                index_code,
                period_label,
                value,
                source_reference,
                verified_at
             FROM lease_index_values
             WHERE legal_entity_id = $1
               AND ($2 = '' OR index_code = $2)
             ORDER BY index_code, period_label DESC",
        )
        .bind(current_legal_entity_id())
        .bind(index_code.trim())
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| LeaseIndexItem {
                id: r.get("id"),
                index_code: r.get("index_code"),
                period_label: r.get("period_label"),
                value: r.get("value"),
                source_reference: r.get("source_reference"),
                verified_at: r.get("verified_at"),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "list_lease_indices est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn save_lease_index(
    index_code: String,
    period_label: String,
    value: Decimal,
    source_reference: String,
    verified_at: Option<NaiveDate>,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        if index_code.trim().is_empty()
            || period_label.trim().is_empty()
        {
            return Err(ServerFnError::new(
                "Indice et période requis",
            ));
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_index_values(
                legal_entity_id,
                index_code,
                period_label,
                value,
                source_reference,
                verified_at
             )
             VALUES($1,$2,$3,$4,$5,$6)
             ON CONFLICT(legal_entity_id,index_code,period_label)
             DO UPDATE SET
                value = EXCLUDED.value,
                source_reference = EXCLUDED.source_reference,
                verified_at = EXCLUDED.verified_at
             RETURNING id",
        )
        .bind(current_legal_entity_id())
        .bind(index_code.trim().to_uppercase())
        .bind(period_label.trim())
        .bind(value)
        .bind(source_reference.trim())
        .bind(verified_at)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(id)
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (
            index_code,
            period_label,
            value,
            source_reference,
            verified_at,
        );

        Err(ServerFnError::new(
            "save_lease_index est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn calculate_lease_rent_revision(
    lease_id: Uuid,
    clause_code: String,
    index_period: String,
    effective_date: NaiveDate,
) -> Result<LeaseRentRevisionItem, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let entity = current_legal_entity_id();

        let lease = sqlx::query(
            "SELECT
                rent_amount_cents,
                COALESCE(index_code, '') AS index_code,
                index_base_value,
                index_cap_bp
             FROM leases
             WHERE id = $1
               AND legal_entity_id = $2",
        )
        .bind(lease_id)
        .bind(entity)
        .fetch_optional(pool)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Bail introuvable"))?;

        let old_rent: i64 = lease.get("rent_amount_cents");
        let code: String = lease.get("index_code");

        if code.is_empty() {
            return Err(ServerFnError::new(
                "Aucun indice configuré sur ce bail",
            ));
        }

        let old_index: Decimal = lease
            .try_get::<Option<Decimal>, _>("index_base_value")
            .map_err(ServerFnError::new)?
            .ok_or_else(|| {
                ServerFnError::new(
                    "Valeur d'indice de base manquante",
                )
            })?;

        let cap: Option<i32> = lease.get("index_cap_bp");

        let row = sqlx::query(
            "SELECT value
             FROM lease_index_values
             WHERE legal_entity_id = $1
               AND index_code = $2
               AND period_label = $3",
        )
        .bind(entity)
        .bind(&code)
        .bind(index_period.trim())
        .fetch_optional(pool)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| {
            ServerFnError::new(
                "Indice de la période introuvable",
            )
        })?;

        let new_index: Decimal = row.get("value");

        if old_index <= Decimal::ZERO
            || new_index <= Decimal::ZERO
        {
            return Err(ServerFnError::new("Indices invalides"));
        }

        let new_rent = calculate_indexed_rent(
            old_rent,
            old_index,
            new_index,
            cap,
        )
        .map_err(ServerFnError::new)?;

        let formula = if cap.is_some() {
            format!(
                "max(0, min({old_rent} × {new_index} / {old_index}, plafond {cap:?}))"
            )
        } else {
            format!(
                "{old_rent} × {new_index} / {old_index}"
            )
        };

        let rule = if clause_code.trim().is_empty() {
            "Indexation contractuelle".to_owned()
        } else {
            sqlx::query_scalar::<_, String>(
                "SELECT body
                 FROM lease_clauses
                 WHERE legal_entity_id = $1
                   AND lease_id = $2
                   AND code = $3
                   AND active
                 ORDER BY version_no DESC
                 LIMIT 1",
            )
            .bind(entity)
            .bind(lease_id)
            .bind(clause_code.trim())
            .fetch_optional(pool)
            .await
            .map_err(ServerFnError::new)?
            .unwrap_or_else(|| {
                "Clause sélectionnée".to_owned()
            })
        };

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_rent_revisions(
                legal_entity_id,
                lease_id,
                calculation_date,
                effective_date,
                rule_text,
                index_code,
                index_period,
                old_rent_cents,
                index_old,
                index_new,
                cap_bp,
                new_rent_cents,
                formula,
                result_status
             )
             VALUES(
                $1,$2,CURRENT_DATE,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,
                'CALCULATED'
             )
             RETURNING id",
        )
        .bind(entity)
        .bind(lease_id)
        .bind(effective_date)
        .bind(&rule)
        .bind(&code)
        .bind(index_period.trim())
        .bind(old_rent)
        .bind(old_index)
        .bind(new_index)
        .bind(cap)
        .bind(new_rent)
        .bind(&formula)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(LeaseRentRevisionItem {
            id,
            lease_id,
            calculation_date: chrono::Utc::now().date_naive(),
            effective_date,
            rule_text: rule,
            index_code: code,
            index_period: index_period.trim().to_owned(),
            old_rent_cents: old_rent,
            index_old: Some(old_index),
            index_new: Some(new_index),
            cap_bp: cap,
            new_rent_cents: new_rent,
            formula,
            result_status: "CALCULATED".into(),
        })
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (
            lease_id,
            clause_code,
            index_period,
            effective_date,
        );

        Err(ServerFnError::new(
            "calculate_lease_rent_revision est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn add_lease_charge(
    lease_id: Uuid,
    charge_type: String,
    mode: String,
    amount_cents: i64,
    variable_formula: String,
    effective_from: NaiveDate,
    effective_to: Option<NaiveDate>,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        if !valid_charge_mode(mode.trim()) || amount_cents < 0 {
            return Err(ServerFnError::new("Charge invalide"));
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_charge_rules(
                legal_entity_id,
                lease_id,
                charge_type,
                mode,
                amount_cents,
                variable_formula,
                effective_from,
                effective_to
             )
             SELECT
                $1,$2,$3,$4,$5,$6,$7,$8
             WHERE EXISTS(
                SELECT 1
                FROM leases
                WHERE id = $2
                  AND legal_entity_id = $1
             )
             RETURNING id",
        )
        .bind(current_legal_entity_id())
        .bind(lease_id)
        .bind(charge_type.trim())
        .bind(mode.trim())
        .bind(amount_cents)
        .bind(variable_formula.trim())
        .bind(effective_from)
        .bind(effective_to)
        .fetch_optional(pool)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Bail introuvable"))?;

        Ok(id)
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (
            lease_id,
            charge_type,
            mode,
            amount_cents,
            variable_formula,
            effective_from,
            effective_to,
        );

        Err(ServerFnError::new(
            "add_lease_charge est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn list_lease_charges(
    lease_id: Uuid,
) -> Result<Vec<LeaseChargeItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let rows = sqlx::query(
            "SELECT
                id,
                lease_id,
                charge_type,
                mode,
                amount_cents,
                variable_formula,
                effective_from,
                effective_to,
                active
             FROM lease_charge_rules
             WHERE lease_id = $1
               AND legal_entity_id = $2
             ORDER BY active DESC, effective_from DESC",
        )
        .bind(lease_id)
        .bind(current_legal_entity_id())
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| LeaseChargeItem {
                id: r.get("id"),
                lease_id: r.get("lease_id"),
                charge_type: r.get("charge_type"),
                mode: r.get("mode"),
                amount_cents: r.get("amount_cents"),
                variable_formula: r.get("variable_formula"),
                effective_from: r.get("effective_from"),
                effective_to: r.get("effective_to"),
                active: r.get("active"),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "list_lease_charges est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn add_lease_reduction(
    lease_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
    amount_cents: Option<i64>,
    percentage_bp: Option<i32>,
    reason: String,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let entity = current_legal_entity_id();

        if end_date < start_date || reason.trim().is_empty() {
            return Err(ServerFnError::new(
                "Réduction invalide",
            ));
        }

        if amount_cents.is_none() && percentage_bp.is_none() {
            return Err(ServerFnError::new(
                "Montant ou pourcentage requis",
            ));
        }

        if percentage_bp
            .map(|v| v < 0 || v > 10000)
            .unwrap_or(false)
        {
            return Err(ServerFnError::new(
                "Pourcentage invalide",
            ));
        }

        let old: i64 = sqlx::query_scalar(
            "SELECT rent_amount_cents
             FROM leases
             WHERE id = $1
               AND legal_entity_id = $2",
        )
        .bind(lease_id)
        .bind(entity)
        .fetch_optional(pool)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Bail introuvable"))?;

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_rent_reductions(
                legal_entity_id,
                lease_id,
                start_date,
                end_date,
                amount_cents,
                percentage_bp,
                reason,
                original_rent_cents
             )
             VALUES($1,$2,$3,$4,$5,$6,$7,$8)
             RETURNING id",
        )
        .bind(entity)
        .bind(lease_id)
        .bind(start_date)
        .bind(end_date)
        .bind(amount_cents)
        .bind(percentage_bp)
        .bind(reason.trim())
        .bind(old)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;

        sqlx::query(
            "INSERT INTO lease_contract_events(
                legal_entity_id,
                lease_id,
                event_type,
                effective_date,
                before_payload,
                after_payload,
                reason
             )
             VALUES(
                $1,
                $2,
                'TEMPORARY_RENT_REDUCTION',
                $3,
                $4,
                $5,
                $6
             )",
        )
        .bind(entity)
        .bind(lease_id)
        .bind(start_date)
        .bind(json!({
            "rent_amount_cents": old
        }))
        .bind(json!({
            "reduction_id": id,
            "start_date": start_date,
            "end_date": end_date,
            "amount_cents": amount_cents,
            "percentage_bp": percentage_bp
        }))
        .bind(reason.trim())
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(id)
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (
            lease_id,
            start_date,
            end_date,
            amount_cents,
            percentage_bp,
            reason,
        );

        Err(ServerFnError::new(
            "add_lease_reduction est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn list_lease_reductions(
    lease_id: Uuid,
) -> Result<Vec<LeaseReductionItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let rows = sqlx::query(
            "SELECT
                id,
                lease_id,
                start_date,
                end_date,
                amount_cents,
                percentage_bp,
                reason,
                original_rent_cents
             FROM lease_rent_reductions
             WHERE lease_id = $1
               AND legal_entity_id = $2
             ORDER BY start_date DESC",
        )
        .bind(lease_id)
        .bind(current_legal_entity_id())
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| LeaseReductionItem {
                id: r.get("id"),
                lease_id: r.get("lease_id"),
                start_date: r.get("start_date"),
                end_date: r.get("end_date"),
                amount_cents: r.get("amount_cents"),
                percentage_bp: r.get("percentage_bp"),
                reason: r.get("reason"),
                original_rent_cents: r.get("original_rent_cents"),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "list_lease_reductions est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn add_lease_deposit(
    lease_id: Uuid,
    movement_type: String,
    amount_cents: i64,
    movement_date: NaiveDate,
    justification: String,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        if amount_cents < 0
            || !matches!(
                movement_type.trim(),
                "EXPECTED" | "RECEIVED" | "RESTITUTION" | "RETAINED" | "ADJUSTMENT"
            )
        {
            return Err(ServerFnError::new(
                "Mouvement de dépôt invalide",
            ));
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_deposits(
                legal_entity_id,
                lease_id,
                movement_type,
                amount_cents,
                movement_date,
                justification
             )
             SELECT $1,$2,$3,$4,$5,$6
             WHERE EXISTS(
                SELECT 1
                FROM leases
                WHERE id = $2
                  AND legal_entity_id = $1
             )
             RETURNING id",
        )
        .bind(current_legal_entity_id())
        .bind(lease_id)
        .bind(movement_type.trim())
        .bind(amount_cents)
        .bind(movement_date)
        .bind(justification.trim())
        .fetch_optional(pool)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Bail introuvable"))?;

        Ok(id)
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (
            lease_id,
            movement_type,
            amount_cents,
            movement_date,
            justification,
        );

        Err(ServerFnError::new(
            "add_lease_deposit est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn list_lease_deposits(
    lease_id: Uuid,
) -> Result<Vec<LeaseDepositItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let rows = sqlx::query(
            "SELECT
                id,
                lease_id,
                movement_type,
                amount_cents,
                movement_date,
                justification,
                bank_transaction_id
             FROM lease_deposits
             WHERE lease_id = $1
               AND legal_entity_id = $2
             ORDER BY movement_date DESC, created_at DESC",
        )
        .bind(lease_id)
        .bind(current_legal_entity_id())
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| LeaseDepositItem {
                id: r.get("id"),
                lease_id: r.get("lease_id"),
                movement_type: r.get("movement_type"),
                amount_cents: r.get("amount_cents"),
                movement_date: r.get("movement_date"),
                justification: r.get("justification"),
                bank_transaction_id: r.get("bank_transaction_id"),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "list_lease_deposits est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn add_lease_guarantee(
    lease_id: Uuid,
    guarantee_type: String,
    guarantor_name: String,
    amount_cents: Option<i64>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    document_id: Option<Uuid>,
    notes: String,
) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        if guarantor_name.trim().is_empty() {
            return Err(ServerFnError::new("Garant requis"));
        }

        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO lease_guarantees(
                legal_entity_id,
                lease_id,
                guarantee_type,
                guarantor_name,
                amount_cents,
                start_date,
                end_date,
                document_id,
                notes
             )
             SELECT $1,$2,$3,$4,$5,$6,$7,$8,$9
             WHERE EXISTS(
                SELECT 1
                FROM leases
                WHERE id = $2
                  AND legal_entity_id = $1
             )
             RETURNING id",
        )
        .bind(current_legal_entity_id())
        .bind(lease_id)
        .bind(guarantee_type.trim())
        .bind(guarantor_name.trim())
        .bind(amount_cents)
        .bind(start_date)
        .bind(end_date)
        .bind(document_id)
        .bind(notes.trim())
        .fetch_optional(pool)
        .await
        .map_err(ServerFnError::new)?
        .ok_or_else(|| ServerFnError::new("Bail introuvable"))?;

        Ok(id)
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (
            lease_id,
            guarantee_type,
            guarantor_name,
            amount_cents,
            start_date,
            end_date,
            document_id,
            notes,
        );

        Err(ServerFnError::new(
            "add_lease_guarantee est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn list_lease_guarantees(
    lease_id: Uuid,
) -> Result<Vec<LeaseGuaranteeItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let rows = sqlx::query(
            "SELECT
                id,
                lease_id,
                guarantee_type,
                guarantor_name,
                amount_cents,
                start_date,
                end_date,
                document_id,
                notes,
                active
             FROM lease_guarantees
             WHERE lease_id = $1
               AND legal_entity_id = $2
             ORDER BY active DESC, created_at DESC",
        )
        .bind(lease_id)
        .bind(current_legal_entity_id())
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| LeaseGuaranteeItem {
                id: r.get("id"),
                lease_id: r.get("lease_id"),
                guarantee_type: r.get("guarantee_type"),
                guarantor_name: r.get("guarantor_name"),
                amount_cents: r.get("amount_cents"),
                start_date: r.get("start_date"),
                end_date: r.get("end_date"),
                document_id: r.get("document_id"),
                notes: r.get("notes"),
                active: r.get("active"),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "list_lease_guarantees est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn set_lease_entry_fee(
    lease_id: Uuid,
    expected_amount_cents: i64,
    qualification_status: String,
    justification: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let entity = current_legal_entity_id();

        if expected_amount_cents < 0
            || !matches!(
                qualification_status.trim(),
                "CONFIRMED" | "PENDING" | "UNCERTAIN" | "NOT_APPLICABLE"
            )
        {
            return Err(ServerFnError::new(
                "Qualification du droit d'entrée invalide",
            ));
        }

        let amount = if expected_amount_cents == 0 {
            sqlx::query_scalar::<_, i64>(
                "SELECT entry_fee_expected_cents
                 FROM leases
                 WHERE id = $1
                   AND legal_entity_id = $2",
            )
            .bind(lease_id)
            .bind(entity)
            .fetch_optional(pool)
            .await
            .map_err(ServerFnError::new)?
            .unwrap_or(0)
        } else {
            expected_amount_cents
        };

        sqlx::query(
            "INSERT INTO lease_entry_fees(
                legal_entity_id,
                lease_id,
                expected_amount_cents,
                qualification_status,
                justification,
                alert_required
             )
             VALUES(
                $1,$2,$3,$4,$5,$4='UNCERTAIN'
             )
             ON CONFLICT(legal_entity_id,lease_id)
             DO UPDATE SET
                expected_amount_cents = EXCLUDED.expected_amount_cents,
                qualification_status = EXCLUDED.qualification_status,
                justification = EXCLUDED.justification,
                alert_required = EXCLUDED.alert_required,
                updated_at = now()",
        )
        .bind(entity)
        .bind(lease_id)
        .bind(amount)
        .bind(qualification_status.trim())
        .bind(justification.trim())
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        sqlx::query(
            "UPDATE leases
             SET
                entry_fee_expected_cents = $3,
                entry_fee_status = $4,
                updated_at = now()
             WHERE id = $1
               AND legal_entity_id = $2",
        )
        .bind(lease_id)
        .bind(entity)
        .bind(amount)
        .bind(qualification_status.trim())
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        sqlx::query(
            "INSERT INTO lease_contract_events(
                legal_entity_id,
                lease_id,
                event_type,
                effective_date,
                after_payload,
                reason
             )
             VALUES(
                $1,
                $2,
                'ENTRY_FEE_QUALIFICATION',
                CURRENT_DATE,
                $3,
                $4
             )",
        )
        .bind(entity)
        .bind(lease_id)
        .bind(json!({
            "expected_amount_cents": amount,
            "qualification_status": qualification_status
        }))
        .bind(justification.trim())
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        if qualification_status.trim() != "UNCERTAIN" {
            sqlx::query(
                "UPDATE tasks
                 SET
                    state = 'DONE',
                    completed_at = now()
                 WHERE legal_entity_id = $1
                   AND code = 'LEASE_ENTRY_FEE_CHECK'
                   AND entity_id = $2
                   AND state <> 'DONE'",
            )
            .bind(entity)
            .bind(lease_id)
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        }

        Ok(())
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = (
            lease_id,
            expected_amount_cents,
            qualification_status,
            justification,
        );

        Err(ServerFnError::new(
            "set_lease_entry_fee est exécutée côté serveur",
        ))
    }
}

#[server]
pub async fn list_lease_revisions(
    lease_id: Uuid,
) -> Result<Vec<LeaseRentRevisionItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = crate::infrastructure::db()
            .await
            .map_err(ServerFnError::new)?;

        let rows = sqlx::query(
            "SELECT
                id,
                lease_id,
                calculation_date,
                effective_date,
                rule_text,
                index_code,
                index_period,
                old_rent_cents,
                index_old,
                index_new,
                cap_bp,
                new_rent_cents,
                formula,
                result_status
             FROM lease_rent_revisions
             WHERE lease_id = $1
               AND legal_entity_id = $2
             ORDER BY calculation_date DESC, created_at DESC",
        )
        .bind(lease_id)
        .bind(current_legal_entity_id())
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| LeaseRentRevisionItem {
                id: r.get("id"),
                lease_id: r.get("lease_id"),
                calculation_date: r.get("calculation_date"),
                effective_date: r.get("effective_date"),
                rule_text: r.get("rule_text"),
                index_code: r.get("index_code"),
                index_period: r.get("index_period"),
                old_rent_cents: r.get("old_rent_cents"),
                index_old: r.get("index_old"),
                index_new: r.get("index_new"),
                cap_bp: r.get("cap_bp"),
                new_rent_cents: r.get("new_rent_cents"),
                formula: r.get("formula"),
                result_status: r.get("result_status"),
            })
            .collect())
    }

    #[cfg(not(feature = "server"))]
    {
        Err(ServerFnError::new(
            "list_lease_revisions est exécutée côté serveur",
        ))
    }
}

#[component]
pub fn LeasesPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);

    let leases = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move {
            list_lease_details().await.unwrap_or_default()
        }
    });

    let units = use_resource(move || {
        let _ = refresh();
        async move {
            crate::server::list_units()
                .await
                .unwrap_or_default()
        }
    });

    let tenants = use_resource(move || {
        let _ = refresh();
        async move {
            crate::server::list_tenants()
                .await
                .unwrap_or_default()
        }
    });

    let mut selected = use_signal(|| None::<Uuid>);
    let mut reference = use_signal(String::new);
    let mut effect =
        use_signal(|| chrono::Utc::now().date_naive().to_string());
    let mut end = use_signal(String::new);
    let mut signature = use_signal(String::new);
    let mut rent = use_signal(String::new);
    let mut destination = use_signal(String::new);
    let mut lease_type =
        use_signal(|| "BAIL_COMMERCIAL".to_owned());
    let mut rent_frequency =
        use_signal(|| "MONTHLY".to_owned());
    let mut vat_mode =
        use_signal(|| "FROM_UNIT".to_owned());
    let mut index_code = use_signal(|| "ILC".to_owned());
    let mut index_base = use_signal(String::new);
    let mut index_base_date = use_signal(String::new);
    let mut index_cap = use_signal(String::new);
    let mut entry_fee = use_signal(String::new);
    let mut security_deposit = use_signal(String::new);
    let mut msg = use_signal(String::new);

    let mut unit = use_signal(String::new);
    let mut tenant = use_signal(String::new);

    let clauses = use_resource(move || {
        let id = selected();

        async move {
            match id {
                Some(i) => list_lease_clauses(i)
                    .await
                    .unwrap_or_default(),
                None => Vec::new(),
            }
        }
    });

    let revisions = use_resource(move || {
        let id = selected();

        async move {
            match id {
                Some(i) => list_lease_revisions(i)
                    .await
                    .unwrap_or_default(),
                None => Vec::new(),
            }
        }
    });

    let charges = use_resource(move || {
        let id = selected();

        async move {
            match id {
                Some(i) => list_lease_charges(i)
                    .await
                    .unwrap_or_default(),
                None => Vec::new(),
            }
        }
    });

    let reductions = use_resource(move || {
        let id = selected();

        async move {
            match id {
                Some(i) => list_lease_reductions(i)
                    .await
                    .unwrap_or_default(),
                None => Vec::new(),
            }
        }
    });

    let deposits = use_resource(move || {
        let id = selected();

        async move {
            match id {
                Some(i) => list_lease_deposits(i)
                    .await
                    .unwrap_or_default(),
                None => Vec::new(),
            }
        }
    });

    let guarantees = use_resource(move || {
        let id = selected();

        async move {
            match id {
                Some(i) => list_lease_guarantees(i)
                    .await
                    .unwrap_or_default(),
                None => Vec::new(),
            }
        }
    });

    let indices = use_resource(move || {
        let code = index_code();

        async move {
            list_lease_indices(code)
                .await
                .unwrap_or_default()
        }
    });

    let mut clause_code =
        use_signal(|| "INDEXATION".to_owned());
    let mut clause_body = use_signal(String::new);
    let mut index_period = use_signal(String::new);
    let mut revision_date =
        use_signal(|| chrono::Utc::now().date_naive().to_string());

    let mut charge_type =
        use_signal(|| "COPROPRIETE".to_owned());
    let mut charge_mode =
        use_signal(|| "PROVISION".to_owned());
    let mut charge_amount = use_signal(String::new);
    let mut charge_formula = use_signal(String::new);

    let mut reduction_start =
        use_signal(|| chrono::Utc::now().date_naive().to_string());
    let mut reduction_end = use_signal(|| {
        (chrono::Utc::now().date_naive()
            + chrono::Duration::days(30))
        .to_string()
    });
    let mut reduction_amount = use_signal(String::new);
    let mut reduction_pct = use_signal(String::new);
    let mut reduction_reason = use_signal(String::new);

    let mut deposit_type =
        use_signal(|| "RECEIVED".to_owned());
    let mut deposit_amount = use_signal(String::new);
    let mut deposit_date =
        use_signal(|| chrono::Utc::now().date_naive().to_string());
    let mut deposit_justification =
        use_signal(String::new);

    let mut guarantee_type =
        use_signal(|| "CAUTION".to_owned());
    let mut guarantor = use_signal(String::new);
    let mut guarantee_amount = use_signal(String::new);
    let mut guarantee_notes = use_signal(String::new);

    let mut entry_status =
        use_signal(|| "UNCERTAIN".to_owned());
    let mut entry_justification =
        use_signal(String::new);

    rsx! {
        ModuleHeader {
            title: "Baux & locations",
            kicker: "CONTRATS • INDEXATION • CHARGES • DÉPÔTS • GARANTIES",
            detail: "Le bail structuré conserve l’historique et traite les modifications comme des événements contractuels."
        }

        section {
            class: "panel",

            h2 { "Créer un bail structuré" }

            div {
                class: "form-grid",

                label {
                    class: "field",

                    span { "Lot" }

                    select {
                        value: unit(),
                        onchange: move |e: FormEvent| unit.set(e.value()),

                        option {
                            value: "",
                            "Sélectionner"
                        }

                        for u in units.read().as_deref().unwrap_or(&[]).iter() {
                            option {
                                value: u.id.to_string(),
                                {format!("{} • {}", u.property_name, u.label)}
                            }
                        }
                    }
                }

                label {
                    class: "field",

                    span { "Locataire" }

                    select {
                        value: tenant(),
                        onchange: move |e: FormEvent| tenant.set(e.value()),

                        option {
                            value: "",
                            "Sélectionner"
                        }

                        for t in tenants.read().as_deref().unwrap_or(&[]).iter() {
                            option {
                                value: t.id.to_string(),
                                "{t.legal_name}"
                            }
                        }
                    }
                }

                FormField {
                    label: "Référence",
                    value: reference(),
                    oninput: move |e: FormEvent| reference.set(e.value())
                }

                FormField {
                    label: "Signature AAAA-MM-JJ",
                    value: signature(),
                    oninput: move |e: FormEvent| signature.set(e.value())
                }

                FormField {
                    label: "Effet AAAA-MM-JJ",
                    value: effect(),
                    oninput: move |e: FormEvent| effect.set(e.value())
                }

                FormField {
                    label: "Échéance AAAA-MM-JJ",
                    value: end(),
                    oninput: move |e: FormEvent| end.set(e.value())
                }

                FormField {
                    label: "Type",
                    value: lease_type(),
                    oninput: move |e: FormEvent| lease_type.set(e.value())
                }

                FormField {
                    label: "Destination",
                    value: destination(),
                    oninput: move |e: FormEvent| destination.set(e.value())
                }

                FormField {
                    label: "Loyer €",
                    value: rent(),
                    oninput: move |e: FormEvent| rent.set(e.value())
                }

                FormField {
                    label: "Périodicité",
                    value: rent_frequency(),
                    oninput: move |e: FormEvent| rent_frequency.set(e.value())
                }

                FormField {
                    label: "TVA",
                    value: vat_mode(),
                    oninput: move |e: FormEvent| vat_mode.set(e.value())
                }

                FormField {
                    label: "Indice (ICC/ILC)",
                    value: index_code(),
                    oninput: move |e: FormEvent| index_code.set(e.value())
                }

                FormField {
                    label: "Indice base",
                    value: index_base(),
                    oninput: move |e: FormEvent| index_base.set(e.value())
                }

                FormField {
                    label: "Date indice base",
                    value: index_base_date(),
                    oninput: move |e: FormEvent| index_base_date.set(e.value())
                }

                FormField {
                    label: "Plafond % (optionnel)",
                    value: index_cap(),
                    oninput: move |e: FormEvent| index_cap.set(e.value())
                }

                FormField {
                    label: "Dépôt prévu €",
                    value: security_deposit(),
                    oninput: move |e: FormEvent| security_deposit.set(e.value())
                }

                FormField {
                    label: "Droit d’entrée €",
                    value: entry_fee(),
                    oninput: move |e: FormEvent| entry_fee.set(e.value())
                }
            }

            button {
                class: "primary",

                onclick: move |_| {
                    let u = Uuid::parse_str(&unit());
                    let t = Uuid::parse_str(&tenant());
                    let e = NaiveDate::parse_from_str(
                        &effect(),
                        "%Y-%m-%d"
                    );

                    async move {
                        match (u, t, e) {
                            (Ok(u), Ok(t), Ok(eff)) => {
                                match (
                                    parse_optional_date(&signature()),
                                    parse_optional_date(&end()),
                                ) {
                                    (Ok(sig), Ok(en)) => {
                                        let r = (
                                            rent()
                                                .replace(',', ".")
                                                .parse::<f64>()
                                                .unwrap_or(0.0)
                                                * 100.0
                                        )
                                            .round() as i64;

                                        let b = index_base()
                                            .replace(',', ".")
                                            .parse::<Decimal>()
                                            .ok();

                                        let cap = index_cap()
                                            .trim()
                                            .parse::<i32>()
                                            .ok();

                                        let deposit = (
                                            security_deposit()
                                                .replace(',', ".")
                                                .parse::<f64>()
                                                .unwrap_or(0.0)
                                                * 100.0
                                        )
                                            .round() as i64;

                                        let entry = (
                                            entry_fee()
                                                .replace(',', ".")
                                                .parse::<f64>()
                                                .unwrap_or(0.0)
                                                * 100.0
                                        )
                                            .round() as i64;

                                        let base_date =
                                            parse_optional_date(
                                                &index_base_date()
                                            )
                                            .ok()
                                            .flatten();

                                        match create_structured_lease(
                                            u,
                                            t,
                                            reference(),
                                            sig,
                                            eff,
                                            en,
                                            lease_type(),
                                            destination(),
                                            r,
                                            rent_frequency(),
                                            5,
                                            vat_mode(),
                                            index_code(),
                                            b,
                                            base_date,
                                            cap,
                                            "NONE".into(),
                                            0,
                                            deposit,
                                            entry,
                                        )
                                        .await
                                        {
                                            Ok(id) => {
                                                selected.set(Some(id));
                                                msg.set(
                                                    "Bail structuré créé"
                                                        .into()
                                                );
                                                bump += 1;
                                            }

                                            Err(e) => {
                                                msg.set(e.to_string());
                                            }
                                        }
                                    }

                                    _ => {
                                        msg.set(
                                            "Date invalide".into()
                                        );
                                    }
                                }
                            }

                            _ => {
                                msg.set(
                                    "Lot, locataire ou effet invalide"
                                        .into()
                                );
                            }
                        }
                    }
                },

                "Créer"
            }

            span {
                class: "save-ok",
                "{msg}"
            }
        }

        for l in leases.read().as_deref().unwrap_or(&[]).iter().cloned() {
            div {
                class: "data-row",

                div {
                    div {
                        class: "data-title",
                        "{l.reference} • {l.tenant_name}"
                    }

                    div {
                        class: "small",
                        "{l.property_name} / {l.unit_label} • loyer {l.rent_amount_cents as f64 / 100.0} • {l.entry_fee_status}"
                    }
                }

                div {
                    class: "row-actions",

                    button {
                        class: "secondary",
                        onclick: move |_| selected.set(Some(l.id)),
                        "Détails"
                    }
                }
            }
        }

        if let Some(id) = selected() {
            section {
                class: "panel",

                h2 { "Clause exploitable" }

                div {
                    class: "form-grid",

                    FormField {
                        label: "Code",
                        value: clause_code(),
                        oninput: move |e: FormEvent| clause_code.set(e.value())
                    }

                    FormField {
                        label: "Texte",
                        value: clause_body(),
                        oninput: move |e: FormEvent| clause_body.set(e.value())
                    }
                }

                button {
                    class: "secondary",

                    onclick: move |_| {
                        let c = clause_code();
                        let b = clause_body();

                        async move {
                            match save_lease_clause(
                                id,
                                c,
                                "Clause contractuelle".into(),
                                "GENERAL".into(),
                                b,
                                chrono::Utc::now().date_naive(),
                                None,
                            )
                            .await
                            {
                                Ok(_) => bump += 1,
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    },

                    "Ajouter une version de clause"
                }

                div {
                    for c in clauses.read().as_deref().unwrap_or(&[]).iter() {
                        div {
                            class: "data-row",

                            div {
                                {format!("{} v{}", c.code, c.version_no)}
                            }

                            div {
                                class: "small",
                                "{c.body}"
                            }
                        }
                    }
                }
            }

            section {
                class: "panel",

                h2 { "Révision de loyer" }

                div {
                    class: "form-grid",

                    FormField {
                        label: "Code clause",
                        value: clause_code(),
                        oninput: move |e: FormEvent| clause_code.set(e.value())
                    }

                    FormField {
                        label: "Période indice",
                        value: index_period(),
                        oninput: move |e: FormEvent| index_period.set(e.value())
                    }

                    FormField {
                        label: "Date d'effet",
                        value: revision_date(),
                        oninput: move |e: FormEvent| revision_date.set(e.value())
                    }
                }

                button {
                    class: "primary",

                    onclick: move |_| {
                        let c = clause_code();
                        let p = index_period();
                        let d = NaiveDate::parse_from_str(
                            &revision_date(),
                            "%Y-%m-%d"
                        );

                        async move {
                            if let Ok(d) = d {
                                match calculate_lease_rent_revision(
                                    id,
                                    c,
                                    p,
                                    d,
                                )
                                .await
                                {
                                    Ok(_) => bump += 1,
                                    Err(e) => msg.set(e.to_string()),
                                }
                            } else {
                                msg.set("Date invalide".into());
                            }
                        }
                    },

                    "Calculer la révision"
                }

                div {
                    for r in revisions.read().as_deref().unwrap_or(&[]).iter() {
                        div {
                            class: "data-row",

                            div {
                                {format!("{} → {} €", r.index_period, r.new_rent_cents as f64 / 100.0)}
                            }

                            div {
                                class: "small",
                                "{r.formula}"
                            }
                        }
                    }
                }
            }

            section {
                class: "panel",

                h2 { "Historique ICC / ILC" }

                div {
                    class: "form-grid",

                    FormField {
                        label: "Indice",
                        value: index_code(),
                        oninput: move |e: FormEvent| index_code.set(e.value())
                    }

                    FormField {
                        label: "Période",
                        value: index_period(),
                        oninput: move |e: FormEvent| index_period.set(e.value())
                    }

                    FormField {
                        label: "Valeur",
                        value: index_base(),
                        oninput: move |e: FormEvent| index_base.set(e.value())
                    }
                }

                button {
                    class: "secondary",

                    onclick: move |_| {
                        let c = index_code();
                        let p = index_period();

                        let v = index_base()
                            .replace(',', ".")
                            .parse::<Decimal>()
                            .unwrap_or(Decimal::ZERO);

                        async move {
                            match save_lease_index(
                                c,
                                p,
                                v,
                                "Source à renseigner".into(),
                                Some(
                                    chrono::Utc::now()
                                        .date_naive()
                                ),
                            )
                            .await
                            {
                                Ok(_) => bump += 1,
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    },

                    "Enregistrer l'indice"
                }

                div {
                    for i in indices.read().as_deref().unwrap_or(&[]).iter() {
                        div {
                            class: "data-row",

                            div {
                                {format!("{} {} = {}", i.index_code, i.period_label, i.value)}
                            }

                            div {
                                class: "small",
                                "{i.source_reference}"
                            }
                        }
                    }
                }
            }

            section {
                class: "panel",

                h2 { "Charges" }

                div {
                    class: "form-grid",

                    FormField {
                        label: "Type",
                        value: charge_type(),
                        oninput: move |e: FormEvent| charge_type.set(e.value())
                    }

                    FormField {
                        label: "Mode",
                        value: charge_mode(),
                        oninput: move |e: FormEvent| charge_mode.set(e.value())
                    }

                    FormField {
                        label: "Montant €",
                        value: charge_amount(),
                        oninput: move |e: FormEvent| charge_amount.set(e.value())
                    }

                    FormField {
                        label: "Formule variable",
                        value: charge_formula(),
                        oninput: move |e: FormEvent| charge_formula.set(e.value())
                    }
                }

                button {
                    class: "secondary",

                    onclick: move |_| {
                        let a = (
                            charge_amount()
                                .replace(',', ".")
                                .parse::<f64>()
                                .unwrap_or(0.0)
                                * 100.0
                        )
                            .round() as i64;

                        let c = charge_type();
                        let m = charge_mode();
                        let f = charge_formula();

                        async move {
                            match add_lease_charge(
                                id,
                                c,
                                m,
                                a,
                                f,
                                chrono::Utc::now().date_naive(),
                                None,
                            )
                            .await
                            {
                                Ok(_) => bump += 1,
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    },

                    "Ajouter une charge"
                }

                div {
                    for c in charges.read().as_deref().unwrap_or(&[]).iter() {
                        div {
                            class: "data-row",

                            div {
                                {format!("{} / {} / {} €", c.charge_type, c.mode, c.amount_cents as f64 / 100.0)}
                            }

                            div {
                                class: "small",
                                "{c.variable_formula}"
                            }
                        }
                    }
                }
            }

            section {
                class: "panel",

                h2 { "Réductions temporaires" }

                div {
                    class: "form-grid",

                    FormField {
                        label: "Début",
                        value: reduction_start(),
                        oninput: move |e: FormEvent| reduction_start.set(e.value())
                    }

                    FormField {
                        label: "Fin",
                        value: reduction_end(),
                        oninput: move |e: FormEvent| reduction_end.set(e.value())
                    }

                    FormField {
                        label: "Montant €",
                        value: reduction_amount(),
                        oninput: move |e: FormEvent| reduction_amount.set(e.value())
                    }

                    FormField {
                        label: "Pourcentage %",
                        value: reduction_pct(),
                        oninput: move |e: FormEvent| reduction_pct.set(e.value())
                    }

                    FormField {
                        label: "Motif",
                        value: reduction_reason(),
                        oninput: move |e: FormEvent| reduction_reason.set(e.value())
                    }
                }

                button {
                    class: "secondary",

                    onclick: move |_| {
                        let s = NaiveDate::parse_from_str(
                            &reduction_start(),
                            "%Y-%m-%d"
                        );

                        let e = NaiveDate::parse_from_str(
                            &reduction_end(),
                            "%Y-%m-%d"
                        );

                        async move {
                            if let (Ok(s), Ok(e)) = (s, e) {
                                let a =
                                    if reduction_amount().trim().is_empty() {
                                        None
                                    } else {
                                        Some(
                                            (
                                                reduction_amount()
                                                    .replace(',', ".")
                                                    .parse::<f64>()
                                                    .unwrap_or(0.0)
                                                    * 100.0
                                            )
                                                .round()
                                                as i64
                                        )
                                    };

                                let pct =
                                    if reduction_pct().trim().is_empty() {
                                        None
                                    } else {
                                        Some(
                                            (
                                                reduction_pct()
                                                    .replace(',', ".")
                                                    .parse::<f64>()
                                                    .unwrap_or(0.0)
                                                    * 100.0
                                            )
                                                .round()
                                                as i32
                                        )
                                    };

                                match add_lease_reduction(
                                    id,
                                    s,
                                    e,
                                    a,
                                    pct,
                                    reduction_reason(),
                                )
                                .await
                                {
                                    Ok(_) => bump += 1,
                                    Err(e) => msg.set(e.to_string()),
                                }
                            } else {
                                msg.set(
                                    "Périodes invalides".into()
                                );
                            }
                        }
                    },

                    "Enregistrer la réduction"
                }

                div {
                    for r in reductions.read().as_deref().unwrap_or(&[]).iter() {
                        div {
                            class: "data-row",

                            div {
                                {format!("{} → {}", r.start_date, r.end_date)}
                            }

                            div {
                                class: "small",
                                "{r.reason}"
                            }
                        }
                    }
                }
            }

            section {
                class: "panel",

                h2 { "Dépôt de garantie" }

                div {
                    class: "form-grid",

                    FormField {
                        label: "Mouvement",
                        value: deposit_type(),
                        oninput: move |e: FormEvent| deposit_type.set(e.value())
                    }

                    FormField {
                        label: "Montant €",
                        value: deposit_amount(),
                        oninput: move |e: FormEvent| deposit_amount.set(e.value())
                    }

                    FormField {
                        label: "Date",
                        value: deposit_date(),
                        oninput: move |e: FormEvent| deposit_date.set(e.value())
                    }

                    FormField {
                        label: "Justification",
                        value: deposit_justification(),
                        oninput: move |e: FormEvent| deposit_justification.set(e.value())
                    }
                }

                button {
                    class: "secondary",

                    onclick: move |_| {
                        let d = NaiveDate::parse_from_str(
                            &deposit_date(),
                            "%Y-%m-%d"
                        );

                        async move {
                            if let Ok(d) = d {
                                let a = (
                                    deposit_amount()
                                        .replace(',', ".")
                                        .parse::<f64>()
                                        .unwrap_or(0.0)
                                        * 100.0
                                )
                                    .round() as i64;

                                match add_lease_deposit(
                                    id,
                                    deposit_type(),
                                    a,
                                    d,
                                    deposit_justification(),
                                )
                                .await
                                {
                                    Ok(_) => bump += 1,
                                    Err(e) => msg.set(e.to_string()),
                                }
                            } else {
                                msg.set("Date invalide".into());
                            }
                        }
                    },

                    "Enregistrer le mouvement"
                }

                div {
                    for d in deposits.read().as_deref().unwrap_or(&[]).iter() {
                        div {
                            class: "data-row",

                            div {
                                {format!("{} {} €", d.movement_type, d.amount_cents as f64 / 100.0)}
                            }

                            div {
                                class: "small",
                                "{d.justification}"
                            }
                        }
                    }
                }
            }

            section {
                class: "panel",

                h2 { "Garanties" }

                div {
                    class: "form-grid",

                    FormField {
                        label: "Type",
                        value: guarantee_type(),
                        oninput: move |e: FormEvent| guarantee_type.set(e.value())
                    }

                    FormField {
                        label: "Garant",
                        value: guarantor(),
                        oninput: move |e: FormEvent| guarantor.set(e.value())
                    }

                    FormField {
                        label: "Montant €",
                        value: guarantee_amount(),
                        oninput: move |e: FormEvent| guarantee_amount.set(e.value())
                    }

                    FormField {
                        label: "Notes",
                        value: guarantee_notes(),
                        oninput: move |e: FormEvent| guarantee_notes.set(e.value())
                    }
                }

                button {
                    class: "secondary",

                    onclick: move |_| {
                        let a = guarantee_amount()
                            .trim()
                            .parse::<f64>()
                            .ok()
                            .map(|v| {
                                (v * 100.0).round() as i64
                            });

                        async move {
                            match add_lease_guarantee(
                                id,
                                guarantee_type(),
                                guarantor(),
                                a,
                                None,
                                None,
                                None,
                                guarantee_notes(),
                            )
                            .await
                            {
                                Ok(_) => bump += 1,
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    },

                    "Ajouter une garantie"
                }

                div {
                    for g in guarantees.read().as_deref().unwrap_or(&[]).iter() {
                        div {
                            class: "data-row",

                            div {
                                {format!("{} • {}", g.guarantee_type, g.guarantor_name)}
                            }

                            div {
                                class: "small",
                                "{g.notes}"
                            }
                        }
                    }
                }
            }

            section {
                class: "panel",

                h2 { "Droit d'entrée" }

                div {
                    class: "form-grid",

                    FormField {
                        label: "Qualification",
                        value: entry_status(),
                        oninput: move |e: FormEvent| entry_status.set(e.value())
                    }

                    FormField {
                        label: "Justification",
                        value: entry_justification(),
                        oninput: move |e: FormEvent| entry_justification.set(e.value())
                    }
                }

                button {
                    class: "secondary",

                    onclick: move |_| {
                        let st = entry_status();
                        let j = entry_justification();

                        async move {
                            match set_lease_entry_fee(
                                id,
                                0,
                                st,
                                j,
                            )
                            .await
                            {
                                Ok(_) => bump += 1,
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    },

                    "Enregistrer la qualification"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::calculate_indexed_rent;
    use rust_decimal::prelude::ToPrimitive;
    use rust_decimal::Decimal;

    #[test]
    fn indexed_rent_is_reproducible() {
        assert_eq!(
            calculate_indexed_rent(
                100_000,
                Decimal::new(1000, 0),
                Decimal::new(1050, 0),
                None
            )
            .unwrap(),
            105_000
        );
    }

    #[test]
    fn indexed_rent_respects_positive_cap() {
        assert_eq!(
            calculate_indexed_rent(
                100_000,
                Decimal::new(1000, 0),
                Decimal::new(1200, 0),
                Some(500)
            )
            .unwrap(),
            105_000
        );
    }
}