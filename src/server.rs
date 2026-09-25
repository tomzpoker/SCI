use crate::domain::*;
use crate::entity_scope::current_legal_entity_id;
#[cfg(feature = "server")]
use crate::engines::load_engine_context;
#[cfg(feature = "server")]
use crate::idempotency::{claim_job, complete_job, fail_job};
#[cfg(feature = "server")]
use crate::services::{record_business_event, record_financial_transaction};
#[cfg(feature = "server")]
use crate::history::{record_entity_change, snapshot_legal_entity};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use dioxus::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::infrastructure::db;
#[cfg(feature = "server")]
use sqlx::Row;

#[server]
pub async fn dashboard_snapshot() -> Result<DashboardSnapshot, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let context = load_engine_context(pool, current_legal_entity_id()).await.map_err(ServerFnError::new)?;
        let id = context.legal_entity_id;
        let profile = fetch_legal_entity_header(pool, id).await.map_err(ServerFnError::new)?;
        let receivables: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(i.gross_cents - COALESCE(p.paid,0)),0)::bigint FROM invoices i LEFT JOIN (SELECT invoice_id, SUM(amount_cents) paid FROM payments GROUP BY invoice_id) p ON p.invoice_id=i.id WHERE i.legal_entity_id=$1 AND i.status IN ('ISSUED','PAID_PARTIAL','OVERDUE')")
            .bind(id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let cash: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_cents),0)::bigint FROM bank_transactions WHERE legal_entity_id=$1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;
        let vat: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(ROUND(p.amount_cents::numeric * i.vat_cents / NULLIF(i.gross_cents,0))),0)::bigint FROM payments p JOIN invoices i ON i.id=p.invoice_id WHERE p.legal_entity_id=$1 AND p.received_at >= date_trunc('month',now())").bind(id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let due30: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM tasks WHERE legal_entity_id=$1 AND state NOT IN ('DONE','SKIPPED') AND due_at < now()+interval '30 days'").bind(id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let overdue: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM tasks WHERE legal_entity_id=$1 AND state NOT IN ('DONE','SKIPPED') AND due_at < now()").bind(id).fetch_one(pool).await.map_err(ServerFnError::new)?;
        let next_actions = list_tasks_inner(pool, id, 8)
            .await
            .map_err(ServerFnError::new)?;
        let forecast = build_forecast(pool, id, cash)
            .await
            .map_err(ServerFnError::new)?;
        let min_cash = forecast
            .iter()
            .map(|x| x.balance_cents)
            .min()
            .unwrap_or(cash);
        let risk_level = if min_cash < 0 || overdue > 0 {
            "CRITIQUE"
        } else if due30 > 5 || vat > cash.max(1) {
            "VIGILANCE"
        } else {
            "NORMAL"
        }
        .to_string();
        Ok(DashboardSnapshot {
            sci_name: profile.legal_name,
            registered_office: profile.registered_office,
            tax_regime: profile.tax_regime,
            vat_basis: profile.vat_basis,
            cash_cents: cash,
            receivables_cents: receivables,
            vat_to_prepare_cents: vat,
            tasks_due_30d: due30,
            overdue_tasks: overdue,
            forecast_min_cash_cents: min_cash,
            risk_level,
            next_actions,
            forecast,
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "dashboard_snapshot est exécutée côté serveur",
    ))
}

#[server]
pub async fn module_counts() -> Result<ModuleCounts, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        fetch_counts(pool, current_legal_entity_id())
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "module_counts est exécutée côté serveur",
    ))
}

#[server]
pub async fn get_sci_profile() -> Result<SciProfile, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        fetch_profile(pool, current_legal_entity_id())
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "get_sci_profile est exécutée côté serveur",
    ))
}

#[server]
pub async fn update_sci_profile(profile: SciProfile) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let before = snapshot_legal_entity(pool, current_legal_entity_id()).await.map_err(ServerFnError::new)?;
        validate_profile(&profile).map_err(ServerFnError::new)?;
        let sci_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM scis WHERE legal_entity_id=$1)")
            .bind(current_legal_entity_id())
            .fetch_one(pool)
            .await
            .map_err(ServerFnError::new)?;
        if !sci_exists {
            return Err(ServerFnError::new("La fiche SCI ne peut pas modifier une SARL ; utilisez Entités"));
        }
        let mut tx = pool.begin().await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE scis SET legal_name=$2,siren=NULLIF($3,''),siret=NULLIF($4,''),registered_office=NULLIF($5,''),tax_regime='IR',vat_status='OPTION_LOYERS',vat_basis='COLLECTION',accounting_period_start=$6,fiscal_year_end=$7,iban=NULLIF($8,''),bic=NULLIF($9,''),updated_at=now() WHERE legal_entity_id=$1")
            .bind(current_legal_entity_id()).bind(profile.legal_name.trim()).bind(profile.siren.trim()).bind(profile.siret.trim()).bind(profile.registered_office.trim()).bind(i16::from(profile.accounting_period_start)).bind(i16::from(profile.fiscal_year_end)).bind(profile.iban.trim()).bind(profile.bic.trim()).execute(&mut *tx).await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE legal_entities SET legal_name=$2,siren=NULLIF($3,''),siret=NULLIF($4,''),registered_office=$5,tax_regime='IR',vat_status='OPTION_LOYERS',vat_basis='COLLECTION',accounting_period_start=$6,fiscal_year_end=$7,updated_at=now() WHERE id=$1")
            .bind(current_legal_entity_id()).bind(profile.legal_name.trim()).bind(profile.siren.trim()).bind(profile.siret.trim()).bind(profile.registered_office.trim()).bind(i16::from(profile.accounting_period_start)).bind(i16::from(profile.fiscal_year_end)).execute(&mut *tx).await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE legal_entity_bank_accounts SET iban=$2,bic=$3,updated_at=now() WHERE legal_entity_id=$1 AND is_primary=true AND active=true")
            .bind(current_legal_entity_id()).bind(profile.iban.trim()).bind(profile.bic.trim()).execute(&mut *tx).await.map_err(ServerFnError::new)?;
        sqlx::query("INSERT INTO legal_entity_bank_accounts(legal_entity_id,label,iban,bic,is_primary,active) SELECT $1,'Compte principal',$2,$3,true,true WHERE NOT EXISTS(SELECT 1 FROM legal_entity_bank_accounts WHERE legal_entity_id=$1 AND is_primary=true AND active=true)")
            .bind(current_legal_entity_id()).bind(profile.iban.trim()).bind(profile.bic.trim()).execute(&mut *tx).await.map_err(ServerFnError::new)?;
        tx.commit().await.map_err(ServerFnError::new)?;
        audit(
            pool,
            "UPDATE_SCI_PROFILE",
            None,
            None,
            serde_json::json!({"legal_name":profile.legal_name}),
        )
        .await
        .map_err(ServerFnError::new)?;
        let after = snapshot_legal_entity(pool, current_legal_entity_id()).await.map_err(ServerFnError::new)?;
        record_entity_change(pool, current_legal_entity_id(), "UPDATE_SCI_PROFILE", "LEGAL_ENTITY", current_legal_entity_id(), before, after, None, Utc::now(), serde_json::json!({"source":"Configuration SCI"})).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "update_sci_profile est exécutée côté serveur",
    ))
}

