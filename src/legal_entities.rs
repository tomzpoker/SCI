use crate::domain::{LegalEntityBankAccountItem, LegalEntityDraft, LegalEntityItem};
#[cfg(feature = "server")]
use crate::history::{record_entity_change, snapshot_legal_entity};
#[cfg(feature = "server")]
use chrono::Utc;
use dioxus::prelude::*;
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::infrastructure::db;
#[cfg(feature = "server")]
use sqlx::Row;

#[server]
pub async fn list_legal_entities() -> Result<Vec<LegalEntityItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let principal = crate::security::require_permission(pool, "DATA_READ").await.map_err(ServerFnError::new)?;
        let user_id = principal.user_id.ok_or_else(|| ServerFnError::new("Utilisateur courant introuvable"))?;
        let rows = sqlx::query(
            r#"
            SELECT
                e.id,
                e.legal_name,
                e.legal_form_code,
                e.tax_regime,
                e.vat_status,
                e.vat_basis,
                COALESCE(e.siren, '') AS siren,
                COALESCE(e.siret, '') AS siret,
                e.registered_office,
                e.accounting_period_start,
                e.fiscal_year_end,
                e.currency_code,
                e.active,
                COUNT(a.id) FILTER (WHERE a.active) AS bank_accounts_count,
                COALESCE(MAX(a.iban) FILTER (WHERE a.active AND a.is_primary), '') AS primary_iban,
                COALESCE(MAX(a.bic) FILTER (WHERE a.active AND a.is_primary), '') AS primary_bic
            FROM legal_entities e
            JOIN auth_user_roles ur ON ur.legal_entity_id=e.id AND ur.user_id=$1
            LEFT JOIN legal_entity_bank_accounts a ON a.legal_entity_id = e.id
            GROUP BY e.id
            ORDER BY e.active DESC, e.legal_name
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| LegalEntityItem {
                id: r.get("id"),
                legal_name: r.get("legal_name"),
                legal_form_code: r.get("legal_form_code"),
                tax_regime: r.get("tax_regime"),
                vat_status: r.get("vat_status"),
                vat_basis: r.get("vat_basis"),
                siren: r.get("siren"),
                siret: r.get("siret"),
                registered_office: r.get("registered_office"),
                accounting_period_start: r.get::<i16, _>("accounting_period_start") as u8,
                fiscal_year_end: r.get::<i16, _>("fiscal_year_end") as u8,
                currency_code: r.get("currency_code"),
                active: r.get("active"),
                bank_accounts_count: r.get("bank_accounts_count"),
                primary_iban: r.get("primary_iban"),
                primary_bic: r.get("primary_bic"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_legal_entities est exécutée côté serveur"))
}

#[server]
pub async fn create_legal_entity(draft: LegalEntityDraft) -> Result<Uuid, ServerFnError> {
    #[cfg(feature = "server")]
    {
        validate_draft(&draft).map_err(ServerFnError::new)?;
        let pool = db().await.map_err(ServerFnError::new)?;
        let principal = crate::security::require_permission(pool, "SECURITY_MANAGE").await.map_err(ServerFnError::new)?;
        let user_id = principal.user_id.ok_or_else(|| ServerFnError::new("Utilisateur courant introuvable"))?;
        let workspace_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001")
            .map_err(ServerFnError::new)?;
        let mut tx = pool.begin().await.map_err(ServerFnError::new)?;
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO legal_entities (
                id, workspace_id, legal_name, legal_form_code, tax_regime,
                vat_status, vat_basis, siren, siret, registered_office,
                accounting_period_start, fiscal_year_end, currency_code
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,NULLIF($8,''),NULLIF($9,''),$10,$11,$12,$13
            )
            "#,
        )
        .bind(id)
        .bind(workspace_id)
        .bind(draft.legal_name.trim())
        .bind(draft.legal_form_code.trim().to_uppercase())
        .bind(draft.tax_regime.trim().to_uppercase())
        .bind(draft.vat_status.trim().to_uppercase())
        .bind(draft.vat_basis.trim().to_uppercase())
        .bind(draft.siren.trim())
        .bind(draft.siret.trim())
        .bind(draft.registered_office.trim())
        .bind(i16::from(draft.accounting_period_start))
        .bind(i16::from(draft.fiscal_year_end))
        .bind(draft.currency_code.trim().to_uppercase())
        .execute(&mut *tx)
        .await
        .map_err(ServerFnError::new)?;

        if !draft.primary_iban.trim().is_empty() || !draft.primary_bic.trim().is_empty() {
            sqlx::query(
                "INSERT INTO legal_entity_bank_accounts(legal_entity_id,label,iban,bic,is_primary,active) VALUES($1,'Compte principal',$2,$3,true,true)",
            )
            .bind(id)
            .bind(draft.primary_iban.trim())
            .bind(draft.primary_bic.trim())
            .execute(&mut *tx)
            .await
            .map_err(ServerFnError::new)?;
        }

        sqlx::query("INSERT INTO auth_user_roles(user_id,legal_entity_id,role) VALUES($1,$2,'OWNER')")
            .bind(user_id)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(ServerFnError::new)?;

        sqlx::query(
            "INSERT INTO audit_events(sci_id,legal_entity_id,actor,action,entity_type,entity_id,payload,actor_user_id) VALUES(NULL,$1,'USER','CREATE_LEGAL_ENTITY','LEGAL_ENTITY',$1,$2,$3)",
        )
        .bind(id)
        .bind(serde_json::json!({
            "legal_name": draft.legal_name.trim(),
            "legal_form_code": draft.legal_form_code.trim().to_uppercase(),
            "tax_regime": draft.tax_regime.trim().to_uppercase()
        }))
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(ServerFnError::new)?;

        tx.commit().await.map_err(ServerFnError::new)?;
        let after = snapshot_legal_entity(pool, id).await.map_err(ServerFnError::new)?;
        record_entity_change(pool, id, "CREATE_LEGAL_ENTITY", "LEGAL_ENTITY", id, serde_json::json!({}), after, Some("Création de l’entité juridique"), Utc::now(), serde_json::json!({"source":"Entités"})).await.map_err(ServerFnError::new)?;
        Ok(id)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("create_legal_entity est exécutée côté serveur"))
}

#[server]
pub async fn update_legal_entity(id: Uuid, draft: LegalEntityDraft) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        validate_draft(&draft).map_err(ServerFnError::new)?;
        let pool = db().await.map_err(ServerFnError::new)?;
        let principal = crate::security::require_permission(pool, "SECURITY_MANAGE").await.map_err(ServerFnError::new)?;
        let user_id = principal.user_id.ok_or_else(|| ServerFnError::new("Utilisateur courant introuvable"))?;
        let allowed_on_target: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_user_roles ur JOIN security_role_permissions rp ON rp.role=ur.role AND rp.permission_code='SECURITY_MANAGE' AND rp.allowed WHERE ur.user_id=$1 AND ur.legal_entity_id=$2)")
            .bind(user_id)
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(ServerFnError::new)?;
        if !allowed_on_target { return Err(ServerFnError::new("Accès refusé à cette entité juridique")); }
        let before = snapshot_legal_entity(pool, id).await.map_err(ServerFnError::new)?;
        sqlx::query(
            r#"
            UPDATE legal_entities
            SET legal_name=$2, legal_form_code=$3, tax_regime=$4,
                vat_status=$5, vat_basis=$6, siren=NULLIF($7,''), siret=NULLIF($8,''),
                registered_office=$9, accounting_period_start=$10, fiscal_year_end=$11,
                currency_code=$12, updated_at=now()
            WHERE id=$1
            "#,
        )
        .bind(id)
        .bind(draft.legal_name.trim())
        .bind(draft.legal_form_code.trim().to_uppercase())
        .bind(draft.tax_regime.trim().to_uppercase())
        .bind(draft.vat_status.trim().to_uppercase())
        .bind(draft.vat_basis.trim().to_uppercase())
        .bind(draft.siren.trim())
        .bind(draft.siret.trim())
        .bind(draft.registered_office.trim())
        .bind(i16::from(draft.accounting_period_start))
        .bind(i16::from(draft.fiscal_year_end))
        .bind(draft.currency_code.trim().to_uppercase())
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM legal_entities WHERE id=$1)")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(ServerFnError::new)?;
        if !exists {
            return Err(ServerFnError::new("Entité juridique introuvable"));
        }

        sqlx::query(
            "UPDATE legal_entity_bank_accounts SET iban=$2,bic=$3,updated_at=now() WHERE legal_entity_id=$1 AND is_primary AND active",
        )
        .bind(id)
        .bind(draft.primary_iban.trim())
        .bind(draft.primary_bic.trim())
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        let has_primary: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM legal_entity_bank_accounts WHERE legal_entity_id=$1 AND is_primary AND active)",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;

        if !has_primary && (!draft.primary_iban.trim().is_empty() || !draft.primary_bic.trim().is_empty()) {
            sqlx::query("INSERT INTO legal_entity_bank_accounts(legal_entity_id,label,iban,bic,is_primary,active) VALUES($1,'Compte principal',$2,$3,true,true)")
                .bind(id)
                .bind(draft.primary_iban.trim())
                .bind(draft.primary_bic.trim())
                .execute(pool)
                .await
                .map_err(ServerFnError::new)?;
        }

        audit_entity(pool, id, "UPDATE_LEGAL_ENTITY", serde_json::json!({"legal_name":draft.legal_name.trim()}))
            .await
            .map_err(ServerFnError::new)?;
        let after = snapshot_legal_entity(pool, id).await.map_err(ServerFnError::new)?;
        record_entity_change(pool, id, "UPDATE_LEGAL_ENTITY", "LEGAL_ENTITY", id, before, after, None, Utc::now(), serde_json::json!({"source":"Entités"})).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("update_legal_entity est exécutée côté serveur"))
}

