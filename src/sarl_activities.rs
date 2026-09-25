use crate::business_profiles::{list_business_profile_catalog, list_entity_business_profiles, set_entity_business_profile};
use crate::domain::{LegalActivityCatalogItem, LegalEntityActivityDraft, LegalEntityActivityItem, LegalEntityBusinessProfileDraft, LegalEntityItem};
#[cfg(feature = "server")]
use crate::history::{record_entity_change, snapshot_activity_set};
#[cfg(feature = "server")]
use chrono::Utc;
use dioxus::prelude::*;
use uuid::Uuid;

#[cfg(feature = "server")]
use crate::infrastructure::db;
#[cfg(feature = "server")]
use sqlx::Row;

#[server]
pub async fn list_sarl_entities_for_activities() -> Result<Vec<LegalEntityItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
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
            LEFT JOIN legal_entity_bank_accounts a ON a.legal_entity_id = e.id
            WHERE e.legal_form_code = 'SARL'
            GROUP BY e.id
            ORDER BY e.active DESC, e.legal_name
            "#,
        )
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
    Err(ServerFnError::new("list_sarl_entities_for_activities est exécutée côté serveur"))
}

#[server]
pub async fn list_activity_catalog() -> Result<Vec<LegalActivityCatalogItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            "SELECT code,label,description,active FROM legal_activity_catalog ORDER BY active DESC,label",
        )
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| LegalActivityCatalogItem {
                code: r.get("code"),
                label: r.get("label"),
                description: r.get("description"),
                active: r.get("active"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_activity_catalog est exécutée côté serveur"))
}

#[server]
pub async fn list_legal_entity_activities(
    legal_entity_id: Uuid,
) -> Result<Vec<LegalEntityActivityItem>, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let pool = db().await.map_err(ServerFnError::new)?;
        let rows = sqlx::query(
            r#"
            SELECT
                a.legal_entity_id,
                a.activity_code,
                c.label,
                c.description,
                a.is_primary,
                a.active,
                a.notes
            FROM legal_entity_activities a
            JOIN legal_activity_catalog c ON c.code = a.activity_code
            WHERE a.legal_entity_id = $1
            ORDER BY a.active DESC, a.is_primary DESC, c.label
            "#,
        )
        .bind(legal_entity_id)
        .fetch_all(pool)
        .await
        .map_err(ServerFnError::new)?;

        Ok(rows
            .into_iter()
            .map(|r| LegalEntityActivityItem {
                legal_entity_id: r.get("legal_entity_id"),
                activity_code: r.get("activity_code"),
                label: r.get("label"),
                description: r.get("description"),
                is_primary: r.get("is_primary"),
                active: r.get("active"),
                notes: r.get("notes"),
            })
            .collect())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("list_legal_entity_activities est exécutée côté serveur"))
}