#[server]
pub async fn onboarding_status() -> Result<OnboardingStatus, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let id = current_legal_entity_id();
        let entity = fetch_legal_entity_header(pool, id).await.map_err(ServerFnError::new)?;
        let c = fetch_counts(pool, id).await.map_err(ServerFnError::new)?;
        let profile_ready = !entity.legal_name.trim().is_empty()
            && !entity.registered_office.trim().is_empty()
            && !entity.legal_name.eq_ignore_ascii_case("SCI À CONFIGURER")
            && !entity.registered_office.eq_ignore_ascii_case("À configurer");
        let checks = if entity.legal_form_code == "SCI" {
            [
                profile_ready,
                c.associates > 0,
                c.properties > 0 && c.units > 0,
                c.tenants > 0,
                c.leases > 0,
                c.bank_transactions > 0 || c.invoices > 0,
                c.active_automation_rules >= 4,
            ]
        } else {
            // Les blocs patrimoine/baux sont spécifiques au vertical slice SCI.
            [
                profile_ready,
                true,
                true,
                true,
                true,
                c.bank_transactions > 0 || c.invoices > 0,
                c.active_automation_rules > 0,
            ]
        };
        let pct = ((checks.iter().filter(|x| **x).count() * 100) / checks.len()) as u8;
        let completed = checks.iter().all(|x| *x);
        if completed && entity.legal_form_code == "SCI" {
            sqlx::query("UPDATE scis SET onboarding_completed_at=COALESCE(onboarding_completed_at,now()) WHERE legal_entity_id=$1").bind(id).execute(pool).await.map_err(ServerFnError::new)?;
        }
        Ok(OnboardingStatus {
            profile_ready: checks[0],
            associates_ready: checks[1],
            property_ready: checks[2],
            tenant_ready: checks[3],
            lease_ready: checks[4],
            finance_ready: checks[5],
            automation_ready: checks[6],
            completed,
            completion_pct: pct,
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "onboarding_status est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_associates() -> Result<Vec<AssociateItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT id,display_name,ownership_pct,current_account_cents,active FROM associates WHERE legal_entity_id=$1 ORDER BY active DESC,display_name").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| AssociateItem {
                id: r.get("id"),
                display_name: r.get("display_name"),
                ownership_pct: r.get("ownership_pct"),
                current_account_cents: r.get("current_account_cents"),
                active: r.get("active"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_associates est exécutée côté serveur",
    ))
}