#[server]
pub async fn set_legal_entity_active(id: Uuid, active: bool) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let principal = crate::security::require_permission(pool, "SECURITY_MANAGE").await.map_err(ServerFnError::new)?;
        let user_id = principal.user_id.ok_or_else(|| ServerFnError::new("Utilisateur courant introuvable"))?;
        let allowed_on_target: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_user_roles ur JOIN security_role_permissions rp ON rp.role=ur.role AND rp.permission_code='SECURITY_MANAGE' AND rp.allowed WHERE ur.user_id=$1 AND ur.legal_entity_id=$2)")
            .bind(user_id)
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(ServerFnError::new)?;
        if !allowed_on_target { return Err(ServerFnError::new("Accès refusé à cette entité juridique")); }
        if !active {
            let linked_to_sci: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM scis WHERE legal_entity_id=$1)",
            )
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(ServerFnError::new)?;
            if linked_to_sci {
                return Err(ServerFnError::new(
                    "Cette entité est liée au socle SCI courant et ne peut pas être désactivée avant le cloisonnement opérationnel US-0203.",
                ));
            }
        }
        let before = snapshot_legal_entity(pool, id).await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE legal_entities SET active=$2,updated_at=now() WHERE id=$1")
            .bind(id)
            .bind(active)
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        audit_entity(
            pool,
            id,
            if active { "ACTIVATE_LEGAL_ENTITY" } else { "DEACTIVATE_LEGAL_ENTITY" },
            serde_json::json!({"active":active}),
        )
        .await
        .map_err(ServerFnError::new)?;
        let after = snapshot_legal_entity(pool, id).await.map_err(ServerFnError::new)?;
        record_entity_change(pool, id, if active { "ACTIVATE_LEGAL_ENTITY" } else { "DEACTIVATE_LEGAL_ENTITY" }, "LEGAL_ENTITY", id, before, after, None, Utc::now(), serde_json::json!({"source":"Entités"})).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("set_legal_entity_active est exécutée côté serveur"))
}