#[server]
pub async fn set_legal_entity_activity(
    draft: LegalEntityActivityDraft,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        validate_activity_draft(&draft).map_err(ServerFnError::new)?;
        let pool = db().await.map_err(ServerFnError::new)?;
        let before = snapshot_activity_set(pool, draft.legal_entity_id).await.map_err(ServerFnError::new)?;

        let is_sarl: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM legal_entities WHERE id=$1 AND legal_form_code='SARL')",
        )
        .bind(draft.legal_entity_id)
        .fetch_one(pool)
        .await
        .map_err(ServerFnError::new)?;
        if !is_sarl {
            return Err(ServerFnError::new(
                "Les activités de cette story ne peuvent être rattachées qu'à une SARL.",
            ));
        }

        if draft.active {
            sqlx::query(
                r#"
                INSERT INTO legal_entity_activities(
                    legal_entity_id,activity_code,is_primary,active,notes
                ) VALUES($1,$2,$3,true,$4)
                ON CONFLICT (legal_entity_id,activity_code)
                DO UPDATE SET active=true,is_primary=EXCLUDED.is_primary,notes=EXCLUDED.notes,updated_at=now()
                "#,
            )
            .bind(draft.legal_entity_id)
            .bind(draft.activity_code.trim().to_uppercase())
            .bind(draft.is_primary)
            .bind(draft.notes.trim())
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        } else {
            sqlx::query(
                "UPDATE legal_entity_activities SET active=false,is_primary=false,updated_at=now() WHERE legal_entity_id=$1 AND activity_code=$2",
            )
            .bind(draft.legal_entity_id)
            .bind(draft.activity_code.trim().to_uppercase())
            .execute(pool)
            .await
            .map_err(ServerFnError::new)?;
        }

        audit_activity(
            pool,
            draft.legal_entity_id,
            if draft.active { "ACTIVATE_SARL_ACTIVITY" } else { "DEACTIVATE_SARL_ACTIVITY" },
            serde_json::json!({
                "activity_code": draft.activity_code.trim().to_uppercase(),
                "is_primary": draft.is_primary,
                "notes": draft.notes.trim()
            }),
        )
        .await
        .map_err(ServerFnError::new)?;
        let after = snapshot_activity_set(pool, draft.legal_entity_id).await.map_err(ServerFnError::new)?;
        record_entity_change(pool, draft.legal_entity_id, if draft.active { "ACTIVATE_SARL_ACTIVITY" } else { "DEACTIVATE_SARL_ACTIVITY" }, "LEGAL_ENTITY_ACTIVITY", draft.legal_entity_id, before, after, None, Utc::now(), serde_json::json!({"activity_code":draft.activity_code.trim().to_uppercase(),"source":"Activités SARL"})).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new("set_legal_entity_activity est exécutée côté serveur"))
}

#[server]
pub async fn set_primary_legal_entity_activity(
    legal_entity_id: Uuid,
    activity_code: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "server")]
    {
        let code = activity_code.trim().to_uppercase();
        if code.is_empty() {
            return Err(ServerFnError::new("Code activité requis"));
        }
        let pool = db().await.map_err(ServerFnError::new)?;
        let before = snapshot_activity_set(pool, legal_entity_id).await.map_err(ServerFnError::new)?;
        let updated = sqlx::query(
            r#"
            UPDATE legal_entity_activities
            SET is_primary=true, updated_at=now()
            WHERE legal_entity_id=$1
              AND activity_code=$2
              AND active=true
              AND EXISTS(
                  SELECT 1 FROM legal_entities e
                  WHERE e.id=$1 AND e.legal_form_code='SARL'
              )
            "#,
        )
        .bind(legal_entity_id)
        .bind(&code)
        .execute(pool)
        .await
        .map_err(ServerFnError::new)?;

        if updated.rows_affected() != 1 {
            return Err(ServerFnError::new(
                "Cette activité n'est pas active sur cette SARL.",
            ));
        }

        audit_activity(
            pool,
            legal_entity_id,
            "SET_PRIMARY_SARL_ACTIVITY",
            serde_json::json!({"activity_code": code}),
        )
        .await
        .map_err(ServerFnError::new)?;
        let after = snapshot_activity_set(pool, legal_entity_id).await.map_err(ServerFnError::new)?;
        record_entity_change(pool, legal_entity_id, "SET_PRIMARY_SARL_ACTIVITY", "LEGAL_ENTITY_ACTIVITY", legal_entity_id, before, after, Some("Changement d’activité principale"), Utc::now(), serde_json::json!({"activity_code":code,"source":"Activités SARL"})).await.map_err(ServerFnError::new)?;
        Ok(())
    }
    #[cfg(not(feature = "server"))]
    Err(ServerFnError::new(
        "set_primary_legal_entity_activity est exécutée côté serveur",
    ))
}

fn validate_activity_draft(draft: &LegalEntityActivityDraft) -> Result<(), String> {
    if draft.legal_entity_id.is_nil() {
        return Err("Entité juridique requise".into());
    }
    if draft.activity_code.trim().is_empty() {
        return Err("Code activité requis".into());
    }
    if !draft
        .activity_code
        .trim()
        .chars()
        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return Err("Code activité invalide".into());
    }
    Ok(())
}