#[server]
pub async fn create_associate(
    display_name: String,
    ownership_pct: Decimal,
    current_account_cents: i64,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if display_name.trim().is_empty() {
            return Err(ServerFnError::new("Nom requis"));
        }
        if ownership_pct <= Decimal::ZERO || ownership_pct > Decimal::from(100u32) {
            return Err(ServerFnError::new("Quote-part invalide"));
        }
        let current: Decimal = sqlx::query_scalar(
            "SELECT COALESCE(SUM(ownership_pct),0) FROM associates WHERE legal_entity_id=$1 AND active=true",
        )
        .bind(current_legal_entity_id())
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;
        if current + ownership_pct > Decimal::from(100u32) {
            return Err(ServerFnError::new(
                "Le total des quote-parts dépasserait 100 %",
            ));
        }
        let sci = sqlx::query_scalar::<_,Uuid>("SELECT id FROM scis WHERE legal_entity_id=$1").bind(current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Les associés ne sont disponibles que pour une SCI"))?;
        sqlx::query("INSERT INTO associates(sci_id,legal_entity_id,display_name,ownership_pct,current_account_cents) VALUES($1,$2,$3,$4,$5)").bind(sci).bind(current_legal_entity_id()).bind(display_name.trim()).bind(ownership_pct).bind(current_account_cents).execute(pool).await.map_err(ServerFnError::new)?;
        audit(
            pool,
            "CREATE_ASSOCIATE",
            None,
            None,
            serde_json::json!({"name":display_name}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "create_associate est exécutée côté serveur",
    ))
}

#[server]
pub async fn delete_associate(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let r = sqlx::query("DELETE FROM associates WHERE id=$1 AND legal_entity_id=$2")
            .bind(id)
            .bind(current_legal_entity_id())
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        if r.rows_affected() == 0 {
            return Err(ServerFnError::new("Associé introuvable"));
        }
        audit(
            pool,
            "DELETE_ASSOCIATE",
            Some("associate"),
            Some(id),
            serde_json::json!({}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "delete_associate est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_properties() -> Result<Vec<PropertyItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT p.id,p.name,p.address,p.acquisition_date,p.acquisition_cents,p.active,COUNT(u.id)::bigint units_count FROM properties p LEFT JOIN units u ON u.property_id=p.id WHERE p.legal_entity_id=$1 GROUP BY p.id ORDER BY p.active DESC,p.name").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| PropertyItem {
                id: r.get("id"),
                name: r.get("name"),
                address: r.get("address"),
                acquisition_date: r.get("acquisition_date"),
                acquisition_cents: r.get("acquisition_cents"),
                units_count: r.get("units_count"),
                active: r.get("active"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_properties est exécutée côté serveur",
    ))
}

#[server]
pub async fn create_property(
    name: String,
    address: String,
    acquisition_date: Option<NaiveDate>,
    acquisition_cents: Option<i64>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if name.trim().is_empty() || address.trim().is_empty() {
            return Err(ServerFnError::new("Nom et adresse requis"));
        }
        if acquisition_cents.unwrap_or(0) < 0 {
            return Err(ServerFnError::new("Prix d'acquisition invalide"));
        }
        let sci = sqlx::query_scalar::<_,Uuid>("SELECT id FROM scis WHERE legal_entity_id=$1").bind(current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Le patrimoine de base est actuellement réservé aux SCI"))?;
        sqlx::query("INSERT INTO properties(sci_id,legal_entity_id,name,address,acquisition_date,acquisition_cents) VALUES($1,$2,$3,$4,$5,$6)").bind(sci).bind(current_legal_entity_id()).bind(name.trim()).bind(address.trim()).bind(acquisition_date).bind(acquisition_cents).execute(pool).await.map_err(ServerFnError::new)?;
        audit(
            pool,
            "CREATE_PROPERTY",
            None,
            None,
            serde_json::json!({"name":name}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "create_property est exécutée côté serveur",
    ))
}

#[server]
pub async fn delete_property(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if sqlx::query_scalar::<_,i64>("SELECT COUNT(*)::bigint FROM leases l JOIN units u ON u.id=l.unit_id WHERE u.property_id=$1 AND u.legal_entity_id=$2 AND l.legal_entity_id=$2").bind(id).bind(current_legal_entity_id()).fetch_one(pool).await.map_err(ServerFnError::new)?>0{return Err(ServerFnError::new("Impossible de supprimer un bien ayant un bail"));}
        let r = sqlx::query("DELETE FROM properties WHERE id=$1 AND legal_entity_id=$2")
            .bind(id)
            .bind(current_legal_entity_id())
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        if r.rows_affected() == 0 {
            return Err(ServerFnError::new("Bien introuvable"));
        }
        audit(
            pool,
            "DELETE_PROPERTY",
            Some("property"),
            Some(id),
            serde_json::json!({}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "delete_property est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_units() -> Result<Vec<UnitItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT u.id,u.property_id,p.name property_name,u.code,u.label,u.unit_type,u.area_m2,u.base_rent_cents,u.vat_rate_bp,u.active FROM units u JOIN properties p ON p.id=u.property_id WHERE p.legal_entity_id=$1 ORDER BY p.name,u.code").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| UnitItem {
                id: r.get("id"),
                property_id: r.get("property_id"),
                property_name: r.get("property_name"),
                code: r.get("code"),
                label: r.get("label"),
                unit_type: r.get("unit_type"),
                area_m2: r.get("area_m2"),
                base_rent_cents: r.get("base_rent_cents"),
                vat_rate_bp: r.get("vat_rate_bp"),
                active: r.get("active"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_units est exécutée côté serveur",
    ))
}

#[server]
pub async fn create_unit(
    property_id: Uuid,
    code: String,
    label: String,
    unit_type: String,
    area_m2: Option<Decimal>,
    base_rent_cents: i64,
    vat_rate_bp: i32,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if code.trim().is_empty() || label.trim().is_empty() {
            return Err(ServerFnError::new("Code et libellé requis"));
        }
        if base_rent_cents < 0 || vat_rate_bp < 0 || vat_rate_bp > 10000 {
            return Err(ServerFnError::new("Montant ou TVA invalide"));
        }
        let r=sqlx::query("INSERT INTO units(property_id,legal_entity_id,code,label,unit_type,area_m2,base_rent_cents,vat_rate_bp) SELECT $1,$8,$2,$3,$4,$5,$6,$7 WHERE EXISTS(SELECT 1 FROM properties WHERE id=$1 AND legal_entity_id=$8)").bind(property_id).bind(code.trim()).bind(label.trim()).bind(unit_type.trim()).bind(area_m2).bind(base_rent_cents).bind(vat_rate_bp).bind(current_legal_entity_id()).execute(pool).await.map_err(ServerFnError::new)?;
        if r.rows_affected() == 0 {
            return Err(ServerFnError::new("Bien invalide"));
        }
        audit(
            pool,
            "CREATE_UNIT",
            Some("unit"),
            None,
            serde_json::json!({"code":code}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "create_unit est exécutée côté serveur",
    ))
}

#[server]
pub async fn delete_unit(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM leases WHERE unit_id=$1 AND legal_entity_id=$2")
            .bind(id)
            .bind(current_legal_entity_id())
            .fetch_one(pool)
            .await
            .map_err(ServerFnError::new)?
            > 0
        {
            return Err(ServerFnError::new(
                "Impossible de supprimer un lot ayant un bail",
            ));
        }
        sqlx::query("DELETE FROM units u USING properties p WHERE u.id=$1 AND u.property_id=p.id AND p.legal_entity_id=$2").bind(id).bind(current_legal_entity_id()).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "delete_unit est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_tenants() -> Result<Vec<TenantItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT id,legal_name,COALESCE(siret,'') AS siret,COALESCE(contact_email,'') AS contact_email,COALESCE(contact_phone,'') AS contact_phone,active FROM tenants WHERE legal_entity_id=$1 ORDER BY active DESC,legal_name").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| TenantItem {
                id: r.get("id"),
                legal_name: r.get("legal_name"),
                siret: r.get("siret"),
                contact_email: r.get("contact_email"),
                contact_phone: r.get("contact_phone"),
                active: r.get("active"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_tenants est exécutée côté serveur",
    ))
}

#[server]
pub async fn create_tenant(
    legal_name: String,
    siret: String,
    contact_email: String,
    contact_phone: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if legal_name.trim().is_empty() {
            return Err(ServerFnError::new("Nom requis"));
        }
        let sci = sqlx::query_scalar::<_,Uuid>("SELECT id FROM scis WHERE legal_entity_id=$1").bind(current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Les locataires de baux sont actuellement réservés aux SCI"))?;
        sqlx::query("INSERT INTO tenants(sci_id,legal_entity_id,legal_name,siret,contact_email,contact_phone) VALUES($1,$2,$3,NULLIF($4,''),NULLIF($5,''),NULLIF($6,''))").bind(sci).bind(current_legal_entity_id()).bind(legal_name.trim()).bind(siret.trim()).bind(contact_email.trim()).bind(contact_phone.trim()).execute(pool).await.map_err(ServerFnError::new)?;
        audit(
            pool,
            "CREATE_TENANT",
            None,
            None,
            serde_json::json!({"name":legal_name}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "create_tenant est exécutée côté serveur",
    ))
}

#[server]
pub async fn delete_tenant(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM leases WHERE tenant_id=$1 AND legal_entity_id=$2")
            .bind(id)
            .bind(current_legal_entity_id())
            .fetch_one(pool)
            .await
            .map_err(ServerFnError::new)?
            > 0
        {
            return Err(ServerFnError::new(
                "Impossible de supprimer un locataire ayant un bail",
            ));
        }
        sqlx::query("DELETE FROM tenants WHERE id=$1 AND legal_entity_id=$2")
            .bind(id)
            .bind(current_legal_entity_id())
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "delete_tenant est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_leases() -> Result<Vec<LeaseItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT l.id,l.unit_id,u.label unit_label,p.name property_name,l.tenant_id,t.legal_name tenant_name,l.reference,l.start_date,l.end_date,l.payment_day,l.annual_review_month,l.active FROM leases l JOIN units u ON u.id=l.unit_id JOIN properties p ON p.id=u.property_id JOIN tenants t ON t.id=l.tenant_id WHERE p.legal_entity_id=$1 ORDER BY l.active DESC,l.start_date DESC").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| LeaseItem {
                id: r.get("id"),
                unit_id: r.get("unit_id"),
                unit_label: r.get("unit_label"),
                property_name: r.get("property_name"),
                tenant_id: r.get("tenant_id"),
                tenant_name: r.get("tenant_name"),
                reference: r.get("reference"),
                start_date: r.get("start_date"),
                end_date: r.get("end_date"),
                payment_day: r.get("payment_day"),
                annual_review_month: r.get("annual_review_month"),
                active: r.get("active"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_leases est exécutée côté serveur",
    ))
}

#[server]
pub async fn create_lease(
    unit_id: Uuid,
    tenant_id: Uuid,
    reference: String,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    notice_months: i32,
    payment_day: i16,
    annual_review_month: Option<i16>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if reference.trim().is_empty() {
            return Err(ServerFnError::new("Référence requise"));
        }
        if !(1..=31).contains(&payment_day) {
            return Err(ServerFnError::new("Jour de paiement invalide"));
        }
        if notice_months < 0 || notice_months > 24 {
            return Err(ServerFnError::new("Préavis invalide"));
        }
        if let Some(d) = end_date {
            if d < start_date {
                return Err(ServerFnError::new("Fin de bail avant le début"));
            }
        }
        let valid:i64=sqlx::query_scalar("SELECT COUNT(*)::bigint FROM units u JOIN properties p ON p.id=u.property_id WHERE u.id=$1 AND p.legal_entity_id=$3 AND EXISTS(SELECT 1 FROM tenants WHERE id=$2 AND legal_entity_id=$3)").bind(unit_id).bind(tenant_id).bind(current_legal_entity_id()).fetch_one(pool).await.map_err(ServerFnError::new)?;
        if valid != 1 {
            return Err(ServerFnError::new("Lot ou locataire invalide"));
        }
        let entity = current_legal_entity_id();
        let lease_id: Uuid = sqlx::query_scalar("INSERT INTO leases(legal_entity_id,unit_id,tenant_id,reference,start_date,end_date,notice_months,payment_day,annual_review_month,active) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,true) RETURNING id").bind(entity).bind(unit_id).bind(tenant_id).bind(reference.trim()).bind(start_date).bind(end_date).bind(notice_months).bind(payment_day).bind(annual_review_month).fetch_one(pool).await.map_err(ServerFnError::new)?;
        record_business_event(pool, entity, "LeaseCreated", Utc::now(), "lease", Some(lease_id), &format!("lease-created:{lease_id}"), serde_json::json!({"unit_id":unit_id,"tenant_id":tenant_id,"reference":reference.trim()})).await.map_err(ServerFnError::new)?;
        audit(
            pool,
            "CREATE_LEASE",
            None,
            None,
            serde_json::json!({"reference":reference}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "create_lease est exécutée côté serveur",
    ))
}

#[server]
pub async fn delete_lease(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        sqlx::query("DELETE FROM leases l USING units u JOIN properties p ON p.id=u.property_id WHERE l.id=$1 AND l.unit_id=u.id AND p.legal_entity_id=$2").bind(id).bind(current_legal_entity_id()).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "delete_lease est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_invoices() -> Result<Vec<InvoiceItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT i.id,i.lease_id,COALESCE(i.invoice_number,'BROUILLON') invoice_number,i.issue_date,i.due_date,i.net_cents,i.vat_cents,i.gross_cents,COALESCE(SUM(p.amount_cents),0)::bigint paid_cents,CASE WHEN i.status='DRAFT' THEN 'BROUILLON' WHEN COALESCE(SUM(p.amount_cents),0)>=i.gross_cents THEN 'PAYÉE' WHEN COALESCE(SUM(p.amount_cents),0)>0 THEN 'PARTIELLE' WHEN i.due_date<CURRENT_DATE THEN 'EN RETARD' ELSE 'ÉMISE' END status,COALESCE(t.legal_name,'Sans locataire') tenant_name FROM invoices i LEFT JOIN leases l ON l.id=i.lease_id LEFT JOIN units u ON u.id=l.unit_id LEFT JOIN properties pr ON pr.id=u.property_id LEFT JOIN tenants t ON t.id=l.tenant_id LEFT JOIN payments p ON p.invoice_id=i.id WHERE i.legal_entity_id=$1 GROUP BY i.id,t.legal_name ORDER BY i.issue_date DESC,i.created_at DESC").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| InvoiceItem {
                id: r.get("id"),
                lease_id: r.get("lease_id"),
                invoice_number: r.get("invoice_number"),
                issue_date: r.get("issue_date"),
                due_date: r.get("due_date"),
                net_cents: r.get("net_cents"),
                vat_cents: r.get("vat_cents"),
                gross_cents: r.get("gross_cents"),
                paid_cents: r.get("paid_cents"),
                status: r.get("status"),
                tenant_name: r.get("tenant_name"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_invoices est exécutée côté serveur",
    ))
}

#[server]
pub async fn create_invoice_from_lease(
    lease_id: Uuid,
    issue_date: NaiveDate,
    due_date: NaiveDate,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if due_date < issue_date {
            return Err(ServerFnError::new("Échéance invalide"));
        }
        let row=sqlx::query("SELECT u.base_rent_cents,u.vat_rate_bp FROM leases l JOIN units u ON u.id=l.unit_id JOIN properties p ON p.id=u.property_id WHERE l.id=$1 AND p.legal_entity_id=$2 AND l.active=true").bind(lease_id).bind(current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Bail introuvable"))?;
        let net: i64 = row.get("base_rent_cents");
        let rate: i32 = row.get("vat_rate_bp");
        let vat = ((Decimal::from(net) * Decimal::from(rate)) / Decimal::from(10000u32))
            .round_dp(0)
            .to_i64()
            .unwrap_or(0);
        let gross = net + vat;
        let period_end = (issue_date.with_day(1).ok_or_else(||ServerFnError::new("Date de période invalide"))? + chrono::Duration::days(32)).with_day(1).ok_or_else(||ServerFnError::new("Date de période invalide"))? - chrono::Duration::days(1);
        let idempotency_key = format!("LEASE_INVOICE:{}:{}:{}", lease_id, issue_date, period_end);
        let existing: Option<Uuid> = sqlx::query_scalar("SELECT id FROM invoices WHERE legal_entity_id=$1 AND idempotency_key=$2")
            .bind(current_legal_entity_id()).bind(&idempotency_key).fetch_optional(pool).await.map_err(ServerFnError::new)?;
        if existing.is_none() {
            let number = format!(
                "{}-{:04}",
                issue_date.format("%Y%m"),
                next_invoice_sequence(pool, current_legal_entity_id(), issue_date)
                    .await
                    .map_err(ServerFnError::new)?
            );
            let inserted: Option<Uuid> = sqlx::query_scalar("INSERT INTO invoices(legal_entity_id,lease_id,invoice_number,issue_date,due_date,service_period_start,service_period_end,net_cents,vat_cents,gross_cents,status,idempotency_key) VALUES($1,$2,$3,$4,$5,$4,$6,$7,$8,$9,'DRAFT',$10) ON CONFLICT(legal_entity_id,idempotency_key) DO NOTHING RETURNING id")
                .bind(current_legal_entity_id()).bind(lease_id).bind(number).bind(issue_date).bind(due_date).bind(period_end).bind(net).bind(vat).bind(gross).bind(&idempotency_key).fetch_optional(pool).await.map_err(ServerFnError::new)?;
            if inserted.is_none() { return Ok(()); }
            audit(
                pool,
                "CREATE_INVOICE",
                Some("lease"),
                Some(lease_id),
                serde_json::json!({"gross_cents":gross,"idempotency_key":idempotency_key}),
            )
            .await
            .map_err(ServerFnError::new)?;
        }
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "create_invoice_from_lease est exécutée côté serveur",
    ))
}

#[server]
pub async fn issue_invoice(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let r=sqlx::query("UPDATE invoices SET status='ISSUED',updated_at=now() WHERE id=$1 AND legal_entity_id=$2 AND status='DRAFT'").bind(id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;
        if r.rows_affected() == 0 {
            return Err(ServerFnError::new("Facture introuvable ou déjà émise"));
        }
        record_business_event(pool, entity, "InvoiceIssued", Utc::now(), "invoice", Some(id), &format!("invoice-issued:{id}"), serde_json::json!({"invoice_id":id})).await.map_err(ServerFnError::new)?;
        audit(
            pool,
            "ISSUE_INVOICE",
            Some("invoice"),
            Some(id),
            serde_json::json!({}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "issue_invoice est exécutée côté serveur",
    ))
}

#[server]
pub async fn delete_invoice(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let entity = current_legal_entity_id();
        let r = sqlx::query("UPDATE invoices SET status='CANCELLED',updated_at=now() WHERE id=$1 AND legal_entity_id=$2 AND status='DRAFT'")
            .bind(id).bind(entity).execute(pool).await.map_err(ServerFnError::new)?;
        if r.rows_affected() == 0 {
            return Err(ServerFnError::new("Seuls les brouillons peuvent être annulés depuis cette action"));
        }
        sqlx::query("INSERT INTO billing_invoice_state_history(legal_entity_id,invoice_id,from_status,to_status,reason) VALUES($1,$2,'DRAFT','CANCELLED','Annulation du brouillon via compatibilité')")
            .bind(entity).bind(id).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("delete_invoice est exécutée côté serveur"))
}

#[server]
pub async fn list_payments() -> Result<Vec<PaymentItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT p.id,p.invoice_id,COALESCE(i.invoice_number,'Non affecté') invoice_number,p.received_at,p.amount_cents,COALESCE(p.reference,'') AS reference,p.source FROM payments p LEFT JOIN invoices i ON i.id=p.invoice_id WHERE p.legal_entity_id=$1 ORDER BY p.received_at DESC").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| PaymentItem {
                id: r.get("id"),
                invoice_id: r.get("invoice_id"),
                invoice_number: r.get("invoice_number"),
                received_at: r.get("received_at"),
                amount_cents: r.get("amount_cents"),
                reference: r.get("reference"),
                source: r.get("source"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_payments est exécutée côté serveur",
    ))
}