#[server]
pub async fn list_legal_entity_bank_accounts(
    legal_entity_id: Uuid,
) -> Result<Vec<LegalEntityBankAccountItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let principal = crate::security::require_permission(pool, "DATA_READ").await.map_err(ServerFnError::new)?;
        let user_id = principal.user_id.ok_or_else(|| ServerFnError::new("Utilisateur courant introuvable"))?;
        let allowed_on_target: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_user_roles ur JOIN legal_entities le ON le.id=ur.legal_entity_id AND le.active=true WHERE ur.user_id=$1 AND ur.legal_entity_id=$2)")
            .bind(user_id)
            .bind(legal_entity_id)
            .fetch_one(pool)
            .await
            .map_err(ServerFnError::new)?;
        if !allowed_on_target { return Err(ServerFnError::new("Accès refusé à cette entité juridique")); }
        let rows = sqlx::query("SELECT id,legal_entity_id,label,iban,bic,is_primary,active FROM legal_entity_bank_accounts WHERE legal_entity_id=$1 ORDER BY active DESC,is_primary DESC,label")
            .bind(legal_entity_id)
            .fetch_all(pool)
            .await
            .map_err(ServerFnError::new)?;
        Ok(rows.into_iter().map(|r| LegalEntityBankAccountItem {
            id: r.get("id"),
            legal_entity_id: r.get("legal_entity_id"),
            label: r.get("label"),
            iban: r.get("iban"),
            bic: r.get("bic"),
            is_primary: r.get("is_primary"),
            active: r.get("active"),
        }).collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_legal_entity_bank_accounts est exécutée côté serveur"))
}