#[cfg(feature = "server")]
async fn audit_activity(
    pool: &sqlx::PgPool,
    legal_entity_id: Uuid,
    action: &str,
    payload: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_events(sci_id,legal_entity_id,actor,action,entity_type,entity_id,payload) VALUES(NULL,$1,'MANAGER',$2,'LEGAL_ENTITY_ACTIVITY',$1,$3)",
    )
    .bind(legal_entity_id)
    .bind(action)
    .bind(payload)
    .execute(pool)
    .await
    .map(|_| ())
}

#[component]
pub fn SarlActivitiesPage(refresh: Signal<u64>) -> Element {
    let mut bump = use_signal(|| 0u64);
    let entities = use_resource(move || {
        let _ = refresh();
        let _ = bump();
        async move { list_sarl_entities_for_activities().await.unwrap_or_default() }
    });
    let catalog = use_resource(move || async move { list_activity_catalog().await.unwrap_or_default() });
    let mut selected = use_signal(String::new);
    let activities = use_resource(move || {
        let _ = bump();
        let id = Uuid::parse_str(&selected()).ok();
        async move {
            match id {
                Some(entity_id) => list_legal_entity_activities(entity_id).await.unwrap_or_default(),
                None => Vec::new(),
            }
        }
    });
    let mut msg = use_signal(String::new);
    let selected_entity_id = use_memo(move || Uuid::parse_str(&selected()).ok());
    let profiles = use_resource(move || {
        let id = selected_entity_id();
        async move { list_entity_business_profiles(id).await.unwrap_or_default() }
    });
    let profile_catalog = use_resource(|| async move { list_business_profile_catalog().await.unwrap_or_default() });

    rsx! {
        div {
            class: "module-header",
            div { class: "eyebrow", "SARL • ACTIVITÉS" }
            h2 { "Activités de la SARL" }
            p { "Plusieurs activités peuvent être activées simultanément dans une même SARL. Cette story ne cloisonne pas encore les flux métier existants entre sociétés : cela relève de US-0203." }
        }
        section {
            class: "panel",
            div { class: "panel-head", h3 { "SARL" }, span { class: "small", "Sélectionner l'entité à piloter" } }
            label {
                class: "field",
                span { "Entité" }
                select {
                    value: selected(),
                    onchange: move |e: FormEvent| { selected.set(e.value()); msg.set(String::new()); },
                    option { value: "", "Sélectionner une SARL" }
                    for entity in entities.read().as_deref().unwrap_or(&[]).iter().cloned() {
                        option { value: entity.id.to_string(), "{entity.legal_name}" }
                    }
                }
            }
            if entities.read().as_deref().unwrap_or(&[]).is_empty() {
                div { class: "empty-state", strong { "Aucune SARL" }, p { "Créez d'abord une entité de forme SARL dans « Entités »." } }
            }
        }
        if !selected().is_empty() {
            section {
                class: "panel",
                div { class: "panel-head", h3 { "Profils d'exploitation" }, span { class: "small", "Modules métier actifs pour la société sélectionnée" } }
                for item in profile_catalog.read().as_deref().unwrap_or(&[]).iter().filter(|p| p.legal_form_code == "SARL").cloned() {
                    SarlBusinessProfileRow {
                        item: item.clone(),
                        profile: profiles.read().as_deref().unwrap_or(&[]).iter().find(|p| p.profile_code == item.code).cloned(),
                        entity_id: Uuid::parse_str(&selected()).unwrap_or(Uuid::nil()),
                        bump,
                        msg,
                    }
                }
            }
            section {
                class: "panel",
                div { class: "panel-head", h3 { "Catalogue d'activités" }, span { class: "small", "Plusieurs choix possibles" } }
                for item in catalog.read().as_deref().unwrap_or(&[]).iter().filter(|item| item.active).cloned() {
                    SarlActivityRow {
                        item: item.clone(),
                        current: activities.read().as_deref().unwrap_or(&[]).iter().find(|a| a.activity_code == item.code).cloned(),
                        entity_id: Uuid::parse_str(&selected()).unwrap_or(Uuid::nil()),
                        bump,
                        msg,
                    }
                }
                div { class: "action-row", span { class: "save-ok", "{msg()}" } }
            }
        }
    }
}