#[server]
pub async fn create_payment(
    invoice_id: Option<Uuid>,
    received_at: DateTime<Utc>,
    amount_cents: i64,
    reference: String,
    source: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if amount_cents <= 0 {
            return Err(ServerFnError::new("Montant d'encaissement invalide"));
        }
        if let Some(iid) = invoice_id {
            let ok: i64 = sqlx::query_scalar(
                "SELECT COUNT(*)::bigint FROM invoices WHERE id=$1 AND legal_entity_id=$2",
            )
            .bind(iid)
            .bind(current_legal_entity_id())
            .fetch_one(pool)
            .await
            .map_err(ServerFnError::new)?;
            if ok == 0 {
                return Err(ServerFnError::new("Facture invalide"));
            }
        }
        let entity = current_legal_entity_id();
        let idempotency_key = format!("PAYMENT:{}:{}:{}:{}:{}", invoice_id.map(|x|x.to_string()).unwrap_or_default(), received_at.timestamp_millis(), amount_cents, reference.trim(), source.trim());
        let payment_id: Uuid = match sqlx::query_scalar("INSERT INTO payments(legal_entity_id,invoice_id,received_at,amount_cents,reference,source,idempotency_key) VALUES($1,$2,$3,$4,NULLIF($5,''),$6,$7) ON CONFLICT(legal_entity_id,idempotency_key) DO NOTHING RETURNING id").bind(entity).bind(invoice_id).bind(received_at).bind(amount_cents).bind(reference.trim()).bind(if source.trim().is_empty(){"BANK"}else{source.trim()}).bind(&idempotency_key).fetch_optional(pool).await.map_err(ServerFnError::new)? { Some(id)=>id, None=>sqlx::query_scalar("SELECT id FROM payments WHERE legal_entity_id=$1 AND idempotency_key=$2").bind(entity).bind(&idempotency_key).fetch_one(pool).await.map_err(ServerFnError::new)? };
        record_financial_transaction(pool, entity, received_at, amount_cents, "IN", "PAYMENT", Some(payment_id), "RECORDED", Some(&format!("payment:{payment_id}")), "", reference.trim(), serde_json::json!({"invoice_id":invoice_id})).await.map_err(ServerFnError::new)?;
        record_business_event(pool, entity, "PaymentDetected", received_at, "payment", Some(payment_id), &format!("payment-detected:{payment_id}"), serde_json::json!({"invoice_id":invoice_id,"amount_cents":amount_cents})).await.map_err(ServerFnError::new)?;
        if let Some(iid) = invoice_id {
            refresh_invoice_status(pool, iid)
                .await
                .map_err(ServerFnError::new)?;
        }
        audit(
            pool,
            "CREATE_PAYMENT",
            Some("invoice"),
            invoice_id,
            serde_json::json!({"amount_cents":amount_cents}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "create_payment est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_bank_transactions() -> Result<Vec<BankTransactionItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let context = load_engine_context(pool, current_legal_entity_id()).await.map_err(ServerFnError::new)?;
        let id = context.legal_entity_id;
        let rows=sqlx::query("SELECT id,booked_at,value_date,amount_cents,label,COALESCE(counterparty,'') AS counterparty,COALESCE(external_id,'') AS external_id,reconciliation_status FROM bank_transactions WHERE legal_entity_id=$1 ORDER BY booked_at DESC LIMIT 250").bind(id).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| BankTransactionItem {
                id: r.get("id"),
                booked_at: r.get("booked_at"),
                value_date: r.get("value_date"),
                amount_cents: r.get("amount_cents"),
                label: r.get("label"),
                counterparty: r.get("counterparty"),
                external_id: r.get("external_id"),
                reconciliation_status: r.get("reconciliation_status"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_bank_transactions est exécutée côté serveur",
    ))
}

#[server]
pub async fn create_bank_transaction(
    booked_at: DateTime<Utc>,
    value_date: Option<NaiveDate>,
    amount_cents: i64,
    label: String,
    counterparty: String,
    external_id: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let context = load_engine_context(pool, current_legal_entity_id()).await.map_err(ServerFnError::new)?;
        let id = context.legal_entity_id;
        if label.trim().is_empty() {
            return Err(ServerFnError::new("Libellé bancaire requis"));
        }
        sqlx::query("INSERT INTO bank_transactions(legal_entity_id,booked_at,value_date,amount_cents,label,counterparty,external_id) VALUES($1,$2,$3,$4,$5,NULLIF($6,''),NULLIF($7,'')) ON CONFLICT(legal_entity_id,external_id) DO UPDATE SET booked_at=EXCLUDED.booked_at,value_date=EXCLUDED.value_date,amount_cents=EXCLUDED.amount_cents,label=EXCLUDED.label,counterparty=EXCLUDED.counterparty,updated_at=now()").bind(id).bind(booked_at).bind(value_date).bind(amount_cents).bind(label.trim()).bind(counterparty.trim()).bind(external_id.trim()).execute(pool).await.map_err(ServerFnError::new)?;
        let direction = if amount_cents >= 0 { "IN" } else { "OUT" };
        record_financial_transaction(pool, id, booked_at, amount_cents.unsigned_abs().min(i64::MAX as u64) as i64, direction, "BANK_TRANSACTION", None, "RECORDED", if external_id.trim().is_empty(){None}else{Some(external_id.trim())}, counterparty.trim(), label.trim(), serde_json::json!({"value_date":value_date})).await.map_err(ServerFnError::new)?;
        record_business_event(pool, id, "BankImported", booked_at, "bank_transaction", None, &format!("bank-import:{}:{}", id, external_id.trim()), serde_json::json!({"amount_cents":amount_cents,"external_id":external_id.trim()})).await.map_err(ServerFnError::new)?;
        audit(
            pool,
            "IMPORT_BANK_TRANSACTION",
            None,
            None,
            serde_json::json!({"amount_cents":amount_cents,"external_id":external_id}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "create_bank_transaction est exécutée côté serveur",
    ))
}

#[server]
pub async fn delete_bank_transaction(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let r=sqlx::query("DELETE FROM bank_transactions WHERE id=$1 AND legal_entity_id=$2 AND reconciliation_status='UNMATCHED'").bind(id).bind(current_legal_entity_id()).execute(pool).await.map_err(ServerFnError::new)?;
        if r.rows_affected() == 0 {
            return Err(ServerFnError::new(
                "Seul un mouvement non rapproché peut être supprimé",
            ));
        }
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "delete_bank_transaction est exécutée côté serveur",
    ))
}