fn validate_draft(draft: &LegalEntityDraft) -> Result<(), String> {
    if draft.legal_name.trim().is_empty() {
        return Err("Dénomination sociale requise".into());
    }
    let form = draft.legal_form_code.trim().to_uppercase();
    if !matches!(form.as_str(), "SCI" | "SARL") {
        return Err("Forme juridique prise en charge : SCI ou SARL".into());
    }
    let tax = draft.tax_regime.trim().to_uppercase();
    if !matches!(tax.as_str(), "IR" | "IS") {
        return Err("Régime fiscal pris en charge : IR ou IS".into());
    }
    if !(1..=12).contains(&draft.accounting_period_start) || !(1..=12).contains(&draft.fiscal_year_end) {
        return Err("Mois d’exercice invalide".into());
    }
    if draft.currency_code.trim().len() != 3 {
        return Err("Code devise ISO de 3 caractères requis".into());
    }
    validate_identifier(&draft.siren, 9, "SIREN")?;
    validate_identifier(&draft.siret, 14, "SIRET")?;
    Ok(())
}

fn validate_identifier(value: &str, expected: usize, label: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(());
    }
    if value.len() != expected || !value.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("{} invalide : {} chiffres attendus", label, expected));
    }
    Ok(())
}

#[cfg(feature = "server")]
async fn audit_entity(
    pool: &sqlx::PgPool,
    id: Uuid,
    action: &str,
    payload: serde_json::Value,
) -> Result<(), sqlx::Error> {
    let principal = crate::security::current_principal(pool).await?;
    let actor = principal.role.as_deref().unwrap_or("USER");
    sqlx::query("INSERT INTO audit_events(sci_id,legal_entity_id,actor,action,entity_type,entity_id,payload,actor_user_id,source) VALUES(NULL,$1,$2,$3,'LEGAL_ENTITY',$1,$4,$5,'APP')")
        .bind(id)
        .bind(actor)
        .bind(action)
        .bind(payload)
        .bind(principal.user_id)
        .execute(pool)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> LegalEntityDraft {
        LegalEntityDraft {
            legal_name: "Test".into(),
            legal_form_code: "SCI".into(),
            tax_regime: "IR".into(),
            vat_status: "OPTION_LOYERS".into(),
            vat_basis: "COLLECTION".into(),
            siren: "123456789".into(),
            siret: "12345678901234".into(),
            registered_office: "France".into(),
            accounting_period_start: 1,
            fiscal_year_end: 12,
            currency_code: "EUR".into(),
            primary_iban: "".into(),
            primary_bic: "".into(),
        }
    }

    #[test]
    fn accepts_sci_ir() {
        assert!(validate_draft(&draft()).is_ok());
    }

    #[test]
    fn accepts_sarl_is() {
        let mut d = draft();
        d.legal_form_code = "SARL".into();
        d.tax_regime = "IS".into();
        assert!(validate_draft(&d).is_ok());
    }

    #[test]
    fn rejects_bad_siren() {
        let mut d = draft();
        d.siren = "12".into();
        assert!(validate_draft(&d).is_err());
    }
}