#[component]
fn SarlBusinessProfileRow(
    item: crate::domain::BusinessProfileCatalogItem,
    profile: Option<crate::domain::LegalEntityBusinessProfileItem>,
    entity_id: Uuid,
    mut bump: Signal<u64>,
    mut msg: Signal<String>,
) -> Element {
    let active = profile.as_ref().map(|p| p.active).unwrap_or(false);
    let primary = profile.as_ref().map(|p| p.is_primary).unwrap_or(false);
    let code = item.code.clone();
    let toggle_active = !active;
    rsx! {
        div {
            class: "list-row",
            div {
                class: "row-main",
                strong { "{item.label}" }
                div { class: "small", "{item.description}" }
                div { class: "small", if active { if primary { "ACTIF • PRINCIPAL" } else { "ACTIF" } } else { "NON ACTIVÉ" } }
            }
            div {
                class: "row-actions",
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let draft = LegalEntityBusinessProfileDraft {
                            legal_entity_id: entity_id,
                            profile_code: code.clone(),
                            is_primary: toggle_active,
                            active: toggle_active,
                            configuration: if toggle_active { serde_json::json!({"configuration_status":"A_CONFIGURER"}) } else { serde_json::json!({}) },
                            notes: String::new(),
                        };
                        async move {
                            match set_entity_business_profile(draft).await {
                                Ok(_) => { msg.set(if toggle_active { "Profil d'exploitation activé" } else { "Profil d'exploitation désactivé" }.into()); bump += 1; }
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    },
                    if active { "Désactiver" } else { "Activer" }
                }
            }
        }
    }
}

#[component]
fn SarlActivityRow(
    item: crate::domain::LegalActivityCatalogItem,
    current: Option<LegalEntityActivityItem>,
    entity_id: Uuid,
    mut bump: Signal<u64>,
    mut msg: Signal<String>,
) -> Element {
    let active = current.as_ref().map(|a| a.active).unwrap_or(false);
    let primary = current.as_ref().map(|a| a.is_primary).unwrap_or(false);
    let code = item.code.clone();
    let code_for_toggle = code.clone();
    let code_for_primary = code.clone();
    rsx! {
        div {
            class: "list-row",
            div {
                class: "row-main",
                strong { "{item.label}" }
                div { class: "small", "{item.description}" }
                if active { div { class: "small", if primary { "ACTIVE • PRINCIPALE" } else { "ACTIVE" } } }
            }
            div {
                class: "row-actions",
                button {
                    class: "secondary",
                    onclick: move |_| {
                        let draft = LegalEntityActivityDraft {
                            legal_entity_id: entity_id,
                            activity_code: code_for_toggle.clone(),
                            is_primary: if active { primary } else { false },
                            active: !active,
                            notes: String::new(),
                        };
                        async move {
                            match set_legal_entity_activity(draft).await {
                                Ok(_) => { msg.set("Activité mise à jour".into()); bump += 1; }
                                Err(e) => msg.set(e.to_string()),
                            }
                        }
                    },
                    if active { "Désactiver" } else { "Activer" }
                }
                if active && !primary {
                    button {
                        class: "secondary",
                        onclick: move |_| {
                            let code = code_for_primary.clone();
                            async move {
                                match set_primary_legal_entity_activity(entity_id, code).await {
                                    Ok(_) => { msg.set("Activité principale définie".into()); bump += 1; }
                                    Err(e) => msg.set(e.to_string()),
                                }
                            }
                        },
                        "Principale"
                    }
                }
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_code_must_be_canonical() {
        let draft = LegalEntityActivityDraft {
            legal_entity_id: Uuid::new_v4(),
            activity_code: "MARCHAND_DE_BIENS".into(),
            is_primary: false,
            active: true,
            notes: String::new(),
        };
        assert!(validate_activity_draft(&draft).is_ok());
    }

    #[test]
    fn activity_code_rejects_spaces() {
        let mut draft = LegalEntityActivityDraft {
            legal_entity_id: Uuid::new_v4(),
            activity_code: "GARAGE AUTO".into(),
            is_primary: false,
            active: true,
            notes: String::new(),
        };
        assert!(validate_activity_draft(&draft).is_err());
        draft.activity_code = "GARAGE_AUTO".into();
        assert!(validate_activity_draft(&draft).is_ok());
    }
}