#[server]
pub async fn import_bank_csv(csv: String) -> Result<usize, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let mut count = 0usize;
        for (line_no, line) in csv.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let parts: Vec<&str> = if line.contains(';') {
                line.split(';').collect()
            } else {
                line.split(',').collect()
            };
            if parts.len() < 3 {
                continue;
            }
            let date = NaiveDate::parse_from_str(parts[0].trim(), "%Y-%m-%d")
                .or_else(|_| NaiveDate::parse_from_str(parts[0].trim(), "%d/%m/%Y"));
            let date = match date {
                Ok(v) => v,
                Err(_) => {
                    if line_no == 0 {
                        continue;
                    } else {
                        continue;
                    }
                }
            };
            let amount = parse_csv_amount(parts[1]).map_err(|_| {
                ServerFnError::new(format!("Montant bancaire invalide ligne {}", line_no + 1))
            })?;
            let label = parts[2..].join(";");
            let ext = if parts.len() > 3 {
                parts[3].trim().to_string()
            } else {
                format!("CSV-{}-{}", date, line_no + 1)
            };
            create_bank_row(pool, date, amount, label, "".into(), ext)
                .await
                .map_err(ServerFnError::new)?;
            count += 1;
        }
        Ok(count)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "import_bank_csv est exécutée côté serveur",
    ))
}

#[server]
pub async fn reconcile_bank_transaction(
    bank_id: Uuid,
    invoice_id: Uuid,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let tx=sqlx::query("SELECT amount_cents FROM bank_transactions WHERE id=$1 AND legal_entity_id=$2 AND reconciliation_status='UNMATCHED'").bind(bank_id).bind(current_legal_entity_id()).fetch_optional(pool).await.map_err(ServerFnError::new)?.ok_or_else(||ServerFnError::new("Mouvement bancaire introuvable ou déjà rapproché"))?;
        let amount: i64 = tx.get("amount_cents");
        if amount <= 0 {
            return Err(ServerFnError::new(
                "Le rapprochement automatique d'un débit n'est pas autorisé ici",
            ));
        }
        let ok: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)::bigint FROM invoices WHERE id=$1 AND legal_entity_id=$2 AND status<>'DRAFT'",
        )
        .bind(invoice_id)
        .bind(current_legal_entity_id())
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;
        if ok == 0 {
            return Err(ServerFnError::new("Facture invalide"));
        }
        let mut tr = pool.begin().await.map_err(ServerFnError::new)?;
        let reconciliation_key = format!("BANK_RECONCILIATION:{}:{}", bank_id, invoice_id);
        sqlx::query("INSERT INTO payments(legal_entity_id,invoice_id,received_at,amount_cents,reference,source,idempotency_key) VALUES($1,$2,now(),$3,$4,'BANK_RECONCILIATION',$5) ON CONFLICT(legal_entity_id,idempotency_key) DO NOTHING").bind(current_legal_entity_id()).bind(invoice_id).bind(amount).bind(format!("Rapprochement {}",bank_id)).bind(&reconciliation_key).execute(&mut *tr).await.map_err(ServerFnError::new)?;
        sqlx::query("UPDATE bank_transactions SET reconciliation_status='MATCHED',updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(bank_id).bind(current_legal_entity_id()).execute(&mut *tr).await.map_err(ServerFnError::new)?;
        tr.commit().await.map_err(ServerFnError::new)?;
        refresh_invoice_status(pool, invoice_id)
            .await
            .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "reconcile_bank_transaction est exécutée côté serveur",
    ))
}

#[server]
pub async fn vat_summary(period: String) -> Result<VatSummary, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let context = load_engine_context(pool, current_legal_entity_id()).await.map_err(ServerFnError::new)?;
        let id = context.legal_entity_id;
        let start = NaiveDate::parse_from_str(&(period + "-01"), "%Y-%m-%d")
            .map_err(|_| ServerFnError::new("Période TVA invalide"))?;
        let end = (start + Duration::days(32)).with_day(1).unwrap();
        let row=sqlx::query("SELECT COALESCE(SUM(p.amount_cents),0)::bigint receipts,COALESCE(SUM(ROUND((p.amount_cents::numeric*i.net_cents)/NULLIF(i.gross_cents,0))),0)::bigint net,COALESCE(SUM(ROUND((p.amount_cents::numeric*i.vat_cents)/NULLIF(i.gross_cents,0))),0)::bigint vat,COUNT(*)::bigint count FROM payments p JOIN invoices i ON i.id=p.invoice_id WHERE p.legal_entity_id=$1 AND p.received_at >= $2 AND p.received_at < $3").bind(id).bind(start).bind(end).fetch_one(pool).await.map_err(ServerFnError::new)?;
        Ok(VatSummary {
            period_label: start.format("%Y-%m").to_string(),
            receipts_gross_cents: row.get("receipts"),
            taxable_net_cents: row.get("net"),
            vat_due_cents: row.get("vat"),
            payments_count: row.get("count"),
        })
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "vat_summary est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_deadlines() -> Result<Vec<DeadlineItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT id,code,label,deadline_date,COALESCE(period_label,'') AS period_label,status FROM tax_deadlines WHERE legal_entity_id=$1 ORDER BY deadline_date").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| DeadlineItem {
                id: r.get("id"),
                code: r.get("code"),
                label: r.get("label"),
                deadline_date: r.get("deadline_date"),
                period_label: r.get("period_label"),
                status: r.get("status"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_deadlines est exécutée côté serveur",
    ))
}

#[server]
pub async fn create_deadline(
    code: String,
    label: String,
    deadline_date: NaiveDate,
    period_label: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if code.trim().is_empty() || label.trim().is_empty() {
            return Err(ServerFnError::new("Code et libellé requis"));
        }
        sqlx::query("INSERT INTO tax_deadlines(legal_entity_id,code,label,deadline_date,period_label,status) VALUES($1,$2,$3,$4,NULLIF($5,''),'PLANNED') ON CONFLICT(legal_entity_id,code,deadline_date) DO UPDATE SET label=EXCLUDED.label,period_label=EXCLUDED.period_label").bind(current_legal_entity_id()).bind(code.trim()).bind(label.trim()).bind(deadline_date).bind(period_label.trim()).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "create_deadline est exécutée côté serveur",
    ))
}

#[server]
pub async fn set_deadline_status(id: Uuid, status: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if !["PLANNED", "READY", "DONE", "SKIPPED"].contains(&status.as_str()) {
            return Err(ServerFnError::new("Statut d'échéance invalide"));
        }
        sqlx::query("UPDATE tax_deadlines SET status=$3 WHERE id=$1 AND legal_entity_id=$2")
            .bind(id)
            .bind(current_legal_entity_id())
            .bind(status)
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "set_deadline_status est exécutée côté serveur",
    ))
}

#[server]
pub async fn delete_deadline(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        sqlx::query("DELETE FROM tax_deadlines WHERE id=$1 AND legal_entity_id=$2 AND status<>'DONE'")
            .bind(id)
            .bind(current_legal_entity_id())
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "delete_deadline est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_documents() -> Result<Vec<DocumentItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let context = load_engine_context(pool, current_legal_entity_id()).await.map_err(ServerFnError::new)?;
        let id = context.legal_entity_id;
        let rows=sqlx::query("SELECT id,category,title,file_name,storage_key,document_date,expires_at,origin,file_size_bytes,mime_type,status,ocr_status,classification_confidence,extraction_confidence,duplicate_status FROM documents WHERE legal_entity_id=$1 ORDER BY COALESCE(document_date,DATE '1900-01-01') DESC,created_at DESC").bind(id).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| DocumentItem {
                id: r.get("id"),
                category: r.get("category"),
                title: r.get("title"),
                file_name: r.get("file_name"),
                storage_key: r.get("storage_key"),
                document_date: r.get("document_date"),
                expires_at: r.get("expires_at"),
                origin: r.get("origin"),
                file_size_bytes: r.get("file_size_bytes"),
                mime_type: r.get("mime_type"),
                status: r.get("status"),
                ocr_status: r.get("ocr_status"),
                classification_confidence: r.get("classification_confidence"),
                extraction_confidence: r.get("extraction_confidence"),
                duplicate_status: r.get("duplicate_status"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_documents est exécutée côté serveur",
    ))
}

#[server]
pub async fn register_document(
    category: String,
    title: String,
    file_name: String,
    storage_key: String,
    document_date: Option<NaiveDate>,
    expires_at: Option<NaiveDate>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let context = load_engine_context(pool, current_legal_entity_id()).await.map_err(ServerFnError::new)?;
        let id = context.legal_entity_id;
        if category.trim().is_empty()
            || title.trim().is_empty()
            || file_name.trim().is_empty()
            || storage_key.trim().is_empty()
        {
            return Err(ServerFnError::new(
                "Catégorie, titre, nom et emplacement requis",
            ));
        }
        sqlx::query("INSERT INTO documents(legal_entity_id,category,title,file_name,storage_key,document_date,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(id).bind(category.trim()).bind(title.trim()).bind(file_name.trim()).bind(storage_key.trim()).bind(document_date).bind(expires_at).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "register_document est exécutée côté serveur",
    ))
}

#[server]
pub async fn delete_document(id: Uuid) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let changed=sqlx::query("UPDATE documents SET status='ARCHIVED',archived_at=now(),archive_reason='Archivage via compatibilité' WHERE id=$1 AND legal_entity_id=$2 AND archived_at IS NULL")
            .bind(id)
            .bind(current_legal_entity_id())
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        if changed.rows_affected()>0 {
            sqlx::query("INSERT INTO document_events(legal_entity_id,document_id,event_type,actor,payload) VALUES($1,$2,'ARCHIVED','MANAGER',$3)")
                .bind(current_legal_entity_id()).bind(id).bind(serde_json::json!({"source":"delete_document compatibility"}))
                .execute(pool).await.map_err(ServerFnError::new)?;
        }
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "delete_document est exécutée côté serveur",
    ))
}

#[server]
pub async fn assistant_ask(
    message: String,
    intent: crate::assistant::AssistantIntent,
) -> Result<crate::assistant::AssistantResponse, ServerFnError> {
    let request = crate::assistant::AssistantRequest { message, intent };
    let context = crate::assistant::AssistantContext {
        workspace_id: Uuid::nil(),
        legal_entity_id: current_legal_entity_id(),
    };
    Ok(crate::assistant::handle_request(&request, &context))
}

#[server]
pub async fn list_automation_rules() -> Result<Vec<AutomationRuleItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT id,code,name,description,trigger_kind,horizon_days,priority,auto_execute,enabled FROM automation_rules WHERE legal_entity_id=$1 ORDER BY priority DESC,name").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| AutomationRuleItem {
                id: r.get("id"),
                code: r.get("code"),
                name: r.get("name"),
                description: r.get("description"),
                trigger_kind: r.get("trigger_kind"),
                horizon_days: r.get("horizon_days"),
                priority: r.get("priority"),
                auto_execute: r.get("auto_execute"),
                enabled: r.get("enabled"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_automation_rules est exécutée côté serveur",
    ))
}

#[server]
pub async fn set_automation_enabled(id: Uuid, enabled: bool) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        sqlx::query(
            "UPDATE automation_rules SET enabled=$3,updated_at=now() WHERE id=$1 AND legal_entity_id=$2",
        )
        .bind(id)
        .bind(current_legal_entity_id())
        .bind(enabled)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;
        audit(
            pool,
            "TOGGLE_AUTOMATION",
            Some("automation_rule"),
            Some(id),
            serde_json::json!({"enabled":enabled}),
        )
        .await
        .map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "set_automation_enabled est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_tasks() -> Result<Vec<TaskItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        list_tasks_inner(pool, current_legal_entity_id(), 250)
            .await
            .map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_tasks est exécutée côté serveur",
    ))
}

#[server]
pub async fn set_task_state(id: Uuid, state: String) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        if !["PLANNED", "READY", "RUNNING", "BLOCKED", "DONE", "SKIPPED"].contains(&state.as_str())
        {
            return Err(ServerFnError::new("Statut de tâche invalide"));
        }
        sqlx::query("UPDATE tasks SET state=$3,completed_at=CASE WHEN $3='DONE' THEN now() ELSE NULL END WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(current_legal_entity_id()).bind(state).execute(pool).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "set_task_state est exécutée côté serveur",
    ))
}

#[server]
pub async fn list_audit() -> Result<Vec<AuditItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows=sqlx::query("SELECT id,occurred_at,actor,action,COALESCE(entity_type,'') AS entity_type,payload::text FROM audit_events WHERE legal_entity_id=$1 ORDER BY id DESC LIMIT 250").bind(current_legal_entity_id()).fetch_all(pool).await.map_err(ServerFnError::new)?;
        Ok(rows
            .into_iter()
            .map(|r| AuditItem {
                id: r.get("id"),
                occurred_at: r.get("occurred_at"),
                actor: r.get("actor"),
                action: r.get("action"),
                entity_type: r.get("entity_type"),
                payload: r.get("payload"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "list_audit est exécutée côté serveur",
    ))
}

#[server]
pub async fn run_anticipation_cycle() -> Result<AutomationRunResult, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        automation_tick(pool).await.map_err(ServerFnError::new)
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "run_anticipation_cycle est exécutée côté serveur",
    ))
}

#[cfg(feature = "server")]
pub async fn automation_tick(pool: &sqlx::PgPool) -> Result<AutomationRunResult, sqlx::Error> {
    let context = load_engine_context(pool, current_legal_entity_id()).await?;
    let id = context.legal_entity_id;
    let now = Utc::now();
    let job_payload = serde_json::json!({"run_date": now.date_naive(), "entity_id": id});
    let claim = claim_job(pool, id, "ANTICIPATION_CYCLE", &format!("{}", now.date_naive()), &job_payload).await?;
    if !claim.execute {
        return Ok(AutomationRunResult { created_tasks: 0, evaluated_rules: 0, ran_at: now });
    }
    let rules=sqlx::query("SELECT id,code,name,horizon_days,priority FROM automation_rules WHERE legal_entity_id=$1 AND enabled=true").bind(id).fetch_all(pool).await?;
    let mut created = 0usize;
    for r in &rules {
        let rid: Uuid = r.get("id");
        let code: String = r.get("code");
        let name: String = r.get("name");
        let horizon: i32 = r.get("horizon_days");
        let priority: i32 = r.get("priority");
        let due_date = (now + Duration::days(i64::from(horizon.clamp(1, 365)))).date_naive();
        let due = due_date.and_hms_opt(9, 0, 0).unwrap().and_utc();
        let key = due_date.to_string();
        let x=sqlx::query("INSERT INTO tasks(legal_entity_id,automation_rule_id,code,title,description,due_at,state,priority,source,occurrence_key,blocking) VALUES($1,$2,$3,$4,$5,$6,'READY',$7,'AUTOMATION',$8,false) ON CONFLICT(legal_entity_id,code,occurrence_key) DO NOTHING").bind(id).bind(rid).bind(&code).bind(&name).bind("Préparée automatiquement par le moteur d'anticipation.").bind(due).bind(priority).bind(&key).execute(pool).await?;
        created += x.rows_affected() as usize;
    }
    let deadline_rows=sqlx::query("SELECT id,code,label,deadline_date FROM tax_deadlines WHERE legal_entity_id=$1 AND status<>'DONE' AND deadline_date BETWEEN CURRENT_DATE AND CURRENT_DATE+60").bind(id).fetch_all(pool).await?;
    for r in deadline_rows {
        let did: Uuid = r.get("id");
        let code: String = r.get("code");
        let label: String = r.get("label");
        let date: NaiveDate = r.get("deadline_date");
        let due = date.and_hms_opt(9, 0, 0).unwrap().and_utc();
        let key = format!("deadline:{}", did);
        let x=sqlx::query("INSERT INTO tasks(legal_entity_id,code,title,description,due_at,state,priority,source,occurrence_key,blocking) VALUES($1,$2,$3,$4,$5,'READY',100,'CALENDAR',$6,true) ON CONFLICT(legal_entity_id,code,occurrence_key) DO NOTHING").bind(id).bind(format!("DEADLINE_{}",code)).bind(label).bind("Échéance déclarative : préparer les pièces et le contrôle avant la date limite.").bind(due).bind(key).execute(pool).await?;
        created += x.rows_affected() as usize;
    }
    audit(
        pool,
        "ANTICIPATION_CYCLE",
        None,
        None,
        serde_json::json!({"created_tasks":created,"evaluated_rules":rules.len()}),
    )
    .await?;
    complete_job(pool,claim.id,&serde_json::json!({"created_tasks":created,"evaluated_rules":rules.len()})).await?;
    Ok(AutomationRunResult {
        created_tasks: created,
        evaluated_rules: rules.len(),
        ran_at: now,
    })
}

#[cfg(feature = "server")]
async fn fetch_legal_entity_header(
    pool: &sqlx::PgPool,
    id: Uuid,
) -> Result<LegalEntityItem, sqlx::Error> {
    let r = sqlx::query(
        "SELECT e.id,e.legal_name,e.legal_form_code,e.tax_regime,e.vat_status,e.vat_basis,COALESCE(e.siren,'') AS siren,COALESCE(e.siret,'') AS siret,e.registered_office,e.accounting_period_start,e.fiscal_year_end,e.currency_code,e.active,COUNT(a.id) FILTER (WHERE a.active) AS bank_accounts_count,COALESCE(MAX(a.iban) FILTER (WHERE a.active AND a.is_primary),'') AS primary_iban,COALESCE(MAX(a.bic) FILTER (WHERE a.active AND a.is_primary),'') AS primary_bic FROM legal_entities e LEFT JOIN legal_entity_bank_accounts a ON a.legal_entity_id=e.id WHERE e.id=$1 GROUP BY e.id",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(LegalEntityItem {
        id: r.get("id"),
        legal_name: r.get("legal_name"),
        legal_form_code: r.get("legal_form_code"),
        tax_regime: r.get("tax_regime"),
        vat_status: r.get("vat_status"),
        vat_basis: r.get("vat_basis"),
        siren: r.get("siren"),
        siret: r.get("siret"),
        registered_office: r.get("registered_office"),
        accounting_period_start: r.get::<i16,_>("accounting_period_start") as u8,
        fiscal_year_end: r.get::<i16,_>("fiscal_year_end") as u8,
        currency_code: r.get("currency_code"),
        active: r.get("active"),
        bank_accounts_count: r.get("bank_accounts_count"),
        primary_iban: r.get("primary_iban"),
        primary_bic: r.get("primary_bic"),
    })
}

#[cfg(feature = "server")]
async fn fetch_profile(pool: &sqlx::PgPool, id: Uuid) -> Result<SciProfile, sqlx::Error> {
    let r = sqlx::query(
        "SELECT e.legal_name,COALESCE(e.siren,'') AS siren,COALESCE(e.siret,'') AS siret,COALESCE(e.registered_office,'') AS registered_office,e.tax_regime,e.vat_status,e.vat_basis,e.accounting_period_start,e.fiscal_year_end,COALESCE(MAX(a.iban) FILTER (WHERE a.active AND a.is_primary),'') AS iban,COALESCE(MAX(a.bic) FILTER (WHERE a.active AND a.is_primary),'') AS bic FROM legal_entities e LEFT JOIN legal_entity_bank_accounts a ON a.legal_entity_id=e.id WHERE e.id=$1 GROUP BY e.id",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(SciProfile {
        legal_name: r.get("legal_name"),
        siren: r.get("siren"),
        siret: r.get("siret"),
        registered_office: r.get("registered_office"),
        tax_regime: r.get("tax_regime"),
        vat_status: r.get("vat_status"),
        vat_basis: r.get("vat_basis"),
        accounting_period_start: r.get::<i16, _>("accounting_period_start") as u8,
        fiscal_year_end: r.get::<i16, _>("fiscal_year_end") as u8,
        iban: r.get("iban"),
        bic: r.get("bic"),
    })
}

#[cfg(feature = "server")]
async fn fetch_counts(pool: &sqlx::PgPool, id: Uuid) -> Result<ModuleCounts, sqlx::Error> {
    let q = |s: &'static str| scalar_count(pool, s, id);
    Ok(ModuleCounts{associates:q("SELECT COUNT(*)::bigint FROM associates WHERE legal_entity_id=$1 AND active").await?,properties:q("SELECT COUNT(*)::bigint FROM properties WHERE legal_entity_id=$1 AND active").await?,units:q("SELECT COUNT(*)::bigint FROM units u JOIN properties p ON p.id=u.property_id WHERE p.legal_entity_id=$1 AND u.active").await?,tenants:q("SELECT COUNT(*)::bigint FROM tenants WHERE legal_entity_id=$1 AND active").await?,leases:q("SELECT COUNT(*)::bigint FROM leases l JOIN units u ON u.id=l.unit_id JOIN properties p ON p.id=u.property_id WHERE p.legal_entity_id=$1 AND l.active").await?,invoices:q("SELECT COUNT(*)::bigint FROM invoices WHERE legal_entity_id=$1").await?,payments:q("SELECT COUNT(*)::bigint FROM payments WHERE legal_entity_id=$1").await?,bank_transactions:q("SELECT COUNT(*)::bigint FROM bank_transactions WHERE legal_entity_id=$1").await?,unmatched_bank:q("SELECT COUNT(*)::bigint FROM bank_transactions WHERE legal_entity_id=$1 AND reconciliation_status='UNMATCHED'").await?,vat_receipts_cents:sqlx::query_scalar("SELECT COALESCE(SUM(amount_cents),0)::bigint FROM payments WHERE legal_entity_id=$1 AND received_at>=date_trunc('month',now())").bind(id).fetch_one(pool).await?,tax_deadlines:q("SELECT COUNT(*)::bigint FROM tax_deadlines WHERE legal_entity_id=$1 AND deadline_date>=CURRENT_DATE").await?,documents:q("SELECT COUNT(*)::bigint FROM documents WHERE legal_entity_id=$1").await?,active_automation_rules:q("SELECT COUNT(*)::bigint FROM automation_rules WHERE legal_entity_id=$1 AND enabled").await?,open_tasks:q("SELECT COUNT(*)::bigint FROM tasks WHERE legal_entity_id=$1 AND state NOT IN ('DONE','SKIPPED')").await?})
}

#[cfg(feature = "server")]
async fn scalar_count(
    pool: &sqlx::PgPool,
    sql: &'static str,
    id: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(sql).bind(id).fetch_one(pool).await
}

#[cfg(feature = "server")]
async fn list_tasks_inner(
    pool: &sqlx::PgPool,
    id: Uuid,
    limit: i64,
) -> Result<Vec<TaskItem>, sqlx::Error> {
    let rows=sqlx::query("SELECT id,code,title,COALESCE(description,'') AS description,due_at,state,priority,blocking FROM tasks WHERE legal_entity_id=$1 ORDER BY CASE state WHEN 'BLOCKED' THEN 0 WHEN 'READY' THEN 1 ELSE 2 END,priority DESC,due_at LIMIT $2").bind(id).bind(limit).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|r| TaskItem {
            id: r.get("id"),
            code: r.get("code"),
            title: r.get("title"),
            description: r.get("description"),
            due_at: r.get("due_at"),
            state: task_state(r.get("state")),
            priority: r.get("priority"),
            blocking: r.get("blocking"),
        })
        .collect())
}

#[cfg(feature = "server")]
async fn build_forecast(
    pool: &sqlx::PgPool,
    id: Uuid,
    cash: i64,
) -> Result<Vec<CashForecastPoint>, sqlx::Error> {
    let rows=sqlx::query("SELECT date_trunc('month',CURRENT_DATE + (g||' month')::interval)::date forecast_month,COALESCE((SELECT SUM(u.base_rent_cents + ROUND(u.base_rent_cents::numeric*u.vat_rate_bp/10000))::bigint FROM leases l JOIN units u ON u.id=l.unit_id JOIN properties p ON p.id=u.property_id WHERE p.legal_entity_id=$1 AND l.active AND l.start_date <= (date_trunc('month',CURRENT_DATE + (g||' month')::interval)+interval '1 month' - interval '1 day')::date AND (l.end_date IS NULL OR l.end_date >= date_trunc('month',CURRENT_DATE + (g||' month')::interval)::date)),0)::bigint inflow FROM generate_series(0,11) g").bind(id).fetch_all(pool).await?;
    let mut balance = cash;
    Ok(rows
        .into_iter()
        .map(|r| {
            let date: NaiveDate = r.get("forecast_month");
            let inflow: i64 = r.get("inflow");
            let outflow = 0i64;
            balance += inflow - outflow;
            CashForecastPoint {
                date,
                expected_inflows_cents: inflow,
                expected_outflows_cents: outflow,
                expected_vat_cents: 0,
                balance_cents: balance,
            }
        })
        .collect())
}

#[cfg(feature = "server")]
async fn refresh_invoice_status(pool: &sqlx::PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    use sqlx::Row;
    let entity = current_legal_entity_id();
    let row = sqlx::query("SELECT status,due_date,gross_cents,COALESCE((SELECT SUM(amount_cents) FROM payments p WHERE p.invoice_id=invoices.id AND p.legal_entity_id=invoices.legal_entity_id),0)::bigint paid_cents FROM invoices WHERE id=$1 AND legal_entity_id=$2")
        .bind(id).bind(entity).fetch_optional(pool).await?;
    let Some(r) = row else { return Ok(()); };
    let old:String = r.get("status");
    if matches!(old.as_str(),"DRAFT"|"VALIDATED"|"CANCELLED"|"CREDITED") { return Ok(()); }
    let due_date:NaiveDate = r.get("due_date");
    let gross:i64 = r.get("gross_cents");
    let paid:i64 = r.get("paid_cents");
    let new_status = if paid >= gross { "PAID" } else if paid > 0 { "PAID_PARTIAL" } else if due_date < Utc::now().date_naive() { "OVERDUE" } else { "ISSUED" };
    if old != new_status {
        sqlx::query("UPDATE invoices SET status=$3,updated_at=now() WHERE id=$1 AND legal_entity_id=$2").bind(id).bind(entity).bind(new_status).execute(pool).await?;
        sqlx::query("INSERT INTO billing_invoice_state_history(legal_entity_id,invoice_id,from_status,to_status,reason) VALUES($1,$2,$3,$4,'Actualisation après encaissement')").bind(entity).bind(id).bind(&old).bind(new_status).execute(pool).await?;
    }
    Ok(())
}

#[cfg(feature = "server")]
async fn next_invoice_sequence(
    pool: &sqlx::PgPool,
    id: Uuid,
    date: NaiveDate,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*)::bigint+1 FROM invoices WHERE legal_entity_id=$1 AND issue_date>=date_trunc('month',$2::date) AND issue_date<date_trunc('month',$2::date)+interval '1 month'").bind(id).bind(date).fetch_one(pool).await
}

#[cfg(feature = "server")]
async fn audit(
    pool: &sqlx::PgPool,
    action: &str,
    entity_type: Option<&str>,
    entity_id: Option<Uuid>,
    payload: serde_json::Value,
) -> Result<(), sqlx::Error> {
    let context = load_engine_context(pool, current_legal_entity_id()).await?;
    sqlx::query("INSERT INTO audit_events(legal_entity_id,actor,action,entity_type,entity_id,payload) VALUES($1,'MANAGER',$2,$3,$4,$5)").bind(context.legal_entity_id).bind(action).bind(entity_type).bind(entity_id).bind(payload).execute(pool).await.map(|_|())
}

#[cfg(feature = "server")]
fn parse_csv_amount(value: &str) -> Result<i64, ()> {
    let mut s = value.trim().replace("€", "").replace(" ", "");
    if s.contains(',') && s.contains('.') {
        if s.rfind(',') > s.rfind('.') {
            s = s.replace(".", "").replace(',', ".");
        } else {
            s = s.replace(",", "");
        }
    } else {
        s = s.replace(',', ".");
    }
    s.parse::<f64>()
        .map(|v| v.round() as i64 * 100)
        .map_err(|_| ())
}

#[cfg(feature = "server")]
async fn create_bank_row(
    pool: &sqlx::PgPool,
    date: NaiveDate,
    amount: i64,
    label: String,
    counterparty: String,
    external_id: String,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO bank_transactions(legal_entity_id,booked_at,value_date,amount_cents,label,counterparty,external_id) VALUES($1,$2,$3,$4,$5,NULLIF($6,''),NULLIF($7,'')) ON CONFLICT(legal_entity_id,external_id) DO UPDATE SET amount_cents=EXCLUDED.amount_cents,label=EXCLUDED.label,counterparty=EXCLUDED.counterparty,value_date=EXCLUDED.value_date,updated_at=now()").bind(current_legal_entity_id()).bind(date.and_hms_opt(12,0,0).unwrap().and_utc()).bind(date).bind(amount).bind(label).bind(counterparty).bind(external_id).execute(pool).await.map(|_|())
}

#[cfg(feature = "server")]
fn validate_profile(p: &SciProfile) -> Result<(), String> {
    if p.legal_name.trim().is_empty() {
        return Err("Dénomination sociale requise".into());
    }
    if p.registered_office.trim().is_empty() {
        return Err("Siège social requis".into());
    }
    if !(1..=12).contains(&p.accounting_period_start) || !(1..=12).contains(&p.fiscal_year_end) {
        return Err("Mois comptable invalide".into());
    }
    Ok(())
}

fn task_state(v: String) -> TaskState {
    match v.as_str() {
        "READY" => TaskState::Ready,
        "RUNNING" => TaskState::Running,
        "BLOCKED" => TaskState::Blocked,
        "DONE" => TaskState::Done,
        "SKIPPED" => TaskState::Skipped,
        _ => TaskState::Planned,
    }
}

